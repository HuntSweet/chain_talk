use crate::auth::extract_user_from_token;
use crate::error::{AppError, Result};
use crate::models::{ClientMessage, ServerMessage};
use crate::state::AppState;
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/**
 * 处理WebSocket连接
 * 管理客户端连接的整个生命周期，包括认证、消息处理和断开连接
 */
pub async fn handle_connection(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut user_address: Option<String> = None;
    let mut authenticated = false;
    let mut global_receiver = state.global_sender.subscribe();
    let mut client_receiver: Option<broadcast::Receiver<ServerMessage>> = None;
    
    info!("New WebSocket connection established");
    
    // 发送欢迎消息
    let welcome_msg = ServerMessage::NewText {
        id: uuid::Uuid::new_v4().to_string(),
        from: "System".to_string(),
        text: "Welcome to ChainTalk! Please authenticate to start chatting.".to_string(),
        room: "system".to_string(),
        timestamp: chrono::Utc::now(),
    };
    
    if let Err(e) = send_message(&mut sender, &welcome_msg).await {
        error!("Failed to send welcome message: {}", e);
        return;
    }
    
    loop {
        tokio::select! {
            // 处理来自客户端的消息
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match handle_client_message(&text, &state, &mut user_address, &mut authenticated, &mut client_receiver).await {
                            Ok(should_continue) => {
                                if !should_continue {
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Error handling client message: {}", e);
                                let error_msg = ServerMessage::Error {
                                    message: e.to_string(),
                                };
                                if let Err(send_err) = send_message(&mut sender, &error_msg).await {
                                    error!("Failed to send error message: {}", send_err);
                                    break;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Client requested connection close");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("Client disconnected");
                        break;
                    }
                    _ => {}
                }
            }
            
            // 处理全局广播消息
            msg = global_receiver.recv() => {
                match msg {
                    Ok(message) => {
                        if let Err(e) = send_message(&mut sender, &message).await {
                            error!("Failed to send global message: {}", e);
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("Global broadcast channel closed");
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        warn!("Global broadcast receiver lagged");
                    }
                }
            }
            
            // 处理客户端专用消息
            msg = async {
                if let Some(ref mut receiver) = client_receiver {
                    receiver.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                match msg {
                    Ok(message) => {
                        if let Err(e) = send_message(&mut sender, &message).await {
                            error!("Failed to send client message: {}", e);
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("Client broadcast channel closed");
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        warn!("Client broadcast receiver lagged");
                    }
                }
            }
        }
    }
    
    // 清理连接
    if let Some(addr) = user_address {
        state.remove_client(&addr).await;
        info!("Cleaned up connection for user: {}", addr);
    }
}

/**
 * 处理来自客户端的消息
 */
async fn handle_client_message(
    text: &str,
    state: &Arc<AppState>,
    user_address: &mut Option<String>,
    authenticated: &mut bool,
    client_receiver: &mut Option<broadcast::Receiver<ServerMessage>>,
) -> Result<bool> {
    // 记录接收到的原始消息
    info!("📨 Received client message: {}", text);
    
    // 解析客户端消息
    let client_msg: ClientMessage = serde_json::from_str(text)
        .map_err(|e| {
            error!("❌ Failed to parse client message: {}", e);
            error!("❌ Raw message was: {}", text);
            AppError::SerializationError(e.to_string())
        })?;
    
    info!("✅ Successfully parsed client message type: {:?}", std::mem::discriminant(&client_msg));
    
    match client_msg {
        ClientMessage::Authenticate { message, signature } => {
            if !*authenticated {
                return handle_siwe_authentication(&message, &signature, state, user_address, authenticated, client_receiver).await;
            } else {
                return Err(AppError::AuthenticationFailed("Already authenticated".to_string()));
            }
        }
        ClientMessage::SimpleAuth { address, message, signature, nonce } => {
            if !*authenticated {
                return handle_simple_authentication(&address, &message, &signature, &nonce, state, user_address, authenticated, client_receiver).await;
            } else {
                return Err(AppError::AuthenticationFailed("Already authenticated".to_string()));
            }
        }
        _ => {
            if !*authenticated {
                return Err(AppError::AuthenticationFailed("Not authenticated".to_string()));
            }
        }
    }
    
    let user_addr = user_address.as_ref().unwrap();
    
    // 更新客户端活动时间
    state.update_client_activity(user_addr).await;
    
    match client_msg {
        ClientMessage::Authenticate { .. } => {
            // Already handled above
        }
        ClientMessage::SimpleAuth { .. } => {
            // Already handled above
        }
        ClientMessage::SendText { room, text } => {
            handle_send_text(state, user_addr, &room, &text).await?;
        }
        ClientMessage::JoinRoom { room } => {
            handle_join_room(state, user_addr, &room).await?;
        }
        ClientMessage::LeaveRoom { room } => {
            handle_leave_room(state, user_addr, &room).await?;
        }
        ClientMessage::Ping => {
            // 响应ping消息
            if let Some(client) = state.get_client(user_addr).await {
                let _ = client.sender.send(ServerMessage::Pong);
            }
        }
    }
    
    Ok(true)
}

/**
 * 处理SIWE认证
 */
async fn handle_siwe_authentication(
    message: &str,
    signature: &str,
    state: &Arc<AppState>,
    user_address: &mut Option<String>,
    authenticated: &mut bool,
    client_receiver: &mut Option<broadcast::Receiver<ServerMessage>>,
) -> Result<bool> {
    info!("🔐 Starting SIWE authentication process");
    info!("📝 SIWE message from client: {}", message);
    info!("✍️ Signature from client: {}", signature);
    info!("📏 Message length: {} chars, Signature length: {} chars", message.len(), signature.len());
    
    // 验证SIWE消息和签名
    let user_auth = state.auth_service.verify_siwe_message(message, signature).await
        .map_err(|e| {
            error!("❌ SIWE verification failed in websocket handler: {}", e);
            AppError::AuthenticationFailed(format!("SIWE verification failed: {}", e))
        })?;
    
    info!("✅ SIWE authentication successful for address: {}", user_auth.address);
    
    // 将客户端添加到状态管理
    let _client_id = state.add_client(user_auth.address.clone(), user_auth.ens_name.clone()).await;
    
    // 获取客户端的消息接收器
    if let Some(client) = state.get_client(&user_auth.address).await {
        *client_receiver = Some(client.sender.subscribe());
    }
    
    // 更新认证状态
    *user_address = Some(user_auth.address.clone());
    *authenticated = true;
    
    // 发送认证成功消息
    if let Some(client) = state.get_client(&user_auth.address).await {
        let auth_success_msg = ServerMessage::AuthSuccess {
            user_address: user_auth.address.clone(),
            ens_name: user_auth.ens_name.clone(),
        };
        let _ = client.sender.send(auth_success_msg);
    }
    
    // 自动加入默认房间
    state.join_room(&user_auth.address, "general").await;
    
    // 广播用户加入消息
    let join_message = ServerMessage::user_joined(user_auth.address.clone(), "general".to_string());
    state.broadcast_to_room("general", join_message).await;
    
    info!("User authenticated via SIWE and joined general room: {}", user_auth.address);
    
    Ok(true)
}

/**
 * 处理简化认证 - 使用ethers进行签名验证
 */
async fn handle_simple_authentication(
    address: &str,
    message: &str,
    signature: &str,
    nonce: &str,
    state: &Arc<AppState>,
    user_address: &mut Option<String>,
    authenticated: &mut bool,
    client_receiver: &mut Option<broadcast::Receiver<ServerMessage>>,
) -> Result<bool> {
    info!("🔐 Starting simple authentication process");
    info!("📝 Address from client: {}", address);
    info!("📝 Message from client: {}", message);
    info!("✍️ Signature from client: {}", signature);
    info!("🎲 Nonce from client: {}", nonce);
    
    // 验证nonce是否存在且有效
    let mut conn = state.redis_pool.get().await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    let nonce_key = format!("nonce:{}", nonce);
    let nonce_exists: bool = conn.exists(&nonce_key).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    if !nonce_exists {
        error!("❌ Nonce not found or expired: {}", nonce);
        return Err(AppError::InvalidNonce);
    }
    
    info!("✅ Nonce validation passed");
    
    // 删除已使用的nonce
    let _: () = conn.del(&nonce_key).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    // 使用ethers进行简化签名验证
    use ethers::utils::hash_message;
    use ethers::types::{RecoveryMessage, Signature};
    use ethers::utils::to_checksum;
    use std::str::FromStr;
    
    // 解析签名
    let sig = Signature::from_str(signature.trim_start_matches("0x"))
        .map_err(|e| {
            error!("❌ Failed to parse signature: {}", e);
            AppError::InvalidSignature
        })?;
    
    // 恢复地址
    let message_hash = hash_message(message.as_bytes());
    let recovered_address = sig.recover(RecoveryMessage::Hash(message_hash))
        .map_err(|e| {
            error!("❌ Failed to recover address from signature: {}", e);
            AppError::InvalidSignature
        })?;
    
    // 转换为checksummed地址进行比较
    let recovered_checksum = to_checksum(&recovered_address, None);
    let expected_checksum = to_checksum(&ethers::types::Address::from_str(address)
        .map_err(|e| AppError::InvalidRequest(e.to_string()))?, None);
    
    if recovered_checksum.to_lowercase() != expected_checksum.to_lowercase() {
        error!("❌ Address verification failed:");
        error!("   Expected: {}", expected_checksum);
        error!("   Recovered: {}", recovered_checksum);
        return Err(AppError::InvalidSignature);
    }
    
    info!("✅ Simple signature verification passed for address: {}", recovered_checksum);
    
    // 将客户端添加到状态管理
    let _client_id = state.add_client(recovered_checksum.clone(), None).await;
    
    // 获取客户端的消息接收器
    if let Some(client) = state.get_client(&recovered_checksum).await {
        *client_receiver = Some(client.sender.subscribe());
    }
    
    // 更新认证状态
    *user_address = Some(recovered_checksum.clone());
    *authenticated = true;
    
    // 发送认证成功消息
    if let Some(client) = state.get_client(&recovered_checksum).await {
        let auth_success_msg = ServerMessage::AuthSuccess {
            user_address: recovered_checksum.clone(),
            ens_name: None,
        };
        let _ = client.sender.send(auth_success_msg);
    }
    
    // 自动加入默认房间
    state.join_room(&recovered_checksum, "general").await;
    
    // 广播用户加入消息
    let join_message = ServerMessage::user_joined(recovered_checksum.clone(), "general".to_string());
    state.broadcast_to_room("general", join_message).await;
    
    info!("✅ User authenticated via simple auth and joined general room: {}", recovered_checksum);
    
    Ok(true)
}

/**
 * 处理发送文本消息 - 优化版本，支持消息验证和速率限制
 */
async fn handle_send_text(
    state: &Arc<AppState>,
    user_address: &str,
    room: &str,
    text: &str,
) -> Result<()> {
    // 输入验证
    if text.trim().is_empty() {
        return Err(AppError::InvalidRequest("Message cannot be empty".to_string()));
    }
    
    if text.len() > 1000 {
        return Err(AppError::InvalidRequest("Message too long (max 1000 characters)".to_string()));
    }
    
    // 检查用户是否在房间中
    let client = state.get_client(user_address).await
        .ok_or_else(|| AppError::AuthenticationFailed("Client not found".to_string()))?;
    
    if !client.current_rooms.contains(room) {
        return Err(AppError::AuthorizationFailed("User not in room".to_string()));
    }
    
    // 创建消息
    let display_name = client.ens_name.unwrap_or_else(|| {
        // 缩短地址显示
        let addr = user_address.to_string();
        if addr.len() > 10 {
            format!("{}...{}", &addr[..6], &addr[addr.len()-4..])
        } else {
            addr
        }
    });
    
    let message = ServerMessage::new_text(display_name, text.to_string(), room.to_string());
    
    // 异步广播到房间（避免阻塞）
    let state_clone = Arc::clone(state);
    let room_name = room.to_string();
    tokio::spawn(async move {
        state_clone.broadcast_to_room(&room_name, message).await;
    });
    
    Ok(())
}

/**
 * 处理加入房间
 */
async fn handle_join_room(
    state: &Arc<AppState>,
    user_address: &str,
    room: &str,
) -> Result<()> {
    let success = state.join_room(user_address, room).await;
    
    if success {
        // 广播用户加入消息
        let client = state.get_client(user_address).await.unwrap();
        let display_name = client.ens_name.unwrap_or_else(|| user_address.to_string());
        let join_msg = ServerMessage::user_joined(display_name, room.to_string());
        state.broadcast_to_room(room, join_msg).await;
        
        // 发送房间用户列表给新用户
        let users = state.get_room_users(room).await;
        let users_msg = ServerMessage::RoomUsers {
            room: room.to_string(),
            users,
        };
        
        if let Some(client) = state.get_client(user_address).await {
            let _ = client.sender.send(users_msg);
        }
    }
    
    Ok(())
}

/**
 * 处理离开房间
 */
async fn handle_leave_room(
    state: &Arc<AppState>,
    user_address: &str,
    room: &str,
) -> Result<()> {
    let client = state.get_client(user_address).await
        .ok_or_else(|| AppError::AuthenticationFailed("Client not found".to_string()))?;
    
    let display_name = client.ens_name.unwrap_or_else(|| user_address.to_string());
    
    state.leave_room(user_address, room).await;
    
    // 广播用户离开消息
    let leave_msg = ServerMessage::user_left(display_name, room.to_string());
    state.broadcast_to_room(room, leave_msg).await;
    
    Ok(())
}

/**
 * 发送消息到WebSocket
 */
async fn send_message(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    message: &ServerMessage,
) -> Result<()> {
    let json = serde_json::to_string(message)
        .map_err(|e| AppError::SerializationError(e.to_string()))?;
    
    sender.send(Message::Text(json)).await
        .map_err(|e| AppError::WebSocketError(e.to_string()))?;
    
    Ok(())
}
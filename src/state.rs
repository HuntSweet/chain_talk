use crate::auth::AuthService;
use crate::models::{ServerMessage, UserAuth};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/**
 * 客户端连接信息
 */
#[derive(Debug, Clone)]
pub struct Client {
    pub id: String,
    pub user_address: String,
    pub ens_name: Option<String>,
    pub current_rooms: HashSet<String>,
    pub sender: broadcast::Sender<ServerMessage>,
}

/**
 * 房间信息
 */
#[derive(Debug, Clone)]
pub struct Room {
    pub name: String,
    pub users: HashSet<String>, // 用户地址集合
    pub message_history: Vec<ServerMessage>, // 最近的消息历史
    pub max_history: usize,
}

/**
 * 应用全局状态
 */
pub struct AppState {
    /// Redis连接池，用于存储会话和nonce
    pub redis_pool: Pool<RedisConnectionManager>,
    
    /// 认证服务
    pub auth_service: AuthService,
    
    /// 已连接的客户端 (user_address -> Client)
    pub clients: RwLock<HashMap<String, Client>>,
    
    /// 聊天室信息 (room_name -> Room)
    pub rooms: RwLock<HashMap<String, Room>>,
    
    /// 用户认证信息缓存 (user_address -> UserAuth)
    pub user_auth_cache: RwLock<HashMap<String, UserAuth>>,
    
    /// 全局消息广播通道
    pub global_sender: broadcast::Sender<ServerMessage>,
}

impl AppState {
    /**
     * 创建新的应用状态实例
     */
    pub fn new(redis_pool: Pool<RedisConnectionManager>, auth_service: AuthService) -> Self {
        let (global_sender, _) = broadcast::channel(1000);
        
        let mut rooms = HashMap::new();
        // 创建默认房间
        rooms.insert(
            "general".to_string(),
            Room {
                name: "general".to_string(),
                users: HashSet::new(),
                message_history: Vec::new(),
                max_history: 100,
            },
        );
        
        Self {
            redis_pool,
            auth_service,
            clients: RwLock::new(HashMap::new()),
            rooms: RwLock::new(rooms),
            user_auth_cache: RwLock::new(HashMap::new()),
            global_sender,
        }
    }
    
    /**
     * 添加客户端连接 - 优化版本
     */
    pub async fn add_client(&self, user_address: String, ens_name: Option<String>) -> String {
        let client_id = Uuid::new_v4().to_string();
        let (sender, _) = broadcast::channel(128); // 增加缓冲区大小
        
        let client = Client {
            id: client_id.clone(),
            user_address: user_address.clone(),
            ens_name,
            current_rooms: HashSet::new(),
            sender,
        };
        
        let mut clients = self.clients.write().await;
        clients.insert(user_address, client);
        
        tracing::info!("Client added: {} (total: {})", client_id, clients.len());
        client_id
    }

    /**
     * 更新客户端活动时间
     */
    pub async fn update_client_activity(&self, user_address: &str) {
        // 这个函数现在是空的，因为Client结构没有活动时间字段
        // 但我们保留接口以备将来扩展
        tracing::trace!("Activity updated for client: {}", user_address);
    }
    
    /**
     * 移除客户端连接
     */
    pub async fn remove_client(&self, user_address: &str) {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.remove(user_address) {
            // 从所有房间中移除用户
            let rooms_to_leave: Vec<String> = client.current_rooms.into_iter().collect();
            drop(clients); // 释放锁
            
            for room_name in rooms_to_leave {
                self.leave_room(user_address, &room_name).await;
            }
        }
    }
    
    /**
     * 用户加入房间
     */
    pub async fn join_room(&self, user_address: &str, room_name: &str) -> bool {
        let mut rooms = self.rooms.write().await;
        let mut clients = self.clients.write().await;
        
        // 确保房间存在
        if !rooms.contains_key(room_name) {
            rooms.insert(
                room_name.to_string(),
                Room {
                    name: room_name.to_string(),
                    users: HashSet::new(),
                    message_history: Vec::new(),
                    max_history: 100,
                },
            );
        }
        
        // 添加用户到房间
        if let Some(room) = rooms.get_mut(room_name) {
            room.users.insert(user_address.to_string());
        }
        
        // 更新客户端状态
        if let Some(client) = clients.get_mut(user_address) {
            client.current_rooms.insert(room_name.to_string());
            return true;
        }
        
        false
    }
    
    /**
     * 用户离开房间
     */
    pub async fn leave_room(&self, user_address: &str, room_name: &str) {
        let mut rooms = self.rooms.write().await;
        let mut clients = self.clients.write().await;
        
        // 从房间中移除用户
        if let Some(room) = rooms.get_mut(room_name) {
            room.users.remove(user_address);
        }
        
        // 更新客户端状态
        if let Some(client) = clients.get_mut(user_address) {
            client.current_rooms.remove(room_name);
        }
    }
    
    /**
     * 获取房间用户列表
     */
    pub async fn get_room_users(&self, room_name: &str) -> Vec<String> {
        let rooms = self.rooms.read().await;
        if let Some(room) = rooms.get(room_name) {
            room.users.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }
    
    /**
     * 向房间广播消息
     */
    pub async fn broadcast_to_room(&self, room_name: &str, message: ServerMessage) {
        let clients = self.clients.read().await;
        let mut rooms = self.rooms.write().await;
        
        // 添加消息到房间历史
        if let Some(room) = rooms.get_mut(room_name) {
            room.message_history.push(message.clone());
            if room.message_history.len() > room.max_history {
                room.message_history.remove(0);
            }
            
            // 向房间内所有用户发送消息
            for user_address in &room.users {
                if let Some(client) = clients.get(user_address) {
                    let _ = client.sender.send(message.clone());
                }
            }
        }
    }
    
    /**
     * 向所有客户端广播消息
     */
    pub async fn broadcast_global(&self, message: ServerMessage) {
        let _ = self.global_sender.send(message);
    }
    
    /**
     * 获取客户端信息
     */
    pub async fn get_client(&self, user_address: &str) -> Option<Client> {
        let clients = self.clients.read().await;
        clients.get(user_address).cloned()
    }
    
    /**
     * 缓存用户认证信息
     */
    pub async fn cache_user_auth(&self, user_address: String, auth: UserAuth) {
        let mut cache = self.user_auth_cache.write().await;
        cache.insert(user_address, auth);
    }
    
    /**
     * 获取缓存的用户认证信息
     */
    pub async fn get_cached_user_auth(&self, user_address: &str) -> Option<UserAuth> {
        let cache = self.user_auth_cache.read().await;
        cache.get(user_address).cloned()
    }
}

impl Room {
    /**
     * 获取房间最近的消息历史
     */
    pub fn get_recent_messages(&self, limit: usize) -> Vec<ServerMessage> {
        let start = if self.message_history.len() > limit {
            self.message_history.len() - limit
        } else {
            0
        };
        self.message_history[start..].to_vec()
    }
}

impl AppState {
    /**
     * 获取房间在线用户详细信息
     */
    pub async fn get_online_users(&self, room_name: &str) -> Vec<crate::models::OnlineUser> {
        let rooms = self.rooms.read().await;
        let clients = self.clients.read().await;
        
        if let Some(room) = rooms.get(room_name) {
            room.users.iter()
                .filter_map(|addr| {
                    clients.get(addr).map(|client| crate::models::OnlineUser {
                        address: addr.clone(),
                        ens_name: client.ens_name.clone(),
                    })
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /**
     * 广播用户加入房间
     */
    pub async fn broadcast_user_joined(&self, room_name: &str, user_address: &str) {
        let client_ens = {
            let clients = self.clients.read().await;
            clients.get(user_address).and_then(|c| c.ens_name.clone())
        };

        let message = ServerMessage::UserJoined {
            user: user_address.to_string(),
            room: room_name.to_string(),
            timestamp: chrono::Utc::now(),
            ens_name: client_ens,
        };

        self.broadcast_to_room(room_name, message).await;

        // 同时广播在线用户列表
        self.broadcast_online_users(room_name).await;
    }

    /**
     * 广播用户离开房间
     */
    pub async fn broadcast_user_left(&self, room_name: &str, user_address: &str) {
        let client_ens = {
            let clients = self.clients.read().await;
            clients.get(user_address).and_then(|c| c.ens_name.clone())
        };

        let message = ServerMessage::UserLeft {
            user: user_address.to_string(),
            room: room_name.to_string(),
            timestamp: chrono::Utc::now(),
            ens_name: client_ens,
        };

        self.broadcast_to_room(room_name, message).await;

        // 同时广播更新后的在线用户列表
        self.broadcast_online_users(room_name).await;
    }

    /**
     * 广播在线用户列表
     */
    pub async fn broadcast_online_users(&self, room_name: &str) {
        let online_users = self.get_online_users(room_name).await;
        
        let message = ServerMessage::OnlineUsers {
            users: online_users,
            room: room_name.to_string(),
        };

        self.broadcast_to_room(room_name, message).await;
    }
}
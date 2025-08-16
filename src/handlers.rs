use crate::auth::AuthService;
use crate::error::{AppError, Result};
use crate::models::{LoginRequest, LoginResponse, NonceResponse, UserInfo};
use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{error, info};

/**
 * 获取认证nonce
 * POST /api/auth/nonce
 */
pub async fn get_nonce(
    State(state): State<Arc<AppState>>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<NonceResponse>> {
    let address = request.get("address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing address field".to_string()))?;
    
    info!("Generating new nonce for address: {}", address);
    
    // 创建认证服务实例
    let jwt_secret = std::env::var("JWT_SECRET")
        .map_err(|_| AppError::InternalError("JWT secret not configured".to_string()))?;
    
    let eth_rpc_url = std::env::var("ETHEREUM_HTTP_URL")
        .unwrap_or_else(|_| "https://mainnet.infura.io/v3/YOUR_PROJECT_ID".to_string());
    
    let auth_service = AuthService::new(
        jwt_secret,
        state.redis_pool.clone(),
        &eth_rpc_url,
    )?;
    
    let nonce = auth_service.generate_nonce().await?;
    
    Ok(Json(NonceResponse { nonce }))
}

/**
 * 用户登录认证
 * POST /api/login
 */
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    info!("Processing login request");
    
    // 创建认证服务实例
    let jwt_secret = std::env::var("JWT_SECRET")
        .map_err(|_| AppError::InternalError("JWT secret not configured".to_string()))?;
    
    let eth_rpc_url = std::env::var("ETHEREUM_HTTP_URL")
        .unwrap_or_else(|_| "https://mainnet.infura.io/v3/YOUR_PROJECT_ID".to_string());
    
    let auth_service = AuthService::new(
        jwt_secret,
        state.redis_pool.clone(),
        &eth_rpc_url,
    )?;
    
    // 验证SIWE消息和签名
    let user_auth = auth_service
        .verify_siwe_message(&request.message, &request.signature)
        .await?;
    
    // 生成JWT token
    let token = auth_service.generate_jwt(&user_auth)?;
    
    // 缓存用户认证信息
    state.cache_user_auth(user_auth.address.clone(), user_auth.clone()).await;
    
    // 创建用户信息
    let user_info = UserInfo {
        address: user_auth.address.clone(),
        ens_name: user_auth.ens_name.clone(),
        avatar: None, // 可以从ENS或其他来源获取
    };
    
    info!("User {} authenticated successfully", user_auth.address);
    
    Ok(Json(LoginResponse {
        token,
        user: user_info,
    }))
}

/**
 * 健康检查端点
 * GET /health
 */
pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

/**
 * 获取用户信息
 * GET /api/user/:address
 */
pub async fn get_user_info(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(address): axum::extract::Path<String>,
) -> Result<Json<UserInfo>> {
    // 从缓存中获取用户信息
    if let Some(user_auth) = state.get_cached_user_auth(&address).await {
        return Ok(Json(UserInfo {
            address: user_auth.address,
            ens_name: user_auth.ens_name,
            avatar: None,
        }));
    }
    
    Err(AppError::InvalidRequest("User not found".to_string()))
}

/**
 * 获取房间信息
 * GET /api/rooms/:room_name
 */
pub async fn get_room_info(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(room_name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>> {
    let users = state.get_room_users(&room_name).await;
    
    Ok(Json(serde_json::json!({
        "name": room_name,
        "users": users,
        "user_count": users.len()
    })))
}

/**
 * 获取所有房间列表
 * GET /api/rooms
 */
pub async fn get_rooms(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>> {
    let rooms = state.rooms.read().await;
    let room_list: Vec<serde_json::Value> = rooms
        .iter()
        .map(|(name, room)| {
            serde_json::json!({
                "name": name,
                "user_count": room.users.len(),
                "users": room.users.iter().collect::<Vec<_>>()
            })
        })
        .collect();
    
    Ok(Json(room_list))
}

/**
 * 验证token门禁
 * POST /api/verify-token-gate
 */
pub async fn verify_token_gate(
    State(state): State<Arc<AppState>>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let user_address = request["user_address"]
        .as_str()
        .ok_or_else(|| AppError::InvalidRequest("Missing user_address".to_string()))?;
    
    let contract_address = request["contract_address"]
        .as_str()
        .ok_or_else(|| AppError::InvalidRequest("Missing contract_address".to_string()))?;
    
    let minimum_balance = request["minimum_balance"].as_str();
    
    // 创建认证服务实例
    let jwt_secret = std::env::var("JWT_SECRET")
        .map_err(|_| AppError::InternalError("JWT secret not configured".to_string()))?;
    
    let eth_rpc_url = std::env::var("ETHEREUM_HTTP_URL")
        .unwrap_or_else(|_| "https://mainnet.infura.io/v3/YOUR_PROJECT_ID".to_string());
    
    let auth_service = AuthService::new(
        jwt_secret,
        state.redis_pool.clone(),
        &eth_rpc_url,
    )?;
    
    // 解析用户地址
    let address = user_address.parse()
        .map_err(|e| AppError::InvalidRequest(format!("Invalid address: {}", e)))?;
    
    // 检查token门禁
    let has_access = auth_service
        .check_token_gate(&address, contract_address, minimum_balance)
        .await?;
    
    Ok(Json(serde_json::json!({
        "has_access": has_access,
        "user_address": user_address,
        "contract_address": contract_address
    })))
}

/**
 * 错误处理中间件
 */
pub async fn handle_error(err: Box<dyn std::error::Error + Send + Sync>) -> AppError {
    error!("Unhandled error: {}", err);
    AppError::InternalError("Internal server error".to_string())
}
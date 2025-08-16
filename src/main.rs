use anyhow::Result;
use axum::{
    extract::{State, WebSocketUpgrade},
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::{info, warn, error};
use std::env;

mod auth;
mod blockchain;
mod config;
mod error;
mod handlers;
mod models;
mod state;
mod websocket;

use auth::AuthService;
use config::Config;
use state::AppState;

/**
 * ChainTalk 主程序入口
 * 初始化配置、状态管理、区块链监听器和Web服务器
 */
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    // 加载配置
    dotenv::dotenv().ok();
    let config = Config::from_env()?;
    
    info!("Starting ChainTalk server...");
    
    // 初始化Redis连接池
    let redis_pool = create_redis_pool(&config.redis_url).await?;
    
    // 创建认证服务
    let auth_service = AuthService::new(
        config.jwt_secret.clone(),
        redis_pool.clone(),
        &config.ethereum_http_url,
    )?;
    
    // 创建应用状态
    let app_state = Arc::new(AppState::new(redis_pool, auth_service));
    
    // 启动区块链监听器 (暂时禁用以避免API限制)
    let blockchain_listener = blockchain::BlockchainListener::new(
        &config.ethereum_ws_url,
        app_state.clone(),
    ).await?;
    
    tokio::spawn(async move {
        if let Err(e) = blockchain_listener.start().await {
            warn!("Blockchain listener error: {}", e);
        }
    });
    
    if let (Ok(ws_url), Ok(_http_url)) = (
        env::var("ETHEREUM_WS_URL"),
        env::var("ETHEREUM_HTTP_URL")
    ) {
        info!("🔗 Blockchain listener configured but disabled for demo");
        info!("   To enable: Set valid ETHEREUM_WS_URL and ETHEREUM_HTTP_URL in .env");
        info!("   Example: ETHEREUM_WS_URL=wss://mainnet.infura.io/ws/v3/YOUR_PROJECT_ID");
        let blockchain_listener = blockchain::BlockchainListener::new(&ws_url, app_state.clone()).await?;
        let _listener_handle = tokio::spawn(async move {
            if let Err(e) = blockchain_listener.start().await {
                error!("Blockchain listener error: {}", e);
            }
        });
    } else {
        info!("⚠️ Blockchain listener disabled - missing ETHEREUM_WS_URL or ETHEREUM_HTTP_URL");
        info!("   See .env.example for configuration details");
    }
    
    // 创建路由
    let app = create_router(app_state);
    
    // 启动服务器
    let listener = TcpListener::bind(&config.server_address).await?;
    info!("Server listening on {}", config.server_address);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/**
 * 创建Redis连接池
 */
async fn create_redis_pool(redis_url: &str) -> Result<bb8::Pool<bb8_redis::RedisConnectionManager>> {
    let manager = bb8_redis::RedisConnectionManager::new(redis_url)?;
    let pool = bb8::Pool::builder().build(manager).await?;
    Ok(pool)
}

/**
 * 创建应用路由
 */
fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        // WebSocket路由
        .route("/ws", get(websocket_handler))
        // API路由
        .route("/api/auth/nonce", post(handlers::get_nonce))
        .route("/api/auth/login", post(handlers::login))
        .route("/api/user/info", get(handlers::get_user_info))
        .route("/api/rooms", get(handlers::get_rooms))
        .route("/api/rooms/:room_id", get(handlers::get_room_info))
        .route("/api/token-gate/verify", post(handlers::verify_token_gate))
        // 健康检查
        .route("/health", get(health_check))
        // 静态文件服务
        .nest_service("/frontend", ServeDir::new("frontend"))
        .nest_service("/", ServeDir::new("frontend"))
        .layer(CorsLayer::permissive())
        .with_state(app_state)
}

/**
 * WebSocket连接处理器
 */
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| websocket::handle_connection(socket, state))
}

/**
 * 健康检查端点
 */
async fn health_check() -> StatusCode {
    StatusCode::OK
}
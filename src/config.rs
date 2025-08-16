use anyhow::{anyhow, Result};
use std::env;

/**
 * 应用配置结构体
 * 包含服务器地址、数据库连接、区块链节点等配置信息
 */
#[derive(Debug, Clone)]
pub struct Config {
    pub server_address: String,
    pub redis_url: String,
    pub ethereum_ws_url: String,
    pub ethereum_http_url: String,
    pub jwt_secret: String,
    pub cors_origins: Vec<String>,
    pub uniswap_v3_factory: String,
    pub default_room: String,
}

impl Config {
    /**
     * 从环境变量加载配置
     */
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            server_address: env::var("SERVER_ADDRESS")
                .unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            ethereum_ws_url: env::var("ETHEREUM_WS_URL")
                .map_err(|_| anyhow!("ETHEREUM_WS_URL environment variable is required"))?,
            ethereum_http_url: env::var("ETHEREUM_HTTP_URL")
                .map_err(|_| anyhow!("ETHEREUM_HTTP_URL environment variable is required"))?,
            jwt_secret: env::var("JWT_SECRET")
                .map_err(|_| anyhow!("JWT_SECRET environment variable is required"))?,
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            uniswap_v3_factory: env::var("UNISWAP_V3_FACTORY")
                .unwrap_or_else(|_| "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
            default_room: env::var("DEFAULT_ROOM")
                .unwrap_or_else(|_| "general".to_string()),
        })
    }
}
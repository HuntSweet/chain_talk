use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/**
 * 从客户端发往服务端的消息类型
 */
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    Authenticate { message: String, signature: String },
    SimpleAuth { address: String, message: String, signature: String, nonce: String },
    SendText { room: String, text: String },
    JoinRoom { room: String },
    LeaveRoom { room: String },
    Ping,
}

/**
 * 从服务端广播给客户端的消息类型
 */
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    NewText {
        id: String,
        from: String,
        text: String,
        room: String,
        timestamp: DateTime<Utc>,
    },
    UserJoined {
        user: String,
        room: String,
        timestamp: DateTime<Utc>,
        ens_name: Option<String>,
    },
    UserLeft {
        user: String,
        room: String,
        timestamp: DateTime<Utc>,
        ens_name: Option<String>,
    },
    RoomUsers {
        room: String,
        users: Vec<String>,
    },
    ChainEvent(OnChainEvent),
    Error {
        message: String,
    },
    AuthSuccess {
        user_address: String,
        ens_name: Option<String>,
    },
    AuthFailed {
        error: String,
    },
    Pong,
    OnlineUsers {
        users: Vec<OnlineUser>,
        room: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineUser {
    pub address: String,
    pub ens_name: Option<String>,
}

/**
 * 链上事件数据模型
 */
#[derive(Debug, Serialize, Clone)]
pub struct OnChainEvent {
    pub id: String,
    pub event_type: String,
    pub transaction_hash: String,
    pub block_number: u64,
    pub timestamp: DateTime<Utc>,
    pub details: serde_json::Value,
}

/**
 * Uniswap V3 Swap事件详情
 */
#[derive(Debug, Serialize, Clone)]
pub struct UniswapV3SwapDetails {
    pub sender: String,
    pub recipient: String,
    pub amount0: String,
    pub amount1: String,
    pub sqrt_price_x96: String,
    pub liquidity: String,
    pub tick: i32,
    pub pool_address: String,
    pub token0: String,
    pub token1: String,
}

/**
 * 用户认证信息
 */
#[derive(Debug, Clone)]
pub struct UserAuth {
    pub address: String,
    pub ens_name: Option<String>,
    pub token_holdings: HashMap<String, String>, // token_address -> balance
    pub nft_holdings: Vec<String>, // NFT contract addresses
}

/**
 * JWT Claims
 */
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // 用户地址
    pub exp: usize,  // 过期时间
    pub iat: usize,  // 签发时间
    pub ens: Option<String>, // ENS名称
}

/**
 * 认证请求
 */
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub message: String,
    pub signature: String,
}

/**
 * 认证响应
 */
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

/**
 * 用户信息
 */
#[derive(Debug, Serialize, Clone)]
pub struct UserInfo {
    pub address: String,
    pub ens_name: Option<String>,
    pub avatar: Option<String>,
}

/**
 * Nonce响应
 */
#[derive(Debug, Serialize)]
pub struct NonceResponse {
    pub nonce: String,
}

/**
 * 房间配置
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConfig {
    pub name: String,
    pub description: Option<String>,
    pub token_gate: Option<TokenGate>,
    pub max_users: Option<usize>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

/**
 * Token门禁配置
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenGate {
    pub gate_type: TokenGateType,
    pub contract_address: String,
    pub minimum_balance: Option<String>,
    pub token_ids: Option<Vec<String>>,
}

/**
 * Token门禁类型
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenGateType {
    ERC20,
    ERC721,
    ERC1155,
}

impl OnChainEvent {
    /**
     * 创建新的链上事件
     */
    pub fn new(
        event_type: String,
        transaction_hash: String,
        block_number: u64,
        details: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            transaction_hash,
            block_number,
            timestamp: Utc::now(),
            details,
        }
    }
}

impl ServerMessage {
    /**
     * 创建新文本消息
     */
    pub fn new_text(from: String, text: String, room: String) -> Self {
        Self::NewText {
            id: Uuid::new_v4().to_string(),
            from,
            text,
            room,
            timestamp: Utc::now(),
        }
    }

    /**
     * 创建用户加入消息
     */
    pub fn user_joined(user: String, room: String) -> Self {
        Self::UserJoined {
            user,
            room,
            timestamp: Utc::now(),
            ens_name: None,
        }
    }

    /**
     * 创建用户离开消息
     */
    pub fn user_left(user: String, room: String) -> Self {
        Self::UserLeft {
            user,
            room,
            timestamp: Utc::now(),
            ens_name: None,
        }
    }
}
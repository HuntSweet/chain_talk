use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/**
 * 应用错误类型定义
 * 包含认证、区块链交互、数据库操作等各种错误情况
 */
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Invalid nonce")]
    InvalidNonce,
    
    #[error("Token gate check failed: {0}")]
    TokenGateFailed(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Blockchain error: {0}")]
    BlockchainError(String),
    
    #[error("WebSocket error: {0}")]
    WebSocketError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
}

/**
 * 将AppError转换为HTTP响应
 */
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::AuthenticationFailed(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::AuthorizationFailed(_) => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::InvalidSignature => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::InvalidNonce => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::TokenGateFailed(_) => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

/**
 * 从其他错误类型转换为AppError
 */
impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<ethers::providers::ProviderError> for AppError {
    fn from(err: ethers::providers::ProviderError) -> Self {
        AppError::BlockchainError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::SerializationError(err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::AuthenticationFailed(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
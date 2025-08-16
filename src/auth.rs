use crate::error::{AppError, Result};
use crate::models::{Claims, UserAuth, UserInfo};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use chrono::{Duration, Utc};
use ethers::{
    providers::{Http, Provider},
    types::{Address, U256},
    utils::to_checksum,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use redis::AsyncCommands;

use siwe::{Message, VerificationOpts};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

/**
 * 认证服务
 */
pub struct AuthService {
    jwt_secret: String,
    redis_pool: Pool<RedisConnectionManager>,
    eth_provider: Provider<Http>,
}

impl AuthService {
    /**
     * 创建新的认证服务实例
     */
    pub fn new(
        jwt_secret: String,
        redis_pool: Pool<RedisConnectionManager>,
        eth_rpc_url: &str,
    ) -> Result<Self> {
        let eth_provider = Provider::<Http>::try_from(eth_rpc_url)
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;
        
        Ok(Self {
            jwt_secret,
            redis_pool,
            eth_provider,
        })
    }
    
    /**
     * 生成认证nonce
     */
    pub async fn generate_nonce(&self) -> Result<String> {
        let nonce = Uuid::new_v4().to_string();
        let mut conn = self.redis_pool.get().await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        // 存储nonce，5分钟过期
        let _: () = conn.set_ex(&format!("nonce:{}", nonce), "1", 300).await?;
        
        Ok(nonce)
    }
    
    /**
     * 验证SIWE消息和签名
     */
    pub async fn verify_siwe_message(
        &self,
        message_str: &str,
        signature: &str,
    ) -> Result<UserAuth> {
        tracing::info!("Starting SIWE verification");
        tracing::info!("Message: {}", message_str);
        tracing::info!("Signature: {}", signature);
        
        // 首先尝试解析原始消息
        let message = match message_str.parse::<Message>() {
            Ok(msg) => {
                tracing::info!("✅ Parsed SIWE message successfully with original format");
                msg
            }
            Err(original_error) => {
                tracing::warn!("⚠️ Failed to parse original SIWE message: {}, trying with normalized addresses", original_error);
                
                // 如果原始消息解析失败，尝试使用normalize后的消息
                let processed_message = self.normalize_siwe_message(message_str)?;
                tracing::info!("📝 Message normalized (addresses converted to checksum format)");
                tracing::info!("Processed message: {}", processed_message);
                
                processed_message.parse::<Message>()
                    .map_err(|e| {
                        tracing::error!("❌ Failed to parse SIWE message even after normalization: {}", e);
                        tracing::error!("❌ Original error: {}", original_error);
                        AppError::InvalidSignature
                    })?
            }
        };
        
        tracing::info!("Parsed message successfully. Nonce: {}, Address: {:?}", message.nonce, message.address);
        
        // 验证nonce是否存在且有效
        let mut conn = self.redis_pool.get().await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        let nonce_key = format!("nonce:{}", message.nonce);
        let nonce_exists: bool = conn.exists(&nonce_key).await?;
        
        if !nonce_exists {
            tracing::error!("Nonce not found or expired: {}", message.nonce);
            return Err(AppError::InvalidNonce);
        }
        
        tracing::info!("Nonce validation passed");
        
        // 删除已使用的nonce
        let _: () = conn.del(&nonce_key).await?;
        
        // 验证签名 - 使用默认选项让SIWE自动处理
        let verification_opts = VerificationOpts {
            domain: None,
            nonce: None,
            timestamp: None,
        };
        
        // 将十六进制签名字符串转换为字节数组
        let signature_bytes = hex::decode(signature.trim_start_matches("0x"))
            .map_err(|e| {
                tracing::error!("❌ Failed to decode hex signature: {}", e);
                AppError::InvalidSignature
            })?;
        
        tracing::info!("🔢 Converting signature hex: {} (length: {})", signature.trim_start_matches("0x"), signature.len());
        tracing::info!("✅ Signature decoded successfully, length: {} bytes", signature_bytes.len());
        
        // 打印签名的前几个字节用于调试
        if signature_bytes.len() >= 8 {
            let hex_bytes: Vec<String> = signature_bytes.iter().take(8).map(|b| format!("{:02x}", b)).collect();
            tracing::info!("🔍 First 8 bytes of signature: [{}]", hex_bytes.join(", "));
        }
        
        // 打印完整的消息用于调试
        tracing::info!("🔍 Complete message for verification:");
        for (i, line) in message_str.lines().enumerate() {
            tracing::info!("   Line {}: '{}'", i + 1, line);
        }
        
        tracing::info!("🔐 Starting SIWE signature verification...");
        
        message.verify(&signature_bytes, &verification_opts)
            .await
            .map_err(|e| {
                tracing::error!("❌ SIWE signature verification failed: {}", e);
                tracing::error!("❌ Verification details:");
                tracing::error!("   - Message address: {:?}", message.address);
                tracing::error!("   - Message domain: {}", message.domain);
                tracing::error!("   - Message nonce: {}", message.nonce);
                tracing::error!("   - Signature bytes length: {}", signature_bytes.len());
                tracing::error!("🔍 Additional debugging information:");
                tracing::error!("   - Raw message length: {} chars", message_str.len());
                tracing::error!("   - Message starts with: '{}'", &message_str[..std::cmp::min(50, message_str.len())]);
                tracing::error!("   - Message ends with: '{}'", &message_str[std::cmp::max(0, message_str.len().saturating_sub(50))..]);
                tracing::error!("   - Signature format: 0x{}", signature.trim_start_matches("0x"));
                tracing::error!("   - This suggests the SIWE library cannot verify the signature with the given message format");
                AppError::InvalidSignature
            })?;
        
        tracing::info!("SIWE signature verification passed");
       // 转换地址类型
        let address = Address::from(message.address);
        
        // 验证签名
        let user_address = to_checksum(&address, None);
        

        
        // 获取用户的ENS名称
        let ens_name = self.resolve_ens(&address).await.ok();
        
        // 获取用户的token持有情况
        let token_holdings = self.get_token_holdings(&address).await?;
        let nft_holdings = self.get_nft_holdings(&address).await?;
        
        Ok(UserAuth {
            address: user_address,
            ens_name,
            token_holdings,
            nft_holdings,
        })
    }
    
    /**
     * 生成JWT token
     */
    pub fn generate_jwt(&self, user_auth: &UserAuth) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(24); // 24小时过期
        
        let claims = Claims {
            sub: user_auth.address.clone(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            ens: user_auth.ens_name.clone(),
        };
        
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )?;
        
        Ok(token)
    }
    
    /**
     * 验证JWT token
     */
    pub fn verify_jwt(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::default(),
        )?;
        
        Ok(token_data.claims)
    }
    
    /**
     * 检查用户是否满足token门禁要求
     */
    pub async fn check_token_gate(
        &self,
        user_address: &Address,
        contract_address: &str,
        minimum_balance: Option<&str>,
    ) -> Result<bool> {
        let contract_addr = Address::from_str(contract_address)
            .map_err(|e| AppError::InvalidRequest(e.to_string()))?;
        
        // 这里简化实现，实际应该根据合约类型（ERC20/ERC721/ERC1155）调用不同的方法
        let balance = self.get_erc20_balance(user_address, &contract_addr).await?;
        
        if let Some(min_balance) = minimum_balance {
            let min_balance_u256 = U256::from_dec_str(min_balance)
                .map_err(|e| AppError::InvalidRequest(e.to_string()))?;
            Ok(balance >= min_balance_u256)
        } else {
            Ok(balance > U256::zero())
        }
    }
    
    /**
     * 解析ENS名称
     */
    async fn resolve_ens(&self, _address: &Address) -> Result<String> {
        // 这里应该调用ENS合约来解析地址对应的ENS名称
        // 简化实现，返回None
        Err(AppError::BlockchainError("ENS resolution not implemented".to_string()))
    }
    
    /**
     * 获取用户的ERC20 token持有情况
     */
    async fn get_token_holdings(&self, _address: &Address) -> Result<HashMap<String, String>> {
        // 这里应该查询用户持有的各种ERC20 token
        // 简化实现，返回空的HashMap
        Ok(HashMap::new())
    }
    
    /**
     * 获取用户的NFT持有情况
     */
    async fn get_nft_holdings(&self, _address: &Address) -> Result<Vec<String>> {
        // 这里应该查询用户持有的NFT
        // 简化实现，返回空的Vec
        Ok(Vec::new())
    }
    
    /**
     * 获取ERC20 token余额
     */
    async fn get_erc20_balance(&self, _user_address: &Address, _token_address: &Address) -> Result<U256> {
        // 这里应该调用ERC20合约的balanceOf方法
        // 简化实现，返回0
        Ok(U256::zero())
    }

    /**
     * 标准化SIWE消息，将地址转换为EIP-55校验和格式
     * @param message_str SIWE消息字符串
     * @returns 处理后的SIWE消息字符串
     */
    fn normalize_siwe_message(&self, message_str: &str) -> Result<String> {
        use regex::Regex;
        
        // 匹配以太坊地址的正则表达式
        let address_regex = Regex::new(r"0x[a-fA-F0-9]{40}").unwrap();
        
        let normalized = address_regex.replace_all(message_str, |caps: &regex::Captures| {
            let address_str = &caps[0];
            // 尝试解析地址并转换为校验和格式
            match Address::from_str(address_str) {
                Ok(address) => to_checksum(&address, None),
                Err(_) => address_str.to_string(), // 如果解析失败，保持原样
            }
        });
        
        Ok(normalized.to_string())
    }
}

/**
 * 从JWT token中提取用户信息
 */
pub fn extract_user_from_token(token: &str, jwt_secret: &str) -> Result<UserInfo> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    )?;
    
    Ok(UserInfo {
        address: token_data.claims.sub,
        ens_name: token_data.claims.ens,
        avatar: None, // 可以从ENS或其他来源获取头像
    })
}

/**
 * 创建SIWE消息模板
 */
pub fn create_siwe_message(
    address: &str,
    domain: &str,
    nonce: &str,
    chain_id: u64,
) -> String {
    format!(
        "{} wants you to sign in with your Ethereum account:\n{}\n\nChainTalk Authentication\n\nURI: https://{}\nVersion: 1\nChain ID: {}\nNonce: {}\nIssued At: {}",
        domain,
        address,
        domain,
        chain_id,
        nonce,
        Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ")
    )
}
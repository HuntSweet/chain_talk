use crate::error::{AppError, Result};
use crate::models::{OnChainEvent, ServerMessage, UniswapV3SwapDetails};
use crate::state::AppState;
use ethers::{
    contract::{abigen, EthEvent},
    providers::{Provider, Ws, Middleware},
    types::{Address, Filter, Log, U256},
    utils::format_units,
};
use serde_json::json;
use std::collections::HashMap;

use std::str::FromStr;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};

// 生成Uniswap V3 Pool合约的ABI绑定
abigen!(
    UniswapV3Pool,
    r#"[
        {
            "anonymous": false,
            "inputs": [
                {"indexed": true, "internalType": "address", "name": "sender", "type": "address"},
                {"indexed": true, "internalType": "address", "name": "recipient", "type": "address"},
                {"indexed": false, "internalType": "int256", "name": "amount0", "type": "int256"},
                {"indexed": false, "internalType": "int256", "name": "amount1", "type": "int256"},
                {"indexed": false, "internalType": "uint160", "name": "sqrtPriceX96", "type": "uint160"},
                {"indexed": false, "internalType": "uint128", "name": "liquidity", "type": "uint128"},
                {"indexed": false, "internalType": "int24", "name": "tick", "type": "int24"}
            ],
            "name": "Swap",
            "type": "event"
        }
    ]"#
);

/**
 * 区块链事件监听器
 * 监听指定的链上事件并广播到聊天室
 */
pub struct BlockchainListener {
    provider: Provider<Ws>,
    app_state: Arc<AppState>,
    monitored_pools: Vec<Address>,
}

impl BlockchainListener {
    /**
     * 创建新的区块链监听器实例
     */
    pub async fn new(
        ws_url: &str,
        app_state: Arc<AppState>,
    ) -> Result<Self> {
        let provider = Provider::<Ws>::connect(ws_url).await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;
        
        // 预定义一些热门的Uniswap V3池子地址
        let monitored_pools = vec![
            // USDC/WETH 0.05% pool
            Address::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640")
                .map_err(|e| AppError::BlockchainError(e.to_string()))?,
            // USDC/WETH 0.3% pool
            Address::from_str("0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8")
                .map_err(|e| AppError::BlockchainError(e.to_string()))?,
            // WBTC/WETH 0.3% pool
            Address::from_str("0xCBCdF9626bC03E24f779434178A73a0B4bad62eD")
                .map_err(|e| AppError::BlockchainError(e.to_string()))?,
        ];
        
        Ok(Self {
            provider,
            app_state,
            monitored_pools,
        })
    }
    
    /**
     * 开始监听区块链事件
     */
    pub async fn start(self) -> Result<()> {
        info!("Starting blockchain listener...");
        
        // 创建事件过滤器
        let filter = Filter::new()
            .address(self.monitored_pools.clone())
            .event(&SwapFilter::abi_signature());
        
        // 订阅事件流
        let mut stream = self.provider.subscribe_logs(&filter).await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;
        
        info!("Blockchain listener started, monitoring {} pools", self.monitored_pools.len());
        
        // 处理事件流
        while let Some(log) = stream.next().await {
            if let Err(e) = self.handle_log(log).await {
                error!("Error handling blockchain log: {}", e);
            }
        }
        
        warn!("Blockchain event stream ended");
        Ok(())
    }
    
    /**
     * 处理单个区块链日志事件
     */
    async fn handle_log(&self, log: Log) -> Result<()> {
        // 尝试解析为Swap事件
        let raw_log = ethers::abi::RawLog {
            topics: log.topics.clone(),
            data: log.data.to_vec(),
        };
        
        if let Ok(swap_event) = SwapFilter::decode_log(&raw_log) {
            self.handle_swap_event(swap_event, &log).await?;
        }
        
        Ok(())
    }
    
    /**
     * 处理Uniswap V3 Swap事件
     */
    async fn handle_swap_event(&self, event: SwapFilter, log: &Log) -> Result<()> {
        // 获取交易金额的绝对值
        let amount0_abs = if event.amount_0.is_negative() {
            U256::from(event.amount_0.abs().as_u128())
        } else {
            U256::from(event.amount_0.as_u128())
        };
        
        let amount1_abs = if event.amount_1.is_negative() {
            U256::from(event.amount_1.abs().as_u128())
        } else {
            U256::from(event.amount_1.as_u128())
        };
        
        // 只广播大额交易（这里设置一个阈值）
        let threshold = U256::from(10).pow(U256::from(18)); // 1 ETH equivalent
        
        if amount0_abs < threshold && amount1_abs < threshold {
            return Ok(()); // 忽略小额交易
        }
        
        // 获取池子信息（简化实现）
        let pool_info = self.get_pool_info(&log.address).await?;
        
        // 创建交易详情
        let swap_details = UniswapV3SwapDetails {
            sender: format!("{:?}", event.sender),
            recipient: format!("{:?}", event.recipient),
            amount0: event.amount_0.to_string(),
            amount1: event.amount_1.to_string(),
            sqrt_price_x96: event.sqrt_price_x96.to_string(),
            liquidity: event.liquidity.to_string(),
            tick: event.tick,
            pool_address: format!("{:?}", log.address),
            token0: pool_info.token0,
            token1: pool_info.token1,
        };
        
        // 创建链上事件
        let chain_event = OnChainEvent::new(
            "UniswapV3Swap".to_string(),
            format!("{:?}", log.transaction_hash.unwrap_or_default()),
            log.block_number.unwrap_or_default().as_u64(),
            json!(swap_details),
        );
        
        // 创建服务器消息
        let server_message = ServerMessage::ChainEvent(chain_event);
        
        // 广播到所有房间
        self.app_state.broadcast_global(server_message).await;
        
        info!(
            "Broadcasted large swap: {} -> {} in pool {}",
            event.amount_0,
            event.amount_1,
            log.address
        );
        
        Ok(())
    }
    
    /**
     * 获取池子信息（简化实现）
     */
    async fn get_pool_info(&self, pool_address: &Address) -> Result<PoolInfo> {
        // 这里应该调用池子合约获取token0和token1地址
        // 简化实现，返回预定义的信息
        match format!("{:?}", pool_address).as_str() {
            "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640" => Ok(PoolInfo {
                token0: "USDC".to_string(),
                token1: "WETH".to_string(),
            }),
            "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8" => Ok(PoolInfo {
                token0: "USDC".to_string(),
                token1: "WETH".to_string(),
            }),
            "0xcbcdf9626bc03e24f779434178a73a0b4bad62ed" => Ok(PoolInfo {
                token0: "WBTC".to_string(),
                token1: "WETH".to_string(),
            }),
            _ => Ok(PoolInfo {
                token0: "Unknown".to_string(),
                token1: "Unknown".to_string(),
            }),
        }
    }
}

/**
 * 池子信息结构体
 */
#[derive(Debug)]
struct PoolInfo {
    token0: String,
    token1: String,
}

/**
 * 大额交易检测器
 * 用于判断交易是否值得广播
 */
pub struct LargeTransactionDetector {
    thresholds: HashMap<String, U256>,
}

impl LargeTransactionDetector {
    /**
     * 创建新的大额交易检测器
     */
    pub fn new() -> Self {
        let mut thresholds = HashMap::new();
        
        // 设置不同token的阈值
        thresholds.insert("WETH".to_string(), U256::from(10).pow(U256::from(18))); // 1 ETH
        thresholds.insert("USDC".to_string(), U256::from(10000) * U256::from(10).pow(U256::from(6))); // 10,000 USDC
        thresholds.insert("WBTC".to_string(), U256::from(1) * U256::from(10).pow(U256::from(7))); // 0.1 BTC
        
        Self { thresholds }
    }
    
    /**
     * 检查交易是否为大额交易
     */
    pub fn is_large_transaction(&self, token_symbol: &str, amount: &U256) -> bool {
        if let Some(threshold) = self.thresholds.get(token_symbol) {
            amount >= threshold
        } else {
            // 对于未知token，使用默认阈值
            let default_threshold = U256::from(1000) * U256::from(10).pow(U256::from(18));
            amount >= &default_threshold
        }
    }
}

/**
 * 格式化交易金额为人类可读格式
 */
pub fn format_amount(amount: &U256, decimals: u8, symbol: &str) -> String {
    let divisor = U256::from(10).pow(U256::from(decimals));
    let whole = amount / divisor;
    let fraction = amount % divisor;
    
    if fraction.is_zero() {
        format!("{} {}", whole, symbol)
    } else {
        // 简化小数显示
        let fraction_str = format!("{:0width$}", fraction, width = decimals as usize);
        let trimmed = fraction_str.trim_end_matches('0');
        if trimmed.is_empty() {
            format!("{} {}", whole, symbol)
        } else {
            format!("{}.{} {}", whole, trimmed, symbol)
        }
    }
}
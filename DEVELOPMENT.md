# ChainTalk 开发指南

## 项目概述

ChainTalk 是一个基于 Rust 的 Web3 实时聊天平台，支持以太坊钱包认证、Token 门禁、链上事件播报等功能。

## 技术栈

- **后端**: Rust + Tokio + Axum + Ethers-rs
- **认证**: JWT + SIWE (Sign-In with Ethereum)
- **缓存**: Redis
- **前端**: 原生 JavaScript + WebSocket
- **部署**: Docker + Docker Compose

## 快速开始

### 1. 环境准备

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 Redis (macOS)
brew install redis
brew services start redis

# 或使用 Docker 运行 Redis
docker run -d -p 6379:6379 redis:alpine
```

### 2. 配置环境变量

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑 .env 文件，配置必要的参数
vim .env
```

重要配置项：
- `ETH_WS_URL`: 以太坊 WebSocket URL (Infura/Alchemy)
- `ETH_HTTP_URL`: 以太坊 HTTP URL
- `JWT_SECRET`: JWT 签名密钥
- `REDIS_URL`: Redis 连接 URL

### 3. 启动项目

```bash
# 使用启动脚本（推荐）
./start.sh

# 或手动启动
cargo run
```

### 4. 访问应用

- 前端页面: http://localhost:3000/frontend/
- WebSocket: ws://localhost:3000/ws
- API 文档: http://localhost:3000/health

## 项目结构

```
src/
├── main.rs          # 主程序入口
├── config.rs        # 配置管理
├── error.rs         # 错误定义
├── models.rs        # 数据模型
├── state.rs         # 应用状态管理
├── auth.rs          # 认证模块
├── websocket.rs     # WebSocket 处理
├── blockchain.rs    # 区块链监听器
└── handlers.rs      # HTTP 处理器
```

## 核心功能

### 1. Web3 认证

使用 SIWE (Sign-In with Ethereum) 标准进行用户认证：

1. 用户连接钱包
2. 获取认证 nonce
3. 签名认证消息
4. 验证签名并生成 JWT

### 2. WebSocket 通信

支持的消息类型：
- `authenticate`: 用户认证
- `send_text`: 发送文本消息
- `join_room`: 加入房间
- `leave_room`: 离开房间

### 3. 区块链监听

监听 Uniswap V3 Swap 事件，自动播报大额交易：

```rust
// 配置监听的合约地址
let factory_address = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
```

### 4. Token 门禁

支持基于 ERC20/ERC721 的房间访问控制：

```rust
pub struct TokenGate {
    pub token_type: TokenGateType,
    pub contract_address: String,
    pub minimum_balance: String,
}
```

## API 接口

### REST API

- `POST /api/auth/nonce` - 获取认证 nonce
- `POST /api/auth/login` - 用户登录
- `GET /api/user/info` - 获取用户信息
- `GET /api/rooms` - 获取房间列表
- `GET /health` - 健康检查

### WebSocket API

连接地址: `ws://localhost:3000/ws`

消息格式:
```json
{
  "type": "message_type",
  "content": "message_content",
  "room": "room_name"
}
```

## 开发调试

### 1. 启用详细日志

```bash
export RUST_LOG=debug
cargo run
```

### 2. 测试 WebSocket 连接

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');
ws.onopen = () => console.log('Connected');
ws.onmessage = (event) => console.log('Message:', event.data);
```

### 3. 测试 API 接口

```bash
# 健康检查
curl http://localhost:3000/health

# 获取 nonce
curl -X POST http://localhost:3000/api/auth/nonce \
  -H "Content-Type: application/json" \
  -d '{"address":"0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b"}'
```

## Docker 部署

### 1. 构建镜像

```bash
docker build -t chaintalk .
```

### 2. 使用 Docker Compose

```bash
docker-compose up -d
```

### 3. 查看日志

```bash
docker-compose logs -f chaintalk
```

## 常见问题

### 1. Redis 连接失败

确保 Redis 服务正在运行：
```bash
redis-cli ping
# 应该返回 PONG
```

### 2. 区块链连接失败

检查 `.env` 文件中的以太坊节点 URL 是否正确。

### 3. WebSocket 连接失败

确保防火墙允许 3000 端口，并检查 CORS 配置。

## 贡献指南

1. Fork 项目
2. 创建功能分支: `git checkout -b feature/new-feature`
3. 提交更改: `git commit -am 'Add new feature'`
4. 推送分支: `git push origin feature/new-feature`
5. 创建 Pull Request

## 许可证

MIT License - 详见 LICENSE 文件
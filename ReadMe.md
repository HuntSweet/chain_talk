# ChainTalk - 实时链上交互聊天室

基于 Rust 开发的 Web3 实时聊天平台，支持钱包认证、Token门禁和链上事件广播。

## 🚀 功能特性

### 核心功能 (V1.0)
- ✅ **Web3 钱包认证**: 通过 MetaMask 等钱包进行 SIWE (Sign-In with Ethereum) 认证
- ✅ **实时聊天**: 基于 WebSocket 的实时消息传输
- ✅ **Token门禁访问控制**: 支持 ERC20/ERC721/ERC1155 token 持有者专属房间
- ✅ **链上事件播报**: 实时监听和广播 Uniswap V3 大额交易等链上事件
- ✅ **在线用户列表**: 实时显示房间内用户（钱包地址/ENS 名称）

### 技术架构
- **后端**: Rust + Tokio + Axum + Ethers-rs
- **WebSocket**: 实时双向通信
- **认证**: JWT + SIWE (EIP-4361)
- **缓存**: Redis (会话管理、nonce 存储)
- **区块链**: Ethereum (通过 Infura/Alchemy)
- **部署**: Docker + Docker Compose + Nginx

## 📦 快速开始

### 环境要求
- Rust 1.75+
- Redis 6.0+
- Docker & Docker Compose (可选)
- Ethereum RPC 节点访问 (Infura/Alchemy)

### 本地开发

1. **克隆项目**
```bash
git clone https://github.com/HuntSweet/chain_talk
cd chainTalk
```

2. **配置环境变量**
```bash
cp .env.example .env
# 编辑 .env 文件，填入你的配置
```

3. **启动 Redis**
```bash
# 使用 Docker
docker run -d -p 6379:6379 redis:7-alpine

# 或使用本地安装的 Redis
redis-server
```

4. **运行应用**
```bash
cargo run
```

应用将在 `http://localhost:3000` 启动。

### Docker 部署

1. **配置环境变量**
```bash
cp .env.example .env
# 编辑 .env 文件
```

2. **启动服务**
```bash
docker-compose up -d
```

3. **查看日志**
```bash
docker-compose logs -f chaintalk
```

## 🔧 配置说明

### 环境变量

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `SERVER_ADDRESS` | 服务器监听地址 | `0.0.0.0:3000` |
| `REDIS_URL` | Redis 连接 URL | `redis://localhost:6379` |
| `ETHEREUM_WS_URL` | 以太坊 WebSocket RPC URL | 必填 |
| `ETHEREUM_HTTP_URL` | 以太坊 HTTP RPC URL | 必填 |
| `JWT_SECRET` | JWT 签名密钥 | 必填 |
| `CORS_ORIGINS` | 允许的跨域来源 | `http://localhost:3000,http://localhost:5173` |
| `UNISWAP_V3_FACTORY` | Uniswap V3 工厂合约地址 | `0x1F98431c8aD98523631AE4a59f267346ea31F984` |
| `DEFAULT_ROOM` | 默认聊天室名称 | `general` |
| `RUST_LOG` | 日志级别 | `info` |

### 获取 Ethereum RPC 访问

1. 注册 [Infura](https://infura.io/) 或 [Alchemy](https://www.alchemy.com/)
2. 创建新项目获取 API Key
3. 配置 WebSocket 和 HTTP 端点：
   ```
   ETHEREUM_WS_URL=wss://mainnet.infura.io/ws/v3/YOUR_PROJECT_ID
   ETHEREUM_HTTP_URL=https://mainnet.infura.io/v3/YOUR_PROJECT_ID
   ```

## 📡 API 文档

### REST API

#### 获取认证 Nonce
```http
GET /api/nonce
```

响应:
```json
{
  "nonce": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
}
```

#### 用户登录
```http
POST /api/login
Content-Type: application/json

{
  "message": "localhost wants you to sign in with your Ethereum account:\n0x...",
  "signature": "0x..."
}
```

响应:
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user": {
    "address": "0x...",
    "ens_name": "vitalik.eth",
    "avatar": null
  }
}
```

#### 健康检查
```http
GET /health
```

### WebSocket API

#### 连接
```
wss://localhost:3000/ws
```

认证后发送 JWT token 作为第一条消息。

#### 消息格式

**客户端 → 服务端**:
```json
{
  "type": "SendText",
  "payload": {
    "room": "general",
    "text": "Hello, ChainTalk!"
  }
}
```

**服务端 → 客户端**:
```json
{
  "type": "NewText",
  "payload": {
    "id": "msg-123",
    "from": "vitalik.eth",
    "text": "Hello, ChainTalk!",
    "room": "general",
    "timestamp": "2024-01-01T12:00:00Z"
  }
}
```

## 🏗️ 项目结构

```
src/
├── main.rs              # 应用入口点
├── config.rs            # 配置管理
├── error.rs             # 错误处理
├── models.rs            # 数据模型定义
├── state.rs             # 应用状态管理
├── auth.rs              # 认证服务 (SIWE + JWT)
├── websocket.rs         # WebSocket 处理
├── blockchain.rs        # 区块链事件监听
└── handlers.rs          # HTTP 处理器
```

## 🔐 安全考虑

1. **JWT 密钥**: 使用强随机密钥，定期轮换
2. **CORS 配置**: 仅允许信任的域名
3. **Rate Limiting**: 考虑添加请求频率限制
4. **输入验证**: 所有用户输入都经过验证和清理
5. **HTTPS**: 生产环境必须使用 HTTPS

## 🚧 开发计划

### V1.1 (计划中)
- [ ] 多房间支持
- [ ] 用户创建自定义房间
- [ ] 更多链上事件类型支持
- [ ] ENS 头像显示

### V1.2 (计划中)
- [ ] 交互式命令 (`!price WETH`)
- [ ] 用户链上身份画像
- [ ] 端到端加密私信
- [ ] 消息历史持久化

## 🤝 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [Ethers-rs](https://github.com/gakonst/ethers-rs) - Ethereum 库
- [Axum](https://github.com/tokio-rs/axum) - Web 框架
- [SIWE](https://github.com/spruceid/siwe-rs) - 以太坊登录标准
- [Tokio](https://tokio.rs/) - 异步运行时

---

**ChainTalk** - 让 Web3 社区连接更紧密 🌐⛓️💬

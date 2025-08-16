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
git clone <repository-url>
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

---

# 原始产品与技术规格文档


1. 文档概述


1.1 项目愿景 (Project Vision)

ChainTalk 旨在打造一个专为 Web3 社区设计的实时、安全、且具备链上身份感知能力的社交平台。它不仅仅是一个聊天工具，更是一个能实时反映链上动态、验证社区成员身份、并促进高价值信息流动的“Alpha”聚集地。

1.2 文档目的

本文档旨在明确 ChainTalk 项目 V1.0 版本的核心功能、技术架构和实施细节，为开发、测试和未来迭代提供清晰的指导。

1.3 目标用户 (User Persona)

DeFi 交易员与研究员: 希望获取实时交易动态、讨论市场策略的用户。
NFT 项目社区成员: 希望在一个持有者专属的环境中交流、获取项目方信息的群体。
DAO 组织成员: 需要一个基于链上身份进行讨论和决策的平台。

2. 产品需求文档 (Product Requirements Document - PRD)


2.1 核心功能 (MVP - V1.0)

功能ID
功能模块
功能描述
优先级
F1
核心聊天功能
用户可以加入默认聊天室，实时发送和接收文本消息。
核心
F2
Web3 钱包认证
用户通过连接钱包（如 MetaMask）并签名消息来登录系统，无需传统的用户名密码。
核心
F3
Token-Gated 访问控制
系统支持创建“门禁”聊天室。只有持有特定 NFT 或达到指定 ERC20 代币数量的用户才能进入。
核心
F4
链上事件播报机器人
聊天室内有一个名为 ChainWatch Bot 的机器人，能够实时播报指定的链上事件（如 Uniswap 大额 Swap）。
核心
F5
在线用户列表
客户端界面能实时显示当前聊天室内的在线用户钱包地址（或 ENS 名称）。
高

2.2 远期规划 (Future Scope)

多房间支持: 用户可以创建或加入多个不同主题或不同准入门槛的聊天室。
交互式命令: 用户可通过 / 或 ! 命令查询链上数据，如 !price WETH。
用户链上身份画像: 展示用户的 ENS、主要 NFT 头像、关键链上交互历史等。
端到端加密私信: 用户之间可以发起加密的私人对话。

3. 技术规格文档 (Technical Design Document - TDD)


3.1 系统架构 (System Architecture)

+----------------+      +----------------+      +------------------+
|   Frontend     |      | Nginx Reverse  |      |  Blockchain Node |
| (React/Vue)    |<---->|     Proxy      |<---->| (Infura/Alchemy) |
+----------------+      +----------------+      +------------------+
       ^                       ^                         ^
       | HTTP/WebSocket        |                         | WSS
       v                       v                         v
+-------------------------------------------------------------------+
|                           Rust Backend (`tokio`)                    |
|                                                                   |
| +---------------------+      +---------------------------------+  |
| | Blockchain Listener |      |      WebSocket Server (`axum`)    |  |
| | (`ethers-rs`)       |----->| - Manages user connections        |  |
| | - Subscribes to     | MPSC | - Handles Auth (SIWE)             |  |
| |   on-chain events   | Chan | - Broadcasts messages             |  |
| +---------------------+      +---------------------------------+  |
|                                      ^                         ^  |
|                                      |                         |  |
|                                      v                         v  |
|                            +------------------+      +-------------+
|                            |  Shared State    |      | Redis       |
|                            | (Arc<Mutex<...>>)|      | - Sessions  |
|                            +------------------+      | - Nonces    |
+-------------------------------------------------------------------+


3.2 技术栈 (Tech Stack)

后端: Rust, Tokio, Axum, Ethers-rs, Tokio-Tungstenite, Serde, BB8 (Redis 连接池), Thiserror
前端: React / Vue.js, Ethers.js, wagmi, viem
数据库/缓存: Redis (用于存储会话、认证随机数 Nonce)
区块链节点: Infura / Alchemy (通过 WebSocket 订阅事件)
部署: Docker, Nginx, Linux (Ubuntu)

3.3 核心组件详述

1. WebSocket 服务 (axum)
连接生命周期:
接收来自客户端的 HTTP GET 请求 (e.g., /ws)。
通过 axum::extract::ws::WebSocketUpgrade 将其升级为 WebSocket 连接。
在升级过程中，可以进行初步的认证（如检查请求头中的 JWT）。
为每个成功的连接 tokio::spawn 一个独立的 handle_connection 任务。
共享状态管理:
定义一个全局的 AppState 结构体，通过 axum::Extension 或 State 注入到 Handler 中。
AppState 将包含 Arc<Mutex<HashMap<UserId, Client>>>，用于管理所有连接的客户端。
Client 结构体包含用于向客户端发送消息的 mpsc::Sender。
消息处理:
handle_connection 任务会进入一个循环，监听来自客户端的消息。
收到的消息经过反序列化和业务逻辑处理（如权限检查、命令解析），然后通过一个中央的 mpsc 通道广播给所有其他客户端。
2. 区块链监听器 (ethers-rs)
独立任务: 在程序启动时，通过 tokio::spawn 启动一个常驻的后台任务。
事件订阅:
使用 ethers::providers::Provider::new_ws() 创建一个 WebSocket Provider。
实例化一个 ethers::contract::Contract 对象，指向目标合约（如 Uniswap V3 Pool）。
调用 contract.event::<SwapFilter>() 获取事件流 EventStream。
在一个 while let Some(Ok(log)) 循环中处理接收到的事件。
组件通信:
监听器任务持有一个全局 mpsc::Sender 的克隆。
当监听到符合条件的链上事件时，将其格式化为一个内部消息结构体，并通过 sender.send() 发送出去。
WebSocket 服务端的广播任务是这个 mpsc::Receiver 的唯一消费者。这种方式完美地解耦了两个核心模块。
3. 认证与授权模块 (Sign-In with Ethereum - SIWE)
流程:
GET /api/nonce: 前端请求一个用于签名的随机数。后端生成一个 nonce，与用户 IP 等信息关联后存入 Redis 并设置短暂过期时间。
POST /api/login:
前端构建一个标准的 EIP-4361 (SIWE) 消息（包含域名、地址、Nonce 等），并使用 ethers.js 进行签名。
前端将原始消息和签名发送到后端。
后端使用 ethers-rs 或专门的 siwe crate 验证签名和 nonce 的有效性。
验证成功后，进行 Token-Gated 检查：调用 ethers-rs 查询用户地址的代币/NFT 持有量。
检查通过后，生成一个 JWT (JSON Web Token)，其中包含用户地址、角色、会话有效期等信息，并返回给前端。
WebSocket 连接: 前端在发起 WebSocket 连接请求时，将此 JWT 放入请求头或作为查询参数。后端在升级连接前验证 JWT 的有效性。

3.4 数据模型 (Data Models - Rust Structs)

Rust
// --- WebSocket 消息协议 ---
// 从客户端发往服务端
#[derive(Deserialize)]
#[serde(tag = "type", content = "payload")]
enum ClientMessage {
    SendText { room: String, text: String },
    JoinRoom { room: String },
}

// 从服务端广播给客户端
#[derive(Serialize, Clone)]
#[serde(tag = "type", content = "payload")]
enum ServerMessage {
    NewText { from: String, text: String, room: String }, // from 可以是地址或ENS
    UserJoined { user: String, room: String },
    UserLeft { user: String, room: String },
    ChainEvent(OnChainEvent),
}

// --- 链上事件数据模型 ---
#[derive(Serialize, Clone)]
pub struct OnChainEvent {
    pub event_type: String, // e.g., "UniswapV3Swap"
    pub transaction_hash: String,
    pub block_number: u64,
    pub details: serde_json::Value, // 使用 Value 可以灵活表示不同事件的细节
}

// --- 服务端内部状态 ---
// 代表一个已连接的客户端
struct Client {
    user_address: String,
    sender: mpsc::Sender<ServerMessage>,
}

// 全局应用状态
struct AppState {
    rooms: HashMap<String, HashSet<String>>, // room_name -> set_of_user_addresses
    clients: HashMap<String, Client>, // user_address -> Client
}


3.5 API 接口定义

1. REST API (用于认证)
GET /api/nonce
响应: 200 OK
Body: {"nonce": "a1b2c3d4e5f6"}
POST /api/login
Body: {"message": "...", "signature": "0x..."}
成功响应: 200 OK, Body: {"token": "ey..."}
失败响应: 401 Unauthorized, 403 Forbidden (Token 检查未通过)
2. WebSocket API (消息格式见 3.4)
连接地址: wss://yo
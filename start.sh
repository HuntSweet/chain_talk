#!/bin/bash

# ChainTalk 启动脚本

echo "🔗 启动 ChainTalk 项目..."

# 检查是否存在 .env 文件
if [ ! -f ".env" ]; then
    echo "⚠️  未找到 .env 文件，正在从 .env.example 复制..."
    cp .env.example .env
    echo "✅ 已创建 .env 文件，请根据需要修改配置"
fi

# 检查 Redis 是否运行
echo "🔍 检查 Redis 服务..."
if ! redis-cli ping > /dev/null 2>&1; then
    echo "⚠️  Redis 未运行，请先启动 Redis 服务"
    echo "   macOS: brew services start redis"
    echo "   或使用 Docker: docker run -d -p 6379:6379 redis:alpine"
    exit 1
fi

echo "✅ Redis 服务正常"

# 构建项目
echo "🔨 构建 Rust 项目..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ 构建失败"
    exit 1
fi

echo "✅ 构建成功"

# 启动服务器
echo "🚀 启动 ChainTalk 服务器..."
echo "📱 前端页面: http://localhost:3000/frontend/"
echo "🔗 WebSocket: ws://localhost:3000/ws"
echo "📊 健康检查: http://localhost:3000/health"
echo ""
echo "按 Ctrl+C 停止服务器"
echo ""

# 设置环境变量并启动
export RUST_LOG=info
cargo run --release
# 使用官方Rust镜像作为构建环境
FROM rust:1.75 as builder

# 设置工作目录
WORKDIR /app

# 复制Cargo文件
COPY Cargo.toml Cargo.lock ./

# 创建一个虚拟的main.rs来缓存依赖
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs

# 复制源代码
COPY src ./src

# 构建应用
RUN touch src/main.rs
RUN cargo build --release

# 使用轻量级的运行时镜像
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 创建应用用户
RUN useradd -r -s /bin/false chaintalk

# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/chaintalk /app/chaintalk

# 更改所有权
RUN chown -R chaintalk:chaintalk /app

# 切换到应用用户
USER chaintalk

# 暴露端口
EXPOSE 3000

# 运行应用
CMD ["./chaintalk"]
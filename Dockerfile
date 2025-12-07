FROM rust:1.91-slim-bullseye AS builder

# 安裝基本工具和更新證書
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

WORKDIR /app
 
# 複製 Cargo.toml 和 Cargo.lock
COPY Cargo.toml Cargo.lock ./

# 創建虛擬的 src 目錄和 main.rs 以預先建構依賴
RUN mkdir -p src && \
    echo 'fn main() { println!("Dummy!"); }' > src/main.rs

# 預先建構依賴
RUN cargo build --release && \
    rm -f target/release/deps/its_mypic_image_server* && \
    rm src/main.rs

# 複製實際的源碼
COPY . .

# 執行最終建構
RUN cargo build --release && \
    cp target/release/its_mypic_image_server /app/its_mypic_image_server

# 使用 distroless 作為最終映像
FROM gcr.io/distroless/cc

WORKDIR /app

# 複製 ffmpeg
COPY --from=mwader/static-ffmpeg:7.1 /ffmpeg /usr/local/bin/

# 複製編譯好的執行檔
COPY --from=builder /app/its_mypic_image_server .

# 設定執行指令
CMD ["/app/its_mypic_image_server"]

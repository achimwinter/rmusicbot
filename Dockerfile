# syntax=docker/dockerfile:1.2
# Builder stage
FROM rust:1-bookworm as builder

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    cmake \
    libssl-dev \
    perl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN rm -f Cargo.lock
RUN cargo build --release --no-default-features

# Runtime Stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    yt-dlp \
    ffmpeg \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/rmusicbot .

CMD ["./rmusicbot"]
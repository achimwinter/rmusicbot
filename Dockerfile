# syntax=docker/dockerfile:1.2
# Builder stage
FROM rust as builder

RUN apt-get -qq update && \
    apt-get install -y \
    build-essential \
    ffmpeg \
    youtube-dl \
    pkg-config \
    cmake \
    libssl-dev \
    openssl


WORKDIR /app
COPY . .

# Pre compile dependecies so that docker has a chance to cache?
RUN mv src/main.rs src/lib.rs
RUN cargo build --release --no-default-features
RUN mv src/lib.rs src/main.rs

# Build the project itself
RUN cargo build --release --no-default-features

# Runtime Stage
FROM ubuntu:latest

RUN apt-get update -qq && apt-get install -y ffmpeg openssl wget

RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux
RUN mv yt-dlp_linux yt-dlp
RUN chmod +x yt-dlp
RUN export PATH="/app:$PATH"

WORKDIR /app
COPY --from=builder /app/target/release/rmusicbot .

CMD ["./rmusicbot"]
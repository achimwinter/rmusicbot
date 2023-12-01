# syntax=docker/dockerfile:1.2
# Builder stage
FROM rust:1.74-alpine as builder

RUN apk add --no-cache \
    alpine-sdk \
    pkgconfig \
    cmake \
    openssl-dev \
    musl-dev

WORKDIR /app
COPY . .

# Pre compile dependecies so that docker has a chance to cache?
RUN mv src/main.rs src/lib.rs
RUN cargo build --release --no-default-features
RUN mv src/lib.rs src/main.rs

# Build the project itself
RUN cargo build --release --no-default-features

# Runtime Stage
FROM alpine:latest

RUN apk add --no-cache yt-dlp ffmpeg openssl

WORKDIR /app
COPY --from=builder /app/target/release/rmusicbot .

CMD ["./rmusicbot"]
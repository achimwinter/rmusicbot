# syntax=docker/dockerfile:1.2

# Builder stage
FROM rust as builder

WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --no-default-features

# Runtime Stage
FROM ubuntu:latest
RUN apt-get update -qq && apt-get install -y ffmpeg youtube-dl openssl
WORKDIR /app
COPY --from=builder /app/target/release/rmusicbot .

CMD ["./rmusicbot"]
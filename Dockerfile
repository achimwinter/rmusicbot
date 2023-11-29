# syntax=docker/dockerfile:1.2

# Builder stage
FROM rust as builder

RUN apt-get -qq update && apt-get install -y \
    cmake

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

RUN apt-get update -qq && apt-get install -y ffmpeg youtube-dl openssl
WORKDIR /app
COPY --from=builder /app/target/release/rmusicbot .

CMD ["./rmusicbot"]
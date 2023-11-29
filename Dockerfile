# syntax=docker/dockerfile:1.2

# Builder stage
FROM rust as builder

RUN apt-get -qq update && apt-get install -y \
    cmake

WORKDIR /app
COPY . .
RUN cargo build --release --no-default-features

# Runtime Stage
FROM ubuntu:latest

RUN apt-get update -qq && apt-get install -y ffmpeg youtube-dl openssl
WORKDIR /app
COPY --from=builder /app/target/release/rmusicbot .

CMD ["./rmusicbot"]
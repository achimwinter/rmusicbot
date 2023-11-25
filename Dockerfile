FROM rust:1.74-alpine as builder

RUN apk add --update \
    alpine-sdk \
    ffmpeg \
    youtube-dl \
    pkgconfig \
    cmake \
    openssl-dev \
    musl-dev \
    openssl

WORKDIR /app

COPY . .

RUN cargo build --release


FROM alpine:latest
RUN apk add --no-cache ffmpeg youtube-dl openssl
WORKDIR /app
COPY --from=builder /app/.env .
COPY --from=builder /app/target/release/rmusicbot .

CMD ["./rmusicbot"]

FROM rust:alpine as builder
RUN apk add openssl-dev build-base git

WORKDIR /usr/src/app

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest as production-runtime
RUN apk add openssl-dev build-base git supervisor

RUN mkdir -p /usr/src/app /usr/src/bin

RUN chmod +x /usr/src/bin/carbonable-juno-starknet-bridge

EXPOSE 8080
CMD ["supervisord", "--nodaemon", "-c", "/etc/supervisord.conf"]

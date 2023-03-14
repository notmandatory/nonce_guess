FROM rust:1.67 as builder

WORKDIR /usr/src/nonce_guess
COPY . .

RUN rustup target add wasm32-unknown-unknown && cargo install trunk
RUN trunk build --release ng_web/index.html && cargo build --bin ng_server --release
RUN cargo install --path ./ng_server

FROM debian:11

WORKDIR /root

RUN apt update && apt upgrade -y
RUN apt install -y git build-essential openssl librust-openssl-dev software-properties-common

COPY --from=builder /usr/local/cargo/bin/ng_server .
EXPOSE 8081

CMD ["RUST_LOG=debug ./ng_server"]
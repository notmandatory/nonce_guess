FROM docker.io/library/rust:1.67 as builder

WORKDIR /usr/src/nonce_guess
COPY . .

RUN trunk build --release

FROM docker.io/library/debian:11

WORKDIR /root

RUN apt update && apt upgrade -y
RUN apt install -y git build-essential openssl librust-openssl-dev software-properties-common

COPY --from=builder /usr/src/nonce_guess/target/release/ng_server .
EXPOSE 8081

VOLUME /data

ENV RUST_LOG=debug
CMD ["./ng_server","-l","0.0.0.0:8081", "-d", "/data/nonce_guess.db"]

FROM docker.io/library/rust:1.84 AS builder

WORKDIR /usr/src/nonce_guess
COPY . .

RUN apt update && apt upgrade -y
RUN apt install curl
RUN curl -sLO https://github.com/tailwindlabs/tailwindcss/releases/download/v4.0.0/tailwindcss-linux-x64 && mv tailwindcss-linux-x64 tailwindcss && chmod +x tailwindcss
ENV PATH=$PATH:.
RUN cargo build --release

FROM docker.io/library/debian:bookworm-slim

WORKDIR /root

RUN apt update && apt upgrade -y
RUN apt install -y pkg-config openssl

COPY --from=builder /usr/src/nonce_guess/target/release/ng_server .
EXPOSE 8081

VOLUME /data

ENV RUST_LOG=debug
ENV NONCE_GUESS_DB_FILE=/data/nonce_guess.redb
CMD ["./ng_server"]

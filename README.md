# Nonce Guess

### Setup

1. Install system packages: gcc, libssl-dev, pkg-config
2. cargo install trunk

### Local Testing

Required Tools:

1. [tailwindcss](https://github.com/tailwindlabs/tailwindcss)

Build and Run:

1. Start the server, it will also serve the latest web client
   ```shell
   RUST_LOG=debug cargo run

By default, the data is stored in SQLite memory database. For authentication the URL must use `localhost` and not `127.0.0.1`, e.g. `http://localhost:3000`.

### Create Release Build

1. Build the server binary, this will include the web artifacts
   ```shell
   cargo build --release
   ```

To run the resulting self contained binary use `RUST_LOG=debug target/release/ng_server`.

In test or release mode the web client can be found at: http://localhost:3000/

### Build Docker Container

1. `docker build -t nonce_guess .`
2. `docker run -d --rm -it -p 8081:8081 -v nonce_vol:/data --name nonce_guess_app nonce_guess`
3. Visit http://localhost:8081/ in a browser

Note: on linux above steps also work with `podman` instead of `docker`.

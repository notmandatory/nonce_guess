# Nonce Guess

### Local Testing

Install:

pnpm
cargo-watch

From ng_server directory:

1. Create tailwind css file and watch for changes
   ```shell
   pnpm dlx tailwindcss -i ./styles/tailwind.css -o ./assets/main.css --watch
   ```

2. Start the api server, it will also serve the latest web client
   ```shell
   RUST_LOG=debug cargo watch -- cargo run --bin ng_server
   ```
   
By default, the data is stored in SQLite memory database. For authentication the URL must use `localhost` and not `127.0.0.1`, e.g. `http://localhost:8081`.
   
### Create Release Build

1. Build the api server binary, this will include the web artifacts
   ```shell
   cargo build --bin ng_server --release
   ```

To run the resulting self contained binary use `RUST_LOG=debug target/release/ng_server`.

In test or release mode the web client can be found at: http://localhost:8081/

### Build Docker Container

1. `docker build -t nonce_guess .`
2. `docker run -d --rm -it -p 8081:8081 -v nonce_vol:/data --name nonce_guess_app nonce_guess`
3. Visit http://localhost:8081/ in a browser

Note: above steps also work with `podman` instead of `docker`.


# Nonce Guess

### Local Testing

1. Start the wasm web client builder in watch mode
   ```shell
   cd ng_web
   trunk watch
   ```

2. Start the api server, it will also serve the latest web client
   ```shell
   RUST_LOG=debug cargo run --bin ng_server
   ```
   
By default the data is stored in SQLite memory database. 
   
### Create Release Build

1. Build the web client artifacts
   ```shell
   cd ng_web
   trunk build --release
   ```

2. Build the api server binary, this will include the web artifacts
   ```shell
   cargo build --bin ng_server --release
   ```

To run the resulting self contained binary use `RUST_LOG=debug target/release/ng_server`.

In test or release mode the web client can be found at: http://127.0.0.1:8081/

### Build Docker Container

1. `docker build -t nonce_guess .`
2. `docker run --rm -it -p 8081:8081 --name nonce_guess_app nonce_guess`
3. Visit http://127.0.0.1:8081/ in a browser
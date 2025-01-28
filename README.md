# Nonce Guess

### Setup

1. Install system packages: gcc, libssl-dev, pkg-config
2. cargo install trunk

### Local Testing

Required Tools:

1. Install [tailwindcss](https://github.com/tailwindlabs/tailwindcss)
```shell
npm install tailwindcss @tailwindcss/cli
```

Build and Run:

1. Set env variables, defaults are
   ```shell
   # if `NONCE_GUESS_DB_FILE` not set the data is stored in temporary file.
   export NONCE_GUESS_DB_FILE=/data/nonce_guess.redb
   export NONCE_GUESS_DOMAIN_NAME=localhost
   export NONCE_GUESS_WEB_URL=http://localhost:8081
   ```
2. Start the server, it will also serve the latest web client
   ```shell
   npx tailwindcss -c tailwind.config.js -i styles/tailwind.css -o assets/main.css --watch
   RUST_LOG=debug cargo run
   ```

### Create Release Build

1. Build the server binary, this will include the web artifacts
   ```shell
   npx tailwindcss -c tailwind.config.js -i styles/tailwind.css -o assets/main.css
   cargo build --release
   ```

To run the resulting self-contained binary use `RUST_LOG=debug target/release/ng_server`.

In test or release mode the web client can be found at: http://localhost:8081/

### Build Docker Container

1. `npx tailwindcss -c tailwind.config.js -i styles/tailwind.css -o assets/main.css`
2. `docker build -t nonce_guess .`
3. `docker run -d --rm -it -p 8081:8081 -v nonce_vol:/data --name nonce_guess_app nonce_guess`
4. Visit http://localhost:8081/ in a browser

Note: on linux above steps also work with `podman` instead of `docker`.

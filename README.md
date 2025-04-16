# Nonce Guess

### Local Testing

Tools:

1.

Install [tailwindcss](https://github.com/tailwindlabs/tailwindcss) [standalone-cli](https://tailwindcss.com/blog/standalone-cli)
and make sure it's in your executables path, for example:

```shell
curl -sLO https://github.com/tailwindlabs/tailwindcss/releases/download/v4.0.0/tailwindcss-macos-arm64
chmod +x tailwindcss-macos-arm64
mv tailwindcss-macos-arm64 ~/bin/tailwindcss
```

2. Prettier plugin for tailwindcss (optional):
   ```shell
   npm install -D prettier prettier-plugin-tailwindcss
   ```

Build and Run:

1. Set env variables, defaults are
   ```shell
   # if `NONCE_GUESS_DB_FILE` not set the data is stored in temporary file.
   export NONCE_GUESS_DB_FILE=/data/nonce_guess.redb
   export NONCE_GUESS_MEMPOOL_URL=/data/nonce_guess.redb
   ```
2. Start the server, it will also serve the latest web client
   ```shell
   RUST_LOG=debug cargo run
   ```

### Create Release Build

1. Build the server binary, this will include the web artifacts
   ```shell
   cargo build --release
   ```

To run the resulting self-contained binary use `RUST_LOG=debug target/release/ng_server`.

In test or release mode the web client can be found at: http://localhost:8081/

### Build Docker Container

1. `docker build -t nonce_guess .`
2. `docker run -d --rm -it -p 8081:8081 -v nonce_vol:/data --name nonce_guess_app nonce_guess`
3. Visit http://localhost:8081/ in a browser

Note: on linux above steps also work with `podman` instead of `docker`.

### Build for pushing to DockerHub

1. `docker login`
2. `docker build --platform linux/amd64 -t notmandatory/nonce_guess:latest .`
3. `docker push notmandatory/nonce_guess:latest`
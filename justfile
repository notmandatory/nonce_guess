d:
    RUST_LOG=debug cargo run
r:
    cargo build --release
tb:
    tailwindcss -c tailwind.config.js -i styles/tailwind.css -o assets/main.css

tw:
    tailwindcss -c tailwind.config.js -i styles/tailwind.css -o assets/main.css --watch

db:
    docker build -t nonce_guess .

dr:
    docker run -d --rm -it -p 8081:8081 -v nonce_vol:/data --name nonce_guess_app nonce_guess

dl:
    docker logs -f nonce_guess_app

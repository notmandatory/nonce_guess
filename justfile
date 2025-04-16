d:
    RUST_LOG=debug cargo run

r:
    cargo build --release

tb:
    npx @tailwindcss/cli -c tailwind.config.js -i styles/tailwind.css -o assets/main.css

tw:
    npx @tailwindcss/cli -c tailwind.config.js -i styles/tailwind.css -o assets/main.css --watch

db:
    docker build -t nonce_guess .

dr:
    docker run -d --rm -it -p 8080:8080 -v nonce_vol:/data --name nonce_guess_app nonce_guess

dl:
    docker logs -f nonce_guess_app

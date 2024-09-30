d:
    RUST_LOG=debug cargo run
r:
    cargo build --release
tb:
    tailwindcss -c tailwind.config.js -i styles/tailwind.css -o assets/main.css

tw:
    tailwindcss -c tailwind.config.js -i styles/tailwind.css -o assets/main.css --watch

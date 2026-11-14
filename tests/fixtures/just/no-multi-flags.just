test:
    cargo test --all --locked --frozen

build:
    cargo build --release --target=x86_64

deploy:
    docker run --name=app --port=8080

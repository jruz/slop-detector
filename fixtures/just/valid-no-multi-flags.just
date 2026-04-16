test:
    cargo test --all --locked

build:
    cargo build --release

build-target:
    cargo build --target=x86_64

run:
    docker run \
        --name=app \
        --port=8080

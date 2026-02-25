build:
    #!/usr/bin/env nu
    cargo build --release
    git add .

deploy:
    #!/usr/bin/env nu
    docker compose up

clean:
    echo "no shebang, plain just"

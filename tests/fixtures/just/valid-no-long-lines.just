short:
    echo "ok"

long-command:
    curl \
        --silent \
        --show-error \
        --fail \
        --location \
        https://example.com/api/endpoint

build:
    just clean
    just compile
    echo "done"

deploy:
    just build
    just test
    just lint
    echo "deploying"

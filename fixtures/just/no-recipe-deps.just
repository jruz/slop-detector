build: clean compile
    echo "done"

deploy: build test lint
    echo "deploying"

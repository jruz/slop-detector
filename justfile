default:
    @just --list

shell:
    nix develop --command nu

rust-build:
    cargo build --all-targets

rust-cargo-check:
    cargo check --all-targets

rust-check:
    just rust-cargo-check
    just rust-lint
    just rust-test

rust-clippy *ARGS='':
    cargo clippy --all-targets {{ARGS}}

rust-clippy-fix *ARGS='':
    cargo clippy --fix --all-targets --allow-dirty {{ARGS}}

rust-fix:
    just rust-fmt-fix
    just rust-clippy-fix

rust-fmt-check:
    cargo fmt --all -- --check

rust-fmt-fix:
    cargo fmt --all

rust-lint:
    just rust-fmt-check
    just rust-clippy

rust-outdated:
    cargo outdated

rust-test *FILTER='':
    cargo nextest run --no-tests=pass {{FILTER}}

rust-upgrade:
    cargo upgrade --incompatible
    cargo update

nu-check:
    #!/usr/bin/env nu
    (^fd
        --absolute-path
        --extension nu
        --exec nu --commands 'nu-check --debug {}')
    nu-lint --config nu-lint.toml

nu-fix:
    nu-lint \
        --config nu-lint.toml \
        --fix

test:
    just rust-test

woodpecker-check:
    woodpecker-cli lint \
        --strict \
        .woodpecker/

check:
    just nu-check
    just rust-check
    just woodpecker-check

build:
    cargo build --release

# ---------------- RELEASE BUILDS

build-linux:
    rustup target add x86_64-unknown-linux-gnu
    rustup target add aarch64-unknown-linux-gnu
    cargo zigbuild --release --target x86_64-unknown-linux-gnu
    cargo zigbuild --release --target aarch64-unknown-linux-gnu
    cp target/x86_64-unknown-linux-gnu/release/slop-detector \
        slop-detector-linux-x86_64
    cp target/aarch64-unknown-linux-gnu/release/slop-detector \
        slop-detector-linux-arm64

build-macos:
    rustup target add x86_64-apple-darwin
    rustup target add aarch64-apple-darwin
    cargo zigbuild --release --target x86_64-apple-darwin
    cargo zigbuild --release --target aarch64-apple-darwin
    cp target/x86_64-apple-darwin/release/slop-detector \
        slop-detector-macos-x86_64
    cp target/aarch64-apple-darwin/release/slop-detector \
        slop-detector-macos-arm64

build-windows:
    rustup target add x86_64-pc-windows-msvc
    cargo xwin build --release --target x86_64-pc-windows-msvc
    cp target/x86_64-pc-windows-msvc/release/slop-detector.exe \
        slop-detector-windows-x86_64.exe

# ---------------- CI WRAPPERS

ci-build-linux:
    #!/usr/bin/env nu
    (docker run
        --rm
        --volume $"(pwd):/work"
        --workdir /work
        127.0.0.1:5000/woodpecker-job:v0.0.13
        ^sh -c ". /entrypoint.sh && nix develop --command just build-linux")

ci-build-macos:
    #!/usr/bin/env nu
    (docker run
        --rm
        --volume $"(pwd):/work"
        --workdir /work
        ghcr.io/rust-cross/cargo-zigbuild
        ^sh -c "cargo install just && just build-macos")

ci-build-windows:
    #!/usr/bin/env nu
    (docker run
        --rm
        --volume $"(pwd):/work"
        --workdir /work
        ghcr.io/rust-cross/cargo-xwin
        ^sh -c "cargo install just && just build-windows")

ci-check:
    #!/usr/bin/env nu
    (docker run
        --rm
        --volume $"(pwd):/work"
        --workdir /work
        127.0.0.1:5000/woodpecker-job:v0.0.13
        ^sh -c ". /entrypoint.sh && nix develop --command just check")

# ---------------- INSTALL

install:
    cargo install --path . --force

# ---------------- RELEASE

release-tag:
    #!/usr/bin/env nu
    let version = (open Cargo.toml | get package.version)
    let tag = $"v($version)"
    print $"Tagging ($tag)..."
    ^git tag --sign $tag --message $tag
    print $"Pushing ($tag)..."
    ^git push origin $tag
    print $"(ansi green)OK: Pushed signed tag ($tag)(ansi reset)"

release-upload TAG:
    bun scripts/release-upload.ts {{ TAG }}

release-delete TAG:
    bun scripts/release-delete.ts {{ TAG }}

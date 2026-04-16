default:
    @just --list

run *ARGS='':
    just rust-run {{ARGS}}

watch:
    just rust-watch

fix:
    #!/usr/bin/env nu
    if (which sandbox | is-empty) { exit 0 }
    ^just nix-fix
    ^just rust-fix

shell:
    nix develop --command nu

rust-build:
    sandbox cargo build --all-targets

rust-cargo-check:
    sandbox cargo check --all-targets

rust-check:
    just rust-cargo-check
    just rust-fmt-check
    just rust-clippy-check
    just rust-test

rust-clippy-check *ARGS='':
    sandbox cargo clippy --all-targets {{ARGS}}

rust-clippy-fix *ARGS='':
    sandbox cargo clippy \
        --fix \
        --all-targets \
        --allow-dirty \
        {{ARGS}}

rust-fix:
    just rust-fmt-fix
    just rust-clippy-fix

rust-fmt-check:
    sandbox cargo fmt --all -- --check

rust-fmt-fix:
    sandbox cargo fmt --all

rust-run *ARGS='':
    sandbox cargo run -- {{ARGS}}

rust-watch:
    sandbox cargo watch --exec run

rust-outdated:
    sandbox cargo outdated

rust-test *FILTER='':
    sandbox cargo nextest run --no-tests=pass {{FILTER}}

rust-upgrade:
    sandbox cargo upgrade --incompatible
    sandbox cargo update

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

nix-check:
    fd \
        --extension nix \
        --exec nixfmt --check
    statix check .
    deadnix --fail .

nix-fix:
    fd \
        --extension nix \
        --exec nixfmt
    statix fix .
    deadnix --edit .

test:
    just rust-test

woodpecker-check:
    woodpecker-cli lint \
        --strict \
        .woodpecker/

check:
    #!/usr/bin/env nu
    if (which sandbox | is-empty) { exit 0 }
    ^just nix-check
    ^just nu-check
    ^just rust-check
    ^just woodpecker-check

sandbox-test:
    sandbox bun scripts/sandbox-test.ts

build:
    sandbox cargo build --release

# ---------------- RELEASE BUILDS

release-build OS ARCH:
    docker run --rm \
        --volume {{ justfile_directory() }}:/work \
        --workdir /work \
        --env REVISION={{ env("REVISION") }} \
        ghcr.io/rust-cross/cargo-zigbuild:0.21.4 \
        scripts/release-build.sh {{ OS }} {{ ARCH }}

release-build-linux:
    just release-build unknown-linux-musl x86_64
    just release-build unknown-linux-musl aarch64

release-build-macos:
    just release-build apple-darwin x86_64
    just release-build apple-darwin aarch64

release-build-windows:
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
        ^sh -c (
            ". /entrypoint.sh && nix develop --command just release-build-linux"
        ))

ci-build-macos:
    #!/usr/bin/env nu
    (docker run
        --rm
        --volume $"(pwd):/work"
        --workdir /work
        ghcr.io/rust-cross/cargo-zigbuild
        ^sh -c "cargo install just && just release-build-macos")

ci-build-windows:
    #!/usr/bin/env nu
    (docker run
        --rm
        --volume $"(pwd):/work"
        --workdir /work
        ghcr.io/rust-cross/cargo-xwin
        ^sh -c "cargo install just && just release-build-windows")

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
    sandbox cargo install --path . --force

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
    sandbox bun scripts/release-upload.ts {{ TAG }}

release-delete TAG:
    sandbox bun scripts/release-delete.ts {{ TAG }}

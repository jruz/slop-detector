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

setup:
    sandbox cargo fetch --locked

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

[working-directory: 'scripts']
nu-check:
    #!/usr/bin/env nu
    (^fd
        --absolute-path
        --extension nu
        --exec nu --commands 'nu-check --debug {}')

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

# ---------------- INSTALL

install:
    sandbox cargo install \
        --path . \
        --locked \
        --force \
        --root target/dist
    install \
        --directory \
        --mode=755 \
        "$HOME/.cargo/bin"
    install \
        --mode=755 \
        target/dist/bin/* \
        "$HOME/.cargo/bin"

# ---------------- RELEASE

release-build TARGET:
    nu scripts/release-build.nu {{ TARGET }}

release-build-all:
    nu scripts/release-build.nu all

release-bump VERSION:
    nu scripts/release-bump.nu {{ VERSION }}

release-upload TAG:
    gh release view {{ TAG }} \
        || gh release create {{ TAG }} \
            --title {{ TAG }} \
            --verify-tag
    gh release upload {{ TAG }} \
        {{ env("CI_REPO_NAME") }}-* \
        --clobber

release-delete TAG:
    gh release delete {{ TAG }} \
        --cleanup-tag \
        --yes

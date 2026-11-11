# Slop Detector

An opinionated linter for catching slop and enforcing my coding standards. I use it in my own projects through an agent’s stop hook; its rules reflect my preferences, not a universal coding standard.

It checks the code, not whether a human or an AI wrote it.

Supported languages:

- Compose
- CSS
- Elixir
- Gleam
- JS/TS
- Just
- Nix
- Nomad
- Nushell
- Playwright
- pnpm
- Rust
- Swift
- OpenTofu

## Installation

```bash
cargo install --git https://github.com/jruz/slop-detector
```

## Usage

```bash
slop-detector          # Check all languages
slop-detector compose
slop-detector css
slop-detector elixir
slop-detector gleam
slop-detector js
slop-detector just
slop-detector nix
slop-detector nomad
slop-detector nu
slop-detector playwright
slop-detector pnpm
slop-detector rust
slop-detector swift
slop-detector tofu
```

## Configuration

Create `slop-detector.toml` to ignore violations:

```toml
[ignore]
rules = ["no-interface"]           # Ignore rules globally
files = ["legacy/"]                # Ignore files by pattern

[ignore.by-rule]
no-throw = ["src/cli.ts"]          # Ignore specific rule in specific files
```

Generate config from current violations:

```bash
slop-detector --create-config-for-violations
```

## Rules

### Compose
- `no-env-passthrough` - Don't redeclare vars `--env-file` passes them automatically
- `no-latest-tag` - Pin to an exact version tag

### CSS
- `no-stylelint-disable` - Fix the violation instead of disabling the rule

### Elixir
- `no-inline-style` - Use CSS classes instead of inline styles
- `no-nested-call-args` - Bind inner call results to locals before passing them
- `pinned-dep-versions` - Pin dependency versions in mix.exs
- `pinned-elixir-version` - Pin to exact matching the flake

### Gleam
- `exact-versions` - Pin dependency versions in gleam.toml
- `no-discarded-error-fields` - Log or propagate error details instead of discarding
- `no-discarded-ok` - Use the Ok value or use `let assert Ok(Nil)` for unit
- `no-error-passthrough` - Use `use value <- result.try(expr)` to avoid re-wrapping
- `no-ignored-error` - Handle or propagate errors instead of ignoring
- `no-let-assert-outside-main` - Use pattern matching or result.try instead
- `no-nested-call-args` - Bind inner call results to locals before passing them
- `no-map-error-discard` - Use case with explicit Nil pattern instead of discarding the error
- `no-nested-if` - Use multi-pattern case or bool.guard to flatten
- `no-nested-option` - Use `use value <- option.then(expr)` to flatten
- `no-nested-result` - Use `use value <- result.try(expr)` to flatten
- `no-unwrap` - Use pattern matching or let assert instead

### JS/TS
- `exact-versions` - Pin dependency versions in package.json
- `no-array-as-const` - Declare the type instead of using `[...] as const`
- `no-console` - Use log.ts, tracing.ts or output.ts
- `no-for-loop` - Use map, filter, or reduce instead of for loops
- `no-nested-call-args` - Bind inner call results to consts before passing them
- `no-throwing-fn-without-try` - Wrap throwing functions in try/catch
- `no-interface` - Use type instead of interface
- `no-defaults` - Avoid `x ?? fallback`, `x || fallback` with literal defaults
- `no-eslint-disable` - Fix the violation instead of disabling the rule
- `no-satisfies` - Declare the type instead
- `no-silent-err-return` - Handle the error in the UI or at least log it
- `no-stderr-write` - Avoid writing to stderr
- `no-stdout-write` - Use log.ts

### Just

Checks `justfile`, `*.just`, and `*.justfile` recursively, including dot-prefixed folders. Git-ignored paths are skipped.

- `no-bash-shebang` - Use nushell instead of bash/sh shebangs
- `no-bunx` - Use `bun run` instead of `bunx`
- `no-external-without-caret` - Use `^command` to mark external commands in nu recipes
- `no-long-lines` - Max 80 chars per line; break with `\` (bash) or `(\n` + indent (nu)
- `no-lowercase-args` - Use UPPERCASE for recipe arguments
- `no-multi-flags` - Put valued flags (`--flag=X`) on their own line; max 2 boolean flags per line
- `no-recipe-deps` - Call dependencies explicitly with `just <recipe>` in the body

### Nix
- `no-unstable-nixpkgs` - Pin nixpkgs to a release branch like nixos-25.11; unstable is fine as a separate input
- `no-manual-for-all-systems` - Use flake-utils instead of manually defining forAllSystems with genAttrs

### Nomad
- `env-var-mismatch` - Match environment keys to their variable names
- `env-var-naming` - Use UPPER_SNAKE_CASE variable references in environment blocks
- `no-hardcoded-env` - Use variable references instead of literal environment values
- `pinned-image-versions` - Require `x.x.x` or `vx.x.x` image tags with optional suffixes; variable-based images are exempt

### Nushell
- `no-catch-exit` - Don't catch an error just to print it and `exit` or swallow it
- `no-complete-rethrow` - Don't use `| complete` only to check `exit_code != 0` and `error make`; failing externals already raise
- `no-config-file` - Don't pass `--no-config-file` to `nu`; non-interactive Nushell skips startup configuration by default
- `no-error-make` - Don't use `error make`; use `log error` with an actionable message, followed by `exit 1`
- `no-external-in-each` - Don't run externals inside `each`, `par-each` or `items`, directly or via same-file functions; use `for`
- `no-ignore-errors` - Don't use `do --ignore-errors`; it swallows failures
- `no-print-stderr` - Don't use `print --stderr`; use `log error`, `log warning` or `log info`; allowed in `log.nu` and `sandbox.nu`
- `no-reduce` - Don't use `reduce`; use a `for` loop with a `mut` variable

### pnpm

Checks `pnpm-workspace.yaml` files recursively.

- `minimum-release-age` - Explicitly set `minimumReleaseAge` greater than zero
- `no-minimum-release-age-exclude` - Don't bypass the release-age policy with exclusions

### Playwright
- `no-nested-call-args` - Bind inner call results to consts before passing them
- `no-page` - Use request for server tests, use page in client tests
- `no-page-request` - Use page for client tests, use request in server tests
- `no-request` - Use page for client tests, use request in server tests

### Rust
- `no-allow-attr` - Refactor instead of suppressing lints
- `no-catch-unwind` - Do not swallow panics
- `no-cfg-attr-suppress` - No cfg_attr with allow/expect
- `no-discard-result` - Handle Result instead of `let _ =`
- `no-discard-try-value` - Check the success value instead of `let _var = expr?`
- `no-drop-result` - Handle Result instead of dropping
- `no-direct-env-access` - Access env only through build.rs, config.rs, or env.rs
- `no-expect-attr` - Refactor instead of suppressing lints
- `no-expect-outside-tests` - Use ? or match, expect only in tests
- `no-if-let-ok-without-else` - Add else to handle error case
- `no-ignore-test` - Don't ignore tests, fix the underlying issue
- `no-impl` - Use free functions
- `no-inline-style` - Use CSS classes instead of inline styles
- `no-is-ok-is-err` - Use the value instead of just checking
- `no-let-ok-else-continue` - Don't silently skip errors in loops
- `no-map-discard` - Don't hide values with .map(|_| ...)
- `no-map-err-discard` - Preserve original error in map_err
- `no-map-or-else` - Use match or .ok_or_else() instead
- `no-mem-forget` - Do not leak resources
- `no-nested-call-args` - Bind inner call results to locals before passing them
- `no-ok-and-propagate` - Use ? on Result directly, not .ok()?
- `no-ok-method` - Handle error explicitly, don't discard with .ok()
- `no-postgres-query-outside-models` - Keep tokio-postgres / deadpool-postgres query calls (`simple_query`, `query`, `query_one`, `query_opt`, `query_raw`) inside `models/`
- `no-print-outside-main` - Keep print statements in main()
- `no-pub-crate-fn` - Use pub instead of pub(crate) in single-crate projects
- `no-return-ok` - Don't hide errors with return Ok(())
- `no-spawn` - Use .output() or .status() instead of .spawn()
- `no-sqlx-query-outside-models` - Keep `sqlx::query*` usage inside `models/`
- `no-sqlx-query-without-skip-pool` - Add `skip(pool)` when an instrumented query function takes `pool`
- `no-sqlx-query-without-tracing-instrument` - Add `#[tracing::instrument]` to query functions
- `no-stderr-null` - Don't silence stderr with Stdio::null()
- `no-swallowed-err` - Don't ignore Err in match arms
- `no-trait-impl` - Avoid trait implementations
- `no-unit-error` - Use meaningful error types, not ()
- `no-unwrap-err` - Handle Ok case explicitly
- `no-unwrap-or` - Avoid silent defaults
- `no-unwrap-or-default` - Avoid silent defaults
- `no-unwrap-or-else` - Avoid silent defaults
- `no-unwrap-unchecked` - Use safe unwrap methods
- `no-while-let-ok` - Handle errors explicitly, don't silently stop
- `pinned-versions` - Pin dependency versions in Cargo.toml
- `thiserror-unused-fields` - Use all error fields in #[error("...")]

### Swift
- `no-guard-return` - Throw or log before returning instead of silently discarding
- `no-nested-call-args` - Bind inner call results to lets before passing them
- `no-silent-catch` - Handle the error instead of silently discarding it
- `no-silent-if-let` - Throw or log before returning instead of silently discarding
- `no-try-question-mark` - Use do/catch to handle errors instead of silencing them

### OpenTofu
- `banned-data-source` - No external data source
- `banned-provider` - No external provider
- `banned-provisioner` - No local-exec/remote-exec
- `banned-resource` - No terraform_data

## AI disclaimer

This project was built with AI assistance (Claude Code, Codex, GLM). These tools assist under my direction; this project is not vibe coded. All code follows my own strict quality standards, enforced through hooks (including this linter), skills, and my manual review. The architecture and design are my own.

## License

Copyright jruz, 2026. Licensed under the EUPL.

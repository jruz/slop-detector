# Slop Detector

Linter for detecting AI generated slop in Gleam, JS/TS, Rust, and OpenTofu code.

## Installation

```bash
cargo install --git https://codeberg.org/jruz/slop-detector
```

## Usage

```bash
slop-detector          # Check all languages
slop-detector gleam    # Check Gleam only
slop-detector js       # Check JS/TS only
slop-detector rust     # Check Rust only
slop-detector tofu     # Check OpenTofu only
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

### Gleam
- `exact-versions` - Pin dependency versions in gleam.toml
- `no-discarded-error-fields` - Log or propagate error details instead of discarding
- `no-discarded-ok` - Use the Ok value or use `let assert Ok(Nil)` for unit
- `no-error-passthrough` - Use `use value <- result.try(expr)` to avoid re-wrapping
- `no-ignored-error` - Handle or propagate errors instead of ignoring
- `no-let-assert-outside-main` - Use pattern matching or result.try instead
- `no-nested-if` - Use multi-pattern case or bool.guard to flatten
- `no-nested-option` - Use `use value <- option.then(expr)` to flatten
- `no-nested-result` - Use `use value <- result.try(expr)` to flatten
- `no-unwrap` - Use pattern matching or let assert instead

### JS/TS
- `exact-versions` - Pin dependency versions in package.json
- `no-console` - Keep console calls in main() only
- `no-early-return` - Return Result/Option instead
- `no-interface` - Use type instead of interface
- `no-scripts-section` - No scripts in package.json
- `no-silent-defaults` - Avoid `x || []`, `x ?? {}` patterns
- `no-stderr-write` - Avoid writing to stderr
- `no-throw` - Return Result.err() instead

### Rust
- `no-allow-attr` - Refactor instead of suppressing lints
- `no-catch-unwind` - Do not swallow panics
- `no-cfg-attr-suppress` - No cfg_attr with allow/expect
- `no-discard-result` - Handle Result instead of `let _ =`
- `no-drop-result` - Handle Result instead of dropping
- `no-expect-attr` - Refactor instead of suppressing lints
- `no-expect-outside-tests` - Use ? or match, expect only in tests
- `no-if-let-ok-without-else` - Add else to handle error case
- `no-impl` - Use free functions
- `no-is-ok-is-err` - Use the value instead of just checking
- `no-let-ok-else-continue` - Don't silently skip errors in loops
- `no-map-discard` - Don't hide values with .map(|_| ...)
- `no-map-err-discard` - Preserve original error in map_err
- `no-map-or-else` - Use match or .ok_or_else() instead
- `no-mem-forget` - Do not leak resources
- `no-ok-and-propagate` - Use ? on Result directly, not .ok()?
- `no-ok-method` - Handle error explicitly, don't discard with .ok()
- `no-print-outside-main` - Keep print statements in main()
- `no-return-ok` - Don't hide errors with return Ok(())
- `no-spawn` - Use .output() or .status() instead of .spawn()
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
- `rust-version-sync` - Keep Rust version in sync across config files
- `thiserror-unused-fields` - Use all error fields in #[error("...")]

### OpenTofu
- `banned-data-source` - No external data source
- `banned-provider` - No external provider
- `banned-provisioner` - No local-exec/remote-exec
- `banned-resource` - No terraform_data

## License

MIT

# Slop Detector

Linter for detecting AI generated slop in:

- CSS
- Elixir
- Gleam
- JS/TS
- Just
- Playwright
- Rust
- Swift
- OpenTofu

## Installation

```bash
cargo install --git https://codeberg.org/jruz/slop-detector
```

## Usage

```bash
slop-detector          # Check all languages
slop-detector css
slop-detector elixir
slop-detector gleam
slop-detector js
slop-detector just
slop-detector playwright
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

### CSS
- `no-stylelint-disable` - Fix the violation instead of disabling the rule

### Elixir
- `no-inline-style` - Use CSS classes instead of inline styles
- `pinned-dep-versions` - Pin dependency versions in mix.exs
- `pinned-elixir-version` - Pin to exact matching the flake

### Gleam
- `exact-versions` - Pin dependency versions in gleam.toml
- `no-discarded-error-fields` - Log or propagate error details instead of discarding
- `no-discarded-ok` - Use the Ok value or use `let assert Ok(Nil)` for unit
- `no-error-passthrough` - Use `use value <- result.try(expr)` to avoid re-wrapping
- `no-ignored-error` - Handle or propagate errors instead of ignoring
- `no-let-assert-outside-main` - Use pattern matching or result.try instead
- `no-map-error-discard` - Use case with explicit Nil pattern instead of discarding the error
- `no-nested-if` - Use multi-pattern case or bool.guard to flatten
- `no-nested-option` - Use `use value <- option.then(expr)` to flatten
- `no-nested-result` - Use `use value <- result.try(expr)` to flatten
- `no-unwrap` - Use pattern matching or let assert instead

### JS/TS
- `exact-versions` - Pin dependency versions in package.json
- `no-array-as-const` - Declare the type instead of using `[...] as const`
- `no-console` - Use log.ts or tracing.ts
- `no-for-loop` - Use map, filter, or reduce instead of for loops
- `no-throwing-fn-without-try` - Wrap throwing functions in try/catch
- `no-interface` - Use type instead of interface
- `no-scripts-section` - No scripts in package.json
- `no-defaults` - Avoid `x ?? fallback`, `x || fallback` with literal defaults
- `no-eslint-disable` - Fix the violation instead of disabling the rule
- `no-satisfies` - Declare the type instead
- `no-stderr-write` - Avoid writing to stderr
- `no-stdout-write` - Use log.ts

### Just
- `no-bash-shebang` - Use nushell instead of bash/sh shebangs
- `no-bunx` - Use `bun run` instead of `bunx`
- `no-external-without-caret` - Use `^command` to mark external commands in nu recipes
- `no-long-lines` - Max 80 chars per line; break with `\` (bash) or `(\n` + indent (nu)
- `no-lowercase-args` - Use UPPERCASE for recipe arguments
- `no-recipe-deps` - Call dependencies explicitly with `just <recipe>` in the body

### Playwright
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
- `thiserror-unused-fields` - Use all error fields in #[error("...")]

### Swift
- `no-guard-return` - Throw or log before returning instead of silently discarding
- `no-silent-catch` - Handle the error instead of silently discarding it
- `no-silent-if-let` - Throw or log before returning instead of silently discarding
- `no-try-question-mark` - Use do/catch to handle errors instead of silencing them

### OpenTofu
- `banned-data-source` - No external data source
- `banned-provider` - No external provider
- `banned-provisioner` - No local-exec/remote-exec
- `banned-resource` - No terraform_data

## License

MIT

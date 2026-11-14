# Slop Detector

Linter for detecting AI generated slop in Compose, CSS, Elixir, Gleam, JS/TS, Just, Nix, Playwright, Rust, Swift, and OpenTofu code.

## Environment

Claude runs inside a Nix shell. All non-cargo dependencies must be added to `flake.nix`.

## Architecture

```
src/
├── main.rs          # CLI entry, orchestrates checks
├── types.rs         # Violation, CheckResult, AppError, Config
├── config.rs        # Load slop-detector.toml, filter violations
├── files.rs         # Glob-based file discovery
├── output.rs        # Format violations, generate config
└── langs/
    ├── mod.rs       # Re-exports lang modules
    ├── compose.rs   # Compose rules (text matching)
    ├── css.rs       # CSS rules (lightningcss parser)
    ├── elixir.rs    # Elixir rules (text matching)
    ├── gleam.rs     # Gleam rules (tree-sitter + toml parser)
    ├── js.rs        # JS/TS rules (oxc parser)
    ├── just.rs      # Just rules (tree-sitter parser)
    ├── nix.rs       # Nix rules (rnix parser)
    ├── nu.rs        # Nushell rules (nu-parser)
    ├── rust.rs      # Rust rules (ra_ap_syntax parser)
    ├── swift.rs     # Swift rules (tree-sitter parser)
    └── tofu.rs      # OpenTofu rules (hcl-rs parser)
```

Each lang module follows: `find() -> parse() -> run_rules() -> CheckResult`

## Adding Languages

Choose the analysis strategy in this order:

1. **Real parser** - the language's own parser, or a maintained Rust parser (e.g. `oxc`, `rnix`, `nu-parser`)
2. **Tree-sitter** - when no real parser is available as a crate
3. **Line scanning** - last resort, only with explicit user approval

Rules:

- never fall back to a lower option without ruling out the one above it
- never use line scanning or text matching without asking the user first
- parse failures are reported as errors, never skipped

## Updating Nushell Support

- the `nu-*` crates in `Cargo.toml` must match the nushell version the flake provides (`nu --version`)
- every `nu-*` crate except `nu-parser` uses `default-features = false`
- bump all `nu-*` crates together
- newer parsers reject syntax older shells accept (e.g. `def run` on 0.116)
- after a bump, run `slop-detector nu` against real nushell files for parse errors

## Commands

- `just check` - Run all checks (cargo check, lint, test)
- `just rust-fix` - Auto-fix formatting and clippy issues
- `just rust-outdated` - Check for outdated dependencies
- `just rust-upgrade` - Upgrade dependencies

## Adding Rules

### 1. Create Fixtures

Create two fixture files in `tests/fixtures/<lang>/`:

**Violation fixture** - code that should trigger the rule:

```
tests/fixtures/<lang>/rule-name.<ext>
```

**Valid fixture** - code that should NOT trigger the rule:

```
tests/fixtures/<lang>/valid-rule-name.<ext>
```

### 2. Add Rule to RULES Array

Rules are data. Add to the RULES array with a check function:

```rust
Rule {
    name: "rule-name",
    hint: Some("how to fix"),
    check: check_rule_name,
}
```

### 3. Implement Check Function

```rust
fn check_rule_name(source: &str, ast: &Ast) -> Vec<String> {
    // return vec![] for pass, vec!["line N: issue"] for fail
}
```

### 4. Add Tests

Add two tests following the naming convention:

```rust
#[test]
fn rule_name_detects_violation() {
    let (source, ast) = parse_fixture("rule-name");
    let errors = check_rule_name(&source, &ast);
    assert_eq!(errors.len(), 1);  // exact count
}

#[test]
fn rule_name_allows_valid() {
    let (source, ast) = parse_fixture("valid-rule-name");
    let errors = check_rule_name(&source, &ast);
    assert!(errors.is_empty());
}
```

### 5. Update README

Add the new rule to the appropriate language section in `README.md`.

### 6. Verify

Run `just check` to ensure all tests pass.

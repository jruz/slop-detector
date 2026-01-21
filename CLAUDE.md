# Slop Detector

Linter for detecting anti-patterns in JS/TS, Rust, and OpenTofu code.

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
    ├── js.rs        # JS/TS rules (oxc parser)
    ├── rust.rs      # Rust rules (ra_ap_syntax parser)
    └── tofu.rs      # OpenTofu rules (hcl-rs parser)
```

Each lang module follows: `find() -> parse() -> run_rules() -> CheckResult`

## Commands

- `just check` - Run all checks (cargo check, lint, test)
- `just rust-fix` - Auto-fix formatting and clippy issues
- `just rust-outdated` - Check for outdated dependencies
- `just rust-upgrade` - Upgrade dependencies

## Adding Rules

Rules are data. Add to RULES array with a check function:

```rust
Rule {
    name: "rule-name",
    hint: Some("how to fix"),
    check: check_rule_name,
}

fn check_rule_name(source: &str, ast: &Ast) -> Vec<String> {
    // return vec![] for pass, vec!["line N: issue"] for fail
}
```

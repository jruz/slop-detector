# Slop Detector

Linter for detecting anti-patterns in Gleam, JS/TS, Rust, and OpenTofu code.

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

### JS/TS
- `no-interface` - Use type instead of interface
- `no-silent-defaults` - Avoid `x || []`, `x ?? {}` patterns
- `no-console` - Keep console calls in main() only
- `no-throw` - Return Result.err() instead
- `no-early-return` - Return Result/Option instead
- `no-scripts-section` - No scripts in package.json
- `exact-versions` - Pin dependency versions

### Rust
- `no-impl` - Use free functions
- `no-trait-impl` - Avoid trait implementations
- `no-print-outside-main` - Keep print statements in main()

### OpenTofu
- `banned-resource` - No terraform_data
- `banned-data-source` - No external data source
- `banned-provisioner` - No local-exec/remote-exec
- `banned-provider` - No external provider

## License

MIT

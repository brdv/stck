# Contributing

Use standard Cargo tooling for formatting, linting, tests, and build checks.

## Local Validation

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --all-features
```

## Scope

- Keep changes minimal and focused.
- Avoid unrelated refactors in the same patch.
- Add or update tests for behavior changes.

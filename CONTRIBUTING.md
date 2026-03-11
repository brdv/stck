# Contributing

Use standard Cargo tooling for formatting, linting, tests, and build checks.

## Local Validation

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --all-features
```

The crate currently declares Rust `1.74` as its minimum supported toolchain in `Cargo.toml`.

## Scope

- Keep changes minimal and focused.
- Avoid unrelated refactors in the same patch.
- Add or update tests for behavior changes.

## Documentation

- Keep Rustdoc accurate as behavior changes.
- Use `//!` for crate/module docs and `///` for public items whose contracts matter.
- Document what an item does, important invariants or assumptions, and notable error behavior when that is not obvious from the code alone.
- Use inline `//` comments sparingly and only for non-obvious logic or workflow invariants.
- Prefer reducing visibility over documenting accidental public API surface.

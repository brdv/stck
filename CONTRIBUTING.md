# Contributing

Use standard Cargo tooling for formatting, linting, tests, and build checks.

## Local Validation

```bash
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo build --locked --all-features
```

The project supports the current stable Rust release and the two preceding
releases (N-2). The minimum is declared in `Cargo.toml`, and CI validates both
that version and the current stable toolchain.

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

## Behavioral Contracts

The tracked [sync recovery contract](./docs/sync-recovery.md) is normative.
Changes to sync state or recovery must preserve its fail-closed guarantees or
update the contract and corresponding stubbed and real-Git tests in the same
pull request.

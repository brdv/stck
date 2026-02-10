# AGENTS.md

## Project Overview

- `stck` is a Rust CLI for stacked GitHub PR workflows.
- Keep all changes local to this repository.
- Prefer minimal, targeted diffs; avoid drive-by refactors and unrelated cleanup.
- If requirements are ambiguous (e.g., CLI UX/output, defaults, error messages, flag naming/behavior), ask the user before making changes.

## Compatibility & Stability

- Maintain backwards compatibility for the public CLI interface unless the user explicitly requests a breaking change.
- Keep CLI behavior predictable:
  - Stable output where possible (especially for machine-readable/greppable output).
  - Write errors to stderr and use meaningful exit codes.
- Target platforms: do not introduce platform-specific behavior unless required and justified.

## Tooling

- Language: Rust.
- Build, lint, format, and test using standard Cargo tooling.
- Prefer Rust standard library and existing project code over adding dependencies.
- If a dependency is not clearly necessary, implement functionality directly.
- If adding a dependency is justified, keep it small, well-maintained, and scoped to the need.

## Standard Commands

- Format: `cargo fmt --all`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Test: `cargo test --all-features`
- Build: `cargo build --all-features`
- Release build: `cargo build --release --all-features`

## Testing Expectations

- This project is critical to developer workflows; test coverage quality is a priority.
- Add or update tests for every functional behavior change.
- Prefer:
  - Unit tests for parsing/logic.
  - Integration tests for CLI behavior (flags, output, exit codes).
- Avoid flaky tests (timing-sensitive, network-dependent, environment-specific).
- Run relevant Cargo tests when changes are applicable and testable.

## Safety Rules

- Never run destructive Git commands (for example `git reset --hard`).
- Never read, add, edit, or delete files outside this repository.
- Network access is limited to read-only operations (fetching public docs or references) when needed.
  - Do not introduce runtime network requirements unless explicitly requested.

## Git/PR Policy

- The agent does local working-tree changes only.
- The agent must not create branches, commit, push, or open/manage PRs.
- All branch/commit/PR operations are performed manually by the user.
- When asked for PR descriptions, use `./github/pull_request_template.md`

## Collaboration Style

- Use balanced communication: short progress updates with concise rationale.
- Keep explanations pragmatic and action-oriented.
- When making a non-trivial change, summarize:
  - what changed,
  - why it changed,
  - how it was tested.

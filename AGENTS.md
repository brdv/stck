# AGENTS.md

## Overview

`stck` is a Rust CLI that automates stacked GitHub PR workflows — creating branches, rebasing stacks, pushing with `--force-with-lease`, and updating PR base targets. Stays close to native `git`/`gh` behavior.

## Commands

```bash
cargo fmt --all                                          # format
cargo clippy --all-targets --all-features -- -D warnings # lint
cargo test --all-features                                # test all
cargo test <test_name>                                   # run single test
cargo build --all-features                               # build
cargo run -- <command>                                   # run locally (always prefer over global install)
```

## Architecture

All source lives in `src/` across 7 modules:

- **main.rs** — entry point; delegates to `cli::run()`
- **cli.rs** — `clap` derive parsing and command dispatch (`new`, `submit`, `status`, `sync`, `push`)
- **github.rs** — GitHub integration via `gh` subprocess; discovers linear PR stacks, creates/updates PRs
- **gitops.rs** — low-level git operations (fetch, rebase, push, ref resolution) via `git` subprocess
- **stack.rs** — stack data model; builds linear stack graphs from PR metadata; status reporting
- **env.rs** — preflight validation (git, gh, auth, clean working tree)
- **sync_state.rs** — persists operation state as JSON to `.git/stck/` for `--continue`/`--reset` resumable workflows

### Key design decisions

- Subprocess calls to `git` and `gh` — no library bindings, keeps deps minimal
- Linear stack assumption — fails fast on non-linear PR graphs
- State persisted to `.git/stck/` for resumable `sync`/`push`
- Always `git push --force-with-lease`, never bare `--force`
- Errors to stderr with meaningful exit codes; output is grep-friendly

### Dependencies

Minimal by design: `clap`, `serde`, `serde_json`.
Dev: `assert_cmd`, `predicates`, `tempfile`.

## Safety

| Forbidden                                           | Do instead                                                               |
| --------------------------------------------------- | ------------------------------------------------------------------------ |
| Destructive git commands (`git reset --hard`, etc.) | Use safe alternatives (`git rebase`, `git stash`) or ask user for input. |
| Read/add/edit/delete files outside this repo        | Keep all changes local to the repository                                 |
| Create branches, commit, push, or open PRs          | Leave all git/PR operations to the user, unless explicitly asked         |
| Introduce runtime network requirements              | Keep network access read-only and optional                               |

## Coding Standards

- Prefer Rust stdlib and existing project code over new dependencies.
- If a dependency is justified, keep it small, well-maintained, and scoped to the need.
- Prefer minimal, targeted diffs — avoid drive-by refactors and unrelated cleanup.
- Maintain backwards compatibility for the public CLI interface unless explicitly asked to break it.
- Keep CLI output stable and grep-friendly; write errors to stderr with meaningful exit codes.
- If requirements are ambiguous (CLI UX, output format, defaults, flag naming), ask the user before guessing.

## Documentation

- Follow standard Rustdoc expectations for crate, module, and public API documentation.
- Use `//!` for crate/module docs and `///` for public structs, enums, and functions whose contracts matter.
- Document what the item does, key invariants/assumptions, important error behavior, and usage context when that is not obvious from the type/signature.
- Add inline `//` comments sparingly and only for non-obvious logic, workflow invariants, or recovery behavior; do not add comments that merely restate the code.
- Prefer reducing visibility over documenting accidental public surface area.
- Keep documentation accurate as behavior changes, and treat missing-docs lint compliance as part of done-ness.

## Testing

- Add or update tests for every functional behavior change.
- Unit tests for parsing and logic; integration tests for CLI behavior (flags, output, exit codes).
- For `sync`/`push` changes: cover both fresh-run and stateful resume/reset paths.
- Avoid flaky tests (timing-sensitive, network-dependent, environment-specific).
- Run relevant tests when changes are applicable and testable.

## Git Workflow

- Before rebasing onto main: `git fetch`, then `git rebase origin/main`.
- For PR descriptions, use `./github/pull_request_template.md`.
- For stacked PRs, keep parent/child base relationships explicit in summaries and descriptions.

## Collaboration

- Use short progress updates with concise rationale.
- When making a non-trivial change, summarize: what changed, why, and how it was tested.
- Prioritize correctness and reliability before output/UX polish unless explicitly reprioritized.
- Keep review findings traceable — each finding maps to a roadmap item or an explicit deferral.

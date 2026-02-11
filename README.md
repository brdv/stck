# stck

`stck` is a Rust CLI for working with stacked GitHub pull requests.

## Status

This project is in active development.

- APIs and command behavior may change.
- Workflows are being implemented incrementally.
- Expect rough edges while core functionality is stabilized.

## Goal

`stck` aims to make stacked PR workflows predictable and low-friction by providing a small set of commands to:

- create the next branch in a stack,
- inspect stack and PR state,
- restack/rebase locally after upstream changes,
- push rewritten branches and update PR base relationships.

The long-form product and implementation plan lives in [`PLAN.md`](./PLAN.md).

## Why stck

Stacked PRs improve review quality and throughput, but day-to-day maintenance can be tedious and error-prone. `stck` focuses on automating the repetitive mechanics while staying close to native `git` and `gh` behavior.

## Basic Usage

Current command surface (in progress):

```bash
stck new <branch>
stck status
stck sync
stck push
```

During development, run via Cargo:

```bash
cargo run -- <command>
# example
cargo run -- status
```

## Command Intent

- `stck new <branch>`: create and stack a new branch/PR on top of the current branch.
- `stck status`: show detected stack order and PR state.
- `stck sync`: restack/rebase the local stack based on current PR graph state.
- `stck push`: push rewritten branches and reconcile PR base targeting.

## Development

For contributors working locally:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --all-features
```

## Milestones

- [x] Milestone 0: Skeleton CLI (`new/status/sync/push` command scaffolding)
- [x] Milestone 1: Environment + repository preflight checks
- [x] Milestone 2: GitHub PR discovery (single PR)
- [x] Milestone 3: Linear stack discovery (ancestors + descendants)
- [x] Milestone 4: `stck status`
- [x] Milestone 5: Rebase plan + executor (`stck sync`)
- [ ] Milestone 6: `stck push`
- [ ] Milestone 7: `stck new`
- [ ] Milestone 8: Homebrew release cycle

Project-specific implementation details, milestone plans, and design constraints are documented in [`PLAN.md`](./PLAN.md).

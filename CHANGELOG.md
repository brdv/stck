# Changelog

All notable changes to this project are documented in this file.

## [0.1.4] - 2026-03-12

### Added

- `stck new` and `stck submit` now auto-push when the branch has an upstream but has unpushed local commits, removing the manual `git push` step between stacked branch operations.

### Changed

- Sync step messaging now shows "dropping already-upstream commits" instead of the confusing "onto main (from main)" when the base branch name is unchanged after a parent PR merge.

### Fixed

- Fixed `stck sync` using stale local main ref as the rebase target after a parent PR merges on GitHub. Sync now prefers `refs/remotes/origin/<branch>` for the `--onto` target, matching the fetched remote state.
- Fixed chained sync regression where rebasing a 3+ branch stack left descendant branches with diverged history. The sync loop now tracks branches rebased in earlier steps and uses their local ref for subsequent steps.

## [0.1.3] - 2026-03-11

### Added

- Added `rust-version = "1.74"` to declare the supported Rust toolchain explicitly.
- Added `#![forbid(unsafe_code)]` to enforce the current no-unsafe safety posture.
- Added a dedicated `Cli::command().debug_assert()` smoke test for the clap definition.
- Added crate, module, and public API Rustdoc coverage across the codebase, and enabled missing-docs linting.

### Changed

- Refactored the binary to use a `src/lib.rs` entrypoint and moved command execution logic out of `src/cli.rs`.
- Split the integration test suite by command and moved shared test artifacts under per-test `TempDir`s.
- Updated README and usage docs to reflect current parent auto-detection behavior and known discovery limits.

### Fixed

- Plain `stck sync` now refuses to implicitly continue a failed sync; users must choose `--continue` or `--reset`.
- Parent-base auto-discovery now fails closed on GitHub lookup errors instead of silently defaulting to the repository default branch.
- `stck status` no longer reports `needs_push` for merged PR branches.
- Fresh `stck sync` runs now fail early when an unrelated git rebase is already in progress.
- `stck push` now skips PR retarget operations that are already satisfied.
- Resumed `stck push` runs now reconcile saved retarget state against current PR bases and clear stale push state when no work remains.

## [0.1.2] - 2026-03-11

### Added

- `stck new` and `stck submit` now auto-detect the stack parent branch when no `--base` is provided, instead of always defaulting to the repository default branch.
- `PrState` enum (`Open`, `Merged`, `Closed`) replaces stringly-typed PR state throughout the codebase.

### Changed

- `resolve_old_base_for_rebase` now uses `git merge-base` to find the true fork point, handling squash-merge and rewritten-ancestry scenarios. Falls back to direct ref resolution.
- Closed (non-merged) PRs are now excluded from stack child discovery, preventing false non-linear stack errors.
- Merged PRs are no longer flagged with `needs_sync` in status output.
- Removed `merged_at` field from `PullRequest` — `PrState` is the single source of truth for PR lifecycle state.

### Fixed

- Fixed `stck new` bootstrap PR targeting the default branch instead of the stack parent when run from a mid-stack branch.
- Fixed `stck submit` defaulting to the repository default branch instead of detecting the parent branch for stacked PRs.

## [0.1.1] - 2026-02-11

### Added

- Added `stck --version` support.
- Added `stck sync --reset` to discard saved sync state and recompute from scratch.

### Changed

- Improved `stck new` default-branch behavior:
  - supports clean stack start from default branch,
  - avoids default-branch PR bootstrap behavior.
- Improved `stck new` guidance when no branch-only commits exist by printing an explicit follow-up `gh pr create ...` command.
- Hardened `stck sync --continue` to avoid skipping failed rebase steps after abort-like states.
- `stck sync` now restores the original branch after successful completion.
- `stck push` now fetches `origin` before planning/execution.
- `stck push` now force-pushes only branches that diverged from `origin`.

### Fixed

- Distinguish `gh pr view` failure from true “no PR exists” in `stck new`.
- Added fallback for sync old-base resolution: local branch first, then `origin/<branch>`.
- Reused cached sync retarget intent in `stck push` when available.
- Improved failure diagnostics by surfacing subprocess stderr in user-facing errors.

## [0.1.0] - 2026-02-11

### Added

- Initial public release of `stck`.
- Core commands:
  - `stck new <branch>`
  - `stck status`
  - `stck sync`
  - `stck push`
- Linear stack discovery and status reporting for GitHub PR stacks.
- Local sync/rebase orchestration with resume state tracking.
- Push + PR retarget orchestration with idempotent retry behavior.
- Homebrew tap distribution scaffold with `git-stck` symlink support (`git stck ...`).

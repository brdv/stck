# Changelog

All notable changes to this project are documented in this file.

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

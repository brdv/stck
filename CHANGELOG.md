# Changelog

All notable changes to this project are documented in this file.

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

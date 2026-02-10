# stck: CLI for syncing stacked GitHub PRs

## Summary

`stck` is a minimal CLI that orchestrates stacked PR workflows on GitHub by **shelling out to `git` and `gh`**. It provides:

- `stck new <branch>`: create next stacked branch + push + create PR(s) as needed
- `stck status`: show linear stack + PR state + “needs sync/push” signals
- `stck sync`: **local-only** restack + rebase the entire stack
- `stck push`: push rewritten branches and update PR base branches on GitHub

**Additional entrypoint:** `git stck ...` works via a `git-stck` symlink installed by Homebrew.

Design goal: **v0.1 correctness of `stck sync`** (restack/rebase the whole PR chain locally every time).

---

## Motivation / Problem

Stacked PRs are great for review, but painful when upstream PRs merge or change: you need to rebase/restack a chain of dependent branches repeatedly. The core pain is **keeping a linear stack synced** without manual rebase bookkeeping and without losing PR diffs. (We require PRs, so the PR graph is the source of truth.)

---

## Goals

- Deterministic, **linear-stack** workflow: `<default> <- A <- B <- C` (no branching stacks in v0.1).
- PRs are **required** for any branch in the stack.
- `stck sync`:
  - Detect stack from GitHub PR bases (via `gh`)
  - If upstream PR(s) merged: restack locally (and record the intended PR base changes to apply later on `push`)
  - Rebase stack in order using `git rebase --onto`
  - Stop on conflicts; support resume by re-running `stck sync` or `stck sync --continue`
- `stck push`:
  - Push all involved branches to `origin` using `--force-with-lease`
  - Apply PR base retargeting (via `gh`/API) after push succeeds
  - Be idempotent on retry: recompute and apply only remaining work
- `stck new`:
  - If current branch is not pushed / PR missing: auto-push + auto-create PR
  - Create new branch stacked on current branch and create ready-for-review PR (v0.1 deterministic)
- Verbose-by-default: prints every command it runs.
- Distribution via **Homebrew**, including `git-stck` symlink for `git stck ...`.

---

## Non-goals (v0.1)

- Branching stacks / tree-shaped PR graphs (fail fast with candidates)
- Interactive selection flows (except possible later enhancements)
- Merge/land automation (leave merging to GitHub)
- Automatic enabling of `git rerere` (document + nudge only)
- Multiple remotes (assume `origin`)
- Mandatory repo-local config overrides (later)

---

## Assumptions / Preconditions (strict)

- `gh` installed and authenticated
- GitHub repo remote exists as `origin` and points to GitHub
- Default base branch is discovered from repo metadata (`origin`/GitHub default branch), not hardcoded
- Active stack branches have PRs; discovery must include `open` + `merged` states to handle merged ancestors
- Working tree is clean (no uncommitted changes)
- Stack is linear (exactly one child per base branch within the stack)

---

## User stories

1. **Create stack:** On branch A, run `stck new feature-b` → pushes A (if needed), creates PR A (if needed), creates branch B and PR B targeting A.
2. **Upstream merge:** PR A merges. On branch C, run `stck sync` → restacks B onto repo default branch, C onto B, rewrites locally. Then it tells me to run `stck push`.
3. **Conflicts:** During `stck sync`, a rebase conflicts. I resolve using Git’s normal flow, then rerun `stck sync` (or `--continue`) to proceed.
4. **Git-native invocation:** In a repo, run `git stck status` to get the same output as `stck status`.

---

## CLI surface (v0.1)

### `stck status`

- Fetches (`git fetch origin`) for accuracy (v0.1 default).
- Uses `gh` to locate the PR for current branch and walk the stack.
- Prints:
  - stack order: `<default> <- feature-a <- feature-b <- feature-c`
  - for each: PR number, state (open/merged), baseRefName/headRefName
  - indicators:
    - base mismatch (PR base not equal to expected parent in stack)
    - “needs sync” (PR parent merged / base changed required)
    - “needs push” (local differs from origin)
- Fails if stack is not linear; prints conflicting branches/PRs.

---

### `stck new <branch>`

Behavior:

1. Validate clean working tree.
2. Ensure current branch has upstream; if not, push `-u origin <current>`.
3. Ensure current branch has PR; if not, create a ready PR targeting its current base (likely repo default branch unless already stacked).
4. Create and checkout new branch `<branch>` from current `HEAD`.
5. Push new branch `-u origin <branch>`.
6. Create PR for `<branch>` with base = current branch (stacked by default), ready-for-review.

Notes:

- Descriptions/body left empty/minimal (title defaults to branch name).
- Deterministic in v0.1; draft/ready configurability later.

---

### `stck sync` (local-only)

Behavior:

1. Validate clean working tree.
2. Discover entire stack (ancestors + descendants) from current branch’s PR:
   - Walk “down” via `baseRefName` until repo default branch
   - Walk “up” by finding PR(s) whose baseRefName equals the current head branch, recursively
   - Enforce linearity: each node has at most one child; else fail.
3. Detect merged PRs in the chain:
   - If a PR is merged, its child PR(s) must be restacked to the merged PR’s base (typically repo default branch or whatever its base was).
4. Plan:
   - For each branch `b` (bottom to top), compute `old_base` (previous PR base) and `new_base` (possibly updated due to merges).
   - Record intended PR base changes (but do **not** apply to GitHub yet).
5. Execute rebase sequence:
   - For each branch `b` in order:  
     `git rebase --onto <new_base> <old_base> <b>`
   - On conflict: stop and surface git output. User resolves and continues with git; rerunning `stck sync` resumes.

At end:

- Print “Sync succeeded locally. Run `stck push` to update remotes + PR bases.”

---

### `stck push`

Behavior:

1. Recompute stack + verify local state matches expected (or reuse cached plan from last sync if present).
2. Push all stack branches to `origin` with `--force-with-lease` (v0.1).
3. Apply PR base changes on GitHub (retarget PRs) where needed.
4. Print summary.

Failure semantics:

- If any push fails, do not apply PR retargeting.
- If push succeeds but retargeting partially fails, report exact remaining retarget operations and make retry safe/idempotent.

---

### Nice-to-have (optional v0.1)

- `stck submit`: create PRs for all branches in detected stack that lack PRs (might conflict with “PR required”; could be a helper command)
- `stck doctor`: diagnose missing auth, missing PRs, non-linearity, wrong remotes, etc.

---

## Git subcommand entrypoint (`git stck`)

Git will run `git-stck` when the user invokes `git stck ...`.

v0.1 distribution installs:

- `stck` binary (canonical)
- `git-stck` symlink → `stck`

So both of these work identically:

- `stck status`
- `git stck status`

---

## Stack discovery via GitHub (`gh`)

Primary source of truth is PR metadata:

- `headRefName` = branch name
- `baseRefName` = parent branch (what the PR targets)

Algorithm (linear stack):

1. Identify current PR by head branch:  
   `gh pr view <branch> --json number,headRefName,baseRefName,state,mergedAt`
2. Walk ancestors by repeatedly looking up PR where `headRefName = baseRefName` (until repo default branch).
3. Walk descendants by searching PR whose `baseRefName = current headRefName` (must be ≤ 1 result).
4. Fail if:
   - Missing PR at any step
   - More than one child PR for a base branch

Implementation note:

- Use `--state all` when listing PRs so merged ancestors remain discoverable.
- Prefer a single graph snapshot fetch (GraphQL via `gh api graphql`) over many sequential calls to reduce race conditions.

---

## Restacking logic

When a PR is merged (say PR A on branch `feature-a`):

- Child PR B currently targets `feature-a`
- After merge, B should target **A’s base** (typically repo default branch)
- Locally, B should be rebased from old base `feature-a` onto new base `<default>`:
  - `git rebase --onto <default> feature-a feature-b`
- Do this bottom-to-top to keep the chain consistent.

We apply PR base changes during `stck push`, not `sync`.

---

## Conflict handling & resume

v0.1 relies on Git’s native behavior:

- `stck` stops on non-zero rebase exit.
- User resolves, runs `git rebase --continue` (or abort).
- Resume:
  - Re-run `stck sync` (auto-detects rebase-in-progress state) **or**
  - `stck sync --continue`

---

## Configuration and state

### Global config (primary)

- Location: `~/.config/stck/config.toml`
- Initially minimal; mostly for future flags (draft PR default, base branch, auto-push, etc.)
- Repo identification key: `origin` URL.

### Repo overrides (later)

- Not required in v0.1; add later if needed.

### Local run state (required for in-progress operations)

To support robust resume/repeatability, store last computed plan in repo-local ephemeral file (untracked), e.g. `.git/stck/last-plan.json`:

- stack order
- default branch at planning time
- PR graph snapshot hash/version
- intended PR base changes
- per-branch rebase commands that were executed
- push steps completed
- retarget steps completed

Resume/idempotency rules:

- `stck sync --continue` must read local plan state and continue from the next unfinished step.
- Plain `stck sync` may recompute only if no operation is in progress.
- `stck push` retries must skip already-completed steps and apply only remaining pushes/retargets.

---

## Implementation (Rust, thin orchestrator)

### Approach

- Rust CLI using `clap`
- Execute `git` and `gh` commands as subprocesses (no libgit2 required).
- Always print the exact command before running it (v0.1).
- Single binary `stck`; Homebrew installs `git-stck` symlink.

### Modules

- `cli/`: argument parsing + help
- `env/`: dependency checks (`git`, `gh`, auth), repo checks (`origin`, clean tree)
- `github/`: wrappers around `gh` calls and JSON parsing
- `stack/`: stack discovery + linearity enforcement + plan generation
- `gitops/`: rebase/push primitives (shell out), rebase state detection
- `ui/`: printing plan/status consistently
- `release/`: (optional) release helpers, version printing, etc.

### Testing strategy

- Unit tests: stack graph building + plan generation (pure functions).
- Integration tests:
  - Use a temporary git repo in tests.
  - Mock `gh` output (fixture JSON) to simulate PR graphs, merges, edge cases.
  - “Golden output” tests for `status` formatting.
  - Failure injection tests for partial `stck push` success (push or retarget failures) and idempotent retry.

### Failure modes (explicit)

- Missing/unauth `gh` → fail with remediation.
- Stack not linear → fail and print the offending PRs.
- Missing PR anywhere in stack → fail and suggest `stck submit` (later) or manual PR creation.
- Uncommitted changes → fail.
- Push rejected (branch protections) → surface git error; stop.

---

# Agent-friendly implementation plan (v0.1)

This section is written so you can hand chunks to coding agents.

## Milestone 0: Skeleton CLI

**Deliverable:** `stck` binary with subcommands and help text.

Acceptance:

- `stck --help` lists `new/status/sync/push`
- Commands run and fail with “not implemented” placeholders

---

## Milestone 1: Environment + repo checks (strict)

**Deliverable:** shared preflight used by all commands.

Checks:

- `git`, `gh`, `gh auth status`, `origin` exists, on a branch, clean tree
- discover and cache repo default branch (`origin`/GitHub metadata)

Acceptance:

- meaningful errors with “how to fix”

---

## Milestone 2: GitHub PR discovery (single PR)

**Deliverable:** `github::pr_for_head(branch)` and parse JSON.

Acceptance:

- For current branch, prints PR number, baseRefName, headRefName, state.
- Uses command shape that works in `gh` (`gh pr view <branch> ...`), no unsupported flags.

---

## Milestone 3: Stack discovery (linear)

**Deliverable:** build full stack (ancestors+descendants) and enforce linearity.

Acceptance:

- Given fixture PR graph, outputs ordered list.
- Detects non-linearity and prints candidates.
- Includes merged ancestors (`state=all`) and uses a single consistent snapshot where possible.

---

## Milestone 4: `stck status`

**Deliverable:** fetch + stack + indicators.

Acceptance:

- Runs `git fetch origin`
- Prints stack with PR metadata
- Marks base mismatch / needs sync / needs push

---

## Milestone 5: Rebase plan + executor (`stck sync`)

**Deliverable:** compute restack changes based on merged PRs, then run `git rebase --onto`.

Acceptance:

- On clean repo fixtures, produces correct rewritten history (integration test).
- On conflict, exits non-zero and does not corrupt state.
- Re-running continues correctly after user resolves.

---

## Milestone 6: `stck push`

**Deliverable:** retarget PR bases (as planned) + force-with-lease push.

Acceptance:

- Pushes all stack branches
- Applies PR base updates (mocked in tests)
- Fails transparently if push rejected
- Guarantees safe ordering: no retarget before successful pushes
- Supports idempotent retry after partial failure

---

## Milestone 7: `stck new`

**Deliverable:** push+PR for current if missing, create next branch, push, create PR with base=current.

Acceptance:

- Works from a branch with no upstream and no PR.
- Produces a stacked PR.

---

## Milestone 8: Homebrew release cycle (after core features)

### Objectives

- Provide a stable installation story:
  - `brew install <tap>/stck`
  - installs `stck` and also enables `git stck ...` via a `git-stck` symlink
- Release process is repeatable and automation-friendly.

### Release artifacts

- GitHub Release per version tag `vX.Y.Z`
- Attach prebuilt tarballs per platform/arch (at least macOS):
  - `stck-vX.Y.Z-x86_64-apple-darwin.tar.gz`
  - `stck-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- Each tarball contains:
  - `stck` binary
  - (optional) `LICENSE`, `README.md`
- Generate and publish SHA256 checksums for each tarball.

### CI / CD (GitHub Actions)

Workflow outline:

1. Trigger on version tag push: `v*`
2. Build matrix:
   - `aarch64-apple-darwin`
   - `x86_64-apple-darwin`
3. Build steps:
   - `cargo build --release`
   - package into tar.gz
   - compute sha256
4. Publish GitHub Release with artifacts + checksums

### Homebrew distribution strategy

Two common options; pick one:

**Option A: Homebrew Tap (recommended for v0.1)**

- Create a tap repo: `brew tap <org>/stck`
- Add formula `Formula/stck.rb` that:
  - downloads the correct tarball for macOS arch
  - installs `stck`
  - installs symlink `git-stck` → `stck`
- Update formula URLs + sha256 on each release.

**Option B: Homebrew Core (later)**

- After tool matures, submit to `homebrew-core` (higher bar, slower iteration).

### Formula behavior (key requirement: `git-stck` symlink)

In the formula’s `install`:

- `bin.install "stck"`
- `bin.install_symlink "stck" => "git-stck"`

Acceptance:

- After install: `stck --help` works
- After install: `git stck --help` works
- `which git-stck` shows it’s installed by brew

### Versioning

- Embed version string in binary: `stck --version` prints `X.Y.Z`
- Release tags must match binary version.

### Post-release checklist

- Verify install on both Apple Silicon + Intel (or via CI runner + local smoke test)
- Verify `git stck status` in a real repo
- Update README install instructions (Homebrew + Git subcommand note)

Acceptance for Milestone 8:

- Tagging `v0.1.0` produces a GitHub Release with macOS artifacts
- `brew install <tap>/stck` installs successfully
- Both `stck` and `git stck` function end-to-end for `status` at minimum

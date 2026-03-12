Repository information request
Bram
·
17m
ago
smart
brdv/stck:main
stck (v0.1.3) is your Rust CLI for managing stacked GitHub PRs. It automates the tedious parts — creating branches, rebasing stacks, force-pushing with lease, and updating PR base targets — while staying close to native git/gh behavior. It has 5 commands (new, submit, status, sync, push), only 3 runtime dependencies (clap, serde, serde_json), and shells out to git/gh rather than using library bindings. Currently in early public release (v0.1.x), installable via Homebrew or from source.

Here's the current feature set across 5 commands:

stck new <branch> — Creates the next branch in a stack. Auto-pushes, sets upstream, bootstraps a PR for the parent branch if missing, and opens a stacked PR targeting the correct base.

stck submit [--base <branch>] — Opens a PR for the current branch. Auto-discovers the stack parent from open PR metadata (up to 100 PRs), falls back to the default branch, or accepts an explicit --base.

stck status — Fetches origin, discovers the linear PR stack via gh, and prints stack order, PR state (open/merged), and flags like needs_sync, needs_push, and base_mismatch.

stck sync [--continue|--reset] — Rebases the local stack in order based on GitHub PR relationships. Handles merged parents by restacking onto the default branch. Supports resumable workflows — state is persisted to .git/stck/ so you can resolve conflicts and --continue, or --reset to start over.

stck push — Pushes rewritten branches with --force-with-lease and retargets PR bases on GitHub. Also resumable on partial failure.

Cross-cutting: preflight checks (clean worktree, git/gh installed, gh authenticated), git stck subcommand alias via Homebrew, and grep-friendly CLI output.

Good catch — let me trace the exact code path to confirm whether --force-with-lease is being neutralized by a prior fetch.

You're right — this is a confirmed bug. Here's the problem:

run_push (line 634) calls gitops::fetch_origin() before calling gitops::push_force_with_lease(). The fetch updates the remote tracking refs (refs/remotes/origin/<branch>), which is exactly what git push --force-with-lease checks against. So by the time the push runs, git thinks your local branch is "up to date" with the remote tracking ref and allows the force push — even though your local branch is actually missing commits that exist on the remote.

The same issue exists in run_sync (line 488 area) — it also fetches before rebasing, so the rebase itself may silently drop remote-only commits.

The fix should be: before pushing each branch, compare the local branch tip against the (freshly fetched) remote tracking ref to verify the local branch is a descendant of the remote. If it isn't (i.e., the remote has commits the local branch doesn't), stck push should abort for that branch with a warning instead of silently overwriting. Would you like me to implement that?

The approach: in run_push, after the fetch but before each push_force_with_lease, add a safety check using the existing gitops::is_ancestor() helper to verify that origin/<branch> is an ancestor of the local <branch>. If it's not — meaning the remote has commits the local branch doesn't — abort for that branch with a clear error telling the user to integrate the remote changes first (e.g., pull or rebase).

The building blocks are already there (is_ancestor wraps git merge-base --is-ancestor), so it's essentially a small guard inserted into the push loop. No changes needed to sync — the problem is only destructive at push time.

Now I understand the test harness. I need to look at how the git stub handles push --force-with-lease and the is_ancestor check to understand what env vars to use.

Now I have the full picture. The test needs to simulate a scenario where the remote has commits the local branch doesn't (remote is NOT an ancestor of local). I need to:

Set up a branch where rev-parse returns different SHAs for local vs remote (so needs_push is true)

Make the is_ancestor check for that remote→local pair return false (remote is ahead/diverged)

Expect that push currently succeeds (the bug), so the test should assert the push goes through — but written to assert the correct behavior (abort), meaning it fails now.

Let me write the test expecting the correct (fixed) behavior so it fails against the current code:

Now I need to add the stck_cmd_with_stubbed_tools_custom_git helper and the is_ancestor override to the harness. The key is making merge-base --is-ancestor origin/feature-branch feature-branch return exit code 1 (not an ancestor) for the diverged scenario:

Now I need to add setup_stubbed_tools_custom_git. Let me look at how the existing git stub is structured to inject extra behavior. Actually, a simpler approach: I can use the existing STCK_TEST_ANCESTOR_PAIRS env var mechanism in the merge-base --is-ancestor handler. The current stub defaults to allowing main:feature-branch, feature-branch:feature-child, etc. I just need a way to make a specific pair return "not ancestor". Let me re-read that section:

The current is_ancestor stub (lines 220-247) first checks STCK_TEST_ANCESTOR_PAIRS for explicit matches, then falls through to hardcoded cases. There's no way to make it return "not ancestor" for a hardcoded pair. Rather than adding a whole custom git approach, let me add a simpler env var STCK_TEST_NOT_ANCESTOR_PAIRS:

Good, reverted. Now let me take a cleaner approach — add a STCK_TEST_NOT_ANCESTOR_PAIRS env var to the existing git stub:

Now add the failing test to push.rs. The scenario: feature-branch has diverged from its remote (remote has commits local doesn't), so origin/feature-branch is NOT an ancestor of feature-branch. The fix should abort, but current code will happily push:

Now let's verify the test fails (confirming the bug):

The test fails as expected — confirming the bug. The current code happily pushes feature-branch with --force-with-lease even though the remote has commits the local branch doesn't. The test asserts the correct behavior (exit code 1, no force push), which will pass once the fix is implemented.

The oracle confirms this is a real bug. Here's the precise chain of events:

Diagram
The root cause: git push --force-with-lease (without an explicit expected SHA) checks against refs/remotes/origin/<branch>. The git fetch origin on line 634–637 of commands.rs updates that tracking ref to match the actual remote — so the lease check degrades to "remote still equals what I fetched moments ago" instead of "remote still equals what my rewrite was based on".

The proposed fix (checking is_ancestor(origin/<branch>, <branch>) after fetch, before push) catches exactly this: if the freshly-fetched remote tip is not contained in your local branch, you'd be overwriting commits. That's precisely what the test we wrote asserts.

Two realistic options:

Option A — Pre-push ancestor check (recommended)
After fetch, before each push_force_with_lease, call is_ancestor("origin/<branch>", "<branch>"). If false, abort with a clear error telling the user to integrate remote changes first. Simple, uses existing gitops::is_ancestor, no new deps, works with any Git version.

Option B — Explicit SHA lease
Use git push --force-with-lease=<branch>:<expected_sha> with a SHA captured before the rebase/fetch. More surgically correct, but requires plumbing the expected SHA through the sync state model and changing the push command — more invasive for the same practical protection.

I'd go with Option A because:

It's a ~10-line guard in the run_push loop

Uses infrastructure that already exists (is_ancestor)

Keeps the push command simple

Matches the test we already wrote

Gives a clear, actionable error message

The fix would go right before the push on line 742 — check ancestry, abort if diverged. Want me to go ahead?

Repository information request - Amp Docs

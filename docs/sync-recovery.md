# Sync Recovery Contract

`stck sync` restacks local branches only. It may fetch and read GitHub metadata,
but it does not push branches or change pull request bases.

This document is the authoritative recovery contract for interrupted sync
operations. Older planning notes that describe automatic continuation from a
plain `stck sync` rerun are historical.

## Fresh sync

A fresh `stck sync`:

1. refuses to start while a native Git rebase is already in progress,
2. fetches `origin` and computes the current stack plan,
3. saves that plan under `.git/stck/` before running its first rebase,
4. records progress after every completed step.

If a rebase fails, `stck` records the failed step and the branch head at which
it failed. That state remains available until the operation is continued,
reset, or completed.

## Continue after resolving a conflict

Finish the native Git rebase first:

```bash
# resolve the conflicted files
git add <resolved-files>
git rebase --continue

# repeat resolve/add/continue until Git finishes, then:
stck sync --continue
```

`stck sync --continue` is deliberately strict. It requires:

- saved sync state,
- no native rebase still in progress,
- evidence that the failed branch head changed after the completed rebase.

Only then does it mark the failed step complete and continue with later stack
branches. A plain `stck sync` does not infer this transition or skip the failed
step; it directs the user to choose `--continue` or `--reset` explicitly.

## Abort and recompute

To discard the interrupted attempt:

```bash
git rebase --abort
stck sync --reset
```

`stck sync --reset` clears the saved sync operation and computes a new plan
from current Git and GitHub state. It does not abort an active native rebase,
so `git rebase --abort` must finish first.

## State and command boundaries

- In-flight sync progress lives in `.git/stck/last-plan.json`.
- `stck push` is blocked while sync state remains unresolved.
- A successful sync clears in-flight state and saves a stack-scoped retarget
  plan in `.git/stck/last-sync-plan.json`.
- `stck push` reuses that cached plan only when its repository and exact stack
  metadata still match.
- A no-op sync clears any stale cached retarget plan.
- Sync recovery never pushes branches or mutates pull requests; `stck push`
  remains the explicit remote mutation step.

Do not edit files under `.git/stck/` manually. Use the recovery commands above
so state validation and cleanup remain intact.

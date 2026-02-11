# Usage

This guide describes the expected `stck` workflow in a stacked PR repository.

## Preconditions

- `git` and `gh` are installed and available in `PATH`.
- `gh auth status` is valid for your GitHub host.
- You are inside a GitHub repo with an `origin` remote.
- Your working tree is clean before running `stck` commands.

## Command Surface

```bash
stck new <branch>
stck status
stck sync
stck push
```

If installed via Homebrew, the Git subcommand entrypoint also works:

```bash
git stck <command>
```

## Typical Flow

### 1. Inspect current state

```bash
stck status
```

`status` fetches remote refs, discovers the linear PR stack, and prints:

- stack order from default branch to current tip,
- PR metadata (`open`/`merged`, base/head),
- indicators such as `needs sync` and `needs push`.

Use this command first whenever you are unsure if your branch is up to date.

### 2. Create the next stacked branch

From your current stack branch:

```bash
stck new feature-b
```

`new` will:

- ensure the current branch has an upstream on `origin`,
- ensure the current branch has a PR (bootstraps if missing),
- create and checkout `feature-b` from current `HEAD`,
- push `feature-b`,
- create a PR for `feature-b` with base = current branch.

If the new branch has no commits beyond its base, `stck` does not create an empty PR and reports that PR creation should happen after adding commits.

### 3. Sync local stack after upstream changes

```bash
stck sync
```

`sync` recomputes the stack/rebase plan from GitHub PR relationships and rebases branches locally in order.

- It may restack branches when a parent PR merged or base relationships changed.
- It updates local branches only.
- It does not push or retarget PR bases yet.

On success, it prints a follow-up message to run `stck push`.

### 4. Push rewritten branches and retarget PR bases

```bash
stck push
```

`push` applies remote changes for the last computed stack state:

- pushes stack branches with `--force-with-lease`,
- applies pending PR base retarget operations,
- reports summary and remaining work on failure.

The operation is designed for safe retries after partial failures.

## Quick Example

```bash
# start on feature-a
stck status
stck new feature-b
# implement + commit on feature-b
stck status
stck sync
stck push
```

## Notes

- `stck` assumes a linear stack in `v0.1.0` and fails fast for non-linear graphs.
- If a rebase conflict happens during `sync`, resolve conflicts with normal Git workflow, then continue and rerun `stck sync` as needed.

//! Git subprocess helpers used by stack planning and command execution.

use std::process::{Command, Stdio};
use std::{env, path::PathBuf};

use crate::util::with_stderr;

/// Fetch updated refs from the `origin` remote.
pub fn fetch_origin() -> Result<(), String> {
    let output = Command::new("git")
        .args(["fetch", "origin"])
        .output()
        .map_err(|_| {
            "failed to run `git fetch origin`; ensure this is a git repository".to_string()
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(with_stderr(
            "failed to fetch from `origin`; check remote connectivity and permissions",
            &output.stderr,
        ))
    }
}

/// Return whether the local branch head differs from `origin/<branch>`.
///
/// Missing remote refs are treated as needing a push so newly created branches
/// show up as actionable.
pub fn branch_needs_push(branch: &str) -> Result<bool, String> {
    let local_ref = format!("refs/heads/{branch}");
    let remote_ref = format!("refs/remotes/origin/{branch}");

    let local_sha = rev_parse(&local_ref)?;
    let remote_sha = match rev_parse(&remote_ref) {
        Ok(sha) => sha,
        Err(_) => return Ok(true),
    };

    Ok(local_sha != remote_sha)
}

/// Resolve a git reference to its full object SHA.
pub fn resolve_ref(reference: &str) -> Result<String, String> {
    rev_parse(reference)
}

/// Resolve the `--onto` target ref for a rebase, preferring `origin/<branch>`
/// over the local ref so the rebase target reflects the fetched remote state.
pub fn resolve_onto_ref(base_branch: &str) -> Result<String, String> {
    resolve_base_ref(base_branch)
}

/// Resolve the fork point to use as the old base for `git rebase --onto`.
///
/// This prefers the merge-base between `branch` and `base_branch` so sync can
/// recover from squash merges and rewritten ancestry. If no merge-base can be
/// found, the resolved base branch ref is used as a fallback.
pub fn resolve_old_base_for_rebase(base_branch: &str, branch: &str) -> Result<String, String> {
    // Try merge-base between the branch and the old base ref to find the true
    // fork point. This handles squash-merge and rewritten-ancestry scenarios
    // where the base branch tip may have moved past the actual divergence point.
    let base_ref = resolve_base_ref(base_branch)?;
    let branch_ref = format!("refs/heads/{branch}");
    if let Ok(fork) = merge_base(&base_ref, &branch_ref) {
        return Ok(fork);
    }

    // Fall back to resolving the base branch ref directly.
    rev_parse(&base_ref)
}

fn resolve_base_ref(base_branch: &str) -> Result<String, String> {
    // Prefer the remote ref because `stck sync` fetches before planning.
    // Using the local ref for shared branches like the default branch can
    // produce a stale merge-base when the remote has advanced (e.g. after a
    // PR merge on GitHub while local main has not been pulled).
    let remote_ref = format!("refs/remotes/origin/{base_branch}");
    if ref_exists(&remote_ref)? {
        return Ok(remote_ref);
    }

    let local_ref = format!("refs/heads/{base_branch}");
    if ref_exists(&local_ref)? {
        return Ok(local_ref);
    }

    Err(format!(
        "could not resolve old base branch `{base_branch}` locally or on origin; fetch and/or restore the branch, then rerun `stck sync`"
    ))
}

fn merge_base(ref_a: &str, ref_b: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["merge-base", ref_a, ref_b])
        .output()
        .map_err(|_| "failed to run `git merge-base`".to_string())?;

    if !output.status.success() {
        return Err(format!(
            "could not find merge-base between `{ref_a}` and `{ref_b}`"
        ));
    }

    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        Err(format!(
            "merge-base between `{ref_a}` and `{ref_b}` resolved to empty SHA"
        ))
    } else {
        Ok(sha)
    }
}

/// Return the absolute path to the repository's `.git` directory.
pub fn git_dir() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map_err(|_| "failed to run `git rev-parse --git-dir`".to_string())?;

    if !output.status.success() {
        return Err("could not determine git directory".to_string());
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() {
        return Err("git directory path is empty".to_string());
    }

    let path = PathBuf::from(raw);
    if path.is_absolute() {
        Ok(path)
    } else {
        let cwd = env::current_dir().map_err(|_| "failed to read current directory".to_string())?;
        Ok(cwd.join(path))
    }
}

/// Return whether `ancestor_ref` is an ancestor of `descendant_ref`.
pub fn is_ancestor(ancestor_ref: &str, descendant_ref: &str) -> Result<bool, String> {
    let output = Command::new("git")
        .args(["merge-base", "--is-ancestor", ancestor_ref, descendant_ref])
        .output()
        .map_err(|_| "failed to run `git merge-base --is-ancestor`".to_string())?;

    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!(
            "failed to check ancestry between `{ancestor_ref}` and `{descendant_ref}`"
        )),
    }
}

/// Detect whether a git rebase is currently in progress in this repository.
pub fn rebase_in_progress() -> Result<bool, String> {
    let git_dir = git_dir()?;
    Ok(git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists())
}

/// Return whether `branch` is behind the fetched remote default branch.
///
/// This uses `origin/<default_branch>` and therefore expects callers to fetch
/// before relying on the result.
pub fn branch_needs_sync_with_default(default_branch: &str, branch: &str) -> Result<bool, String> {
    let default_ref = format!("refs/remotes/origin/{default_branch}");
    let branch_ref = format!("refs/heads/{branch}");
    let output = Command::new("git")
        .args(["merge-base", "--is-ancestor", &default_ref, &branch_ref])
        .output()
        .map_err(|_| "failed to run `git merge-base --is-ancestor`".to_string())?;

    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(format!(
            "failed to compare {} against {}; ensure refs are available locally",
            default_ref, branch_ref
        )),
    }
}

/// Rebase `branch` onto `new_base`, using `old_base` as the fork point.
///
/// Standard git rebase progress and conflict output is inherited directly so
/// the user can continue or abort with native git commands when needed.
pub fn rebase_onto(new_base: &str, old_base: &str, branch: &str) -> Result<(), String> {
    let status = Command::new("git")
        .args(["rebase", "--onto", new_base, old_base, branch])
        .stderr(Stdio::inherit())
        .status()
        .map_err(|_| "failed to run `git rebase`; ensure this is a git repository".to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "rebase failed for branch {branch}; resolve conflicts, run `git rebase --continue` or `git rebase --abort`, then rerun `stck sync`"
        ))
    }
}

/// Push `branch` to `origin` as a regular (fast-forward) push.
///
/// Unlike [`push_force_with_lease`] this does **not** rewrite remote history.
/// A non-fast-forward push will fail, which is the desired safety behaviour
/// when the caller simply wants to publish new local commits.
pub fn push_branch(branch: &str) -> Result<(), String> {
    let status = Command::new("git")
        .args(["push", "origin", branch])
        .stderr(Stdio::inherit())
        .status()
        .map_err(|_| "failed to run `git push`; ensure this is a git repository".to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "push failed for branch {branch}; fix the push error and retry"
        ))
    }
}

/// Push `branch` to `origin` with `--force-with-lease`.
pub fn push_force_with_lease(branch: &str) -> Result<(), String> {
    let status = Command::new("git")
        .args(["push", "--force-with-lease", "origin", branch])
        .stderr(Stdio::inherit())
        .status()
        .map_err(|_| "failed to run `git push`; ensure this is a git repository".to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "push failed for branch {branch}; fix the push error and rerun `stck push`"
        ))
    }
}

/// Return whether `branch` has an upstream tracking branch configured.
pub fn branch_has_upstream(branch: &str) -> Result<bool, String> {
    let output = Command::new("git")
        .args([
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            &format!("{branch}@{{upstream}}"),
        ])
        .output()
        .map_err(|_| "failed to check branch upstream with `git rev-parse`".to_string())?;

    match output.status.code() {
        Some(0) => Ok(true),
        Some(_) => Ok(false),
        None => Err("failed to check branch upstream".to_string()),
    }
}

/// Return whether a local branch named `branch` exists.
pub fn local_branch_exists(branch: &str) -> Result<bool, String> {
    ref_exists(&format!("refs/heads/{branch}"))
}

/// Return whether `origin/<branch>` exists locally.
pub fn remote_branch_exists(branch: &str) -> Result<bool, String> {
    ref_exists(&format!("refs/remotes/origin/{branch}"))
}

/// Push `branch` to `origin` and configure it as the upstream branch.
pub fn push_set_upstream(branch: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["push", "-u", "origin", branch])
        .output()
        .map_err(|_| "failed to run `git push -u`; ensure this is a git repository".to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(with_stderr(
            &format!("failed to push branch {branch} with upstream; fix the push error and retry"),
            &output.stderr,
        ))
    }
}

/// Create and check out a new local branch.
pub fn checkout_new_branch(branch: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["checkout", "-b", branch])
        .output()
        .map_err(|_| {
            "failed to run `git checkout -b`; ensure this is a git repository".to_string()
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(with_stderr(
            &format!(
                "failed to create and checkout branch {branch}; ensure the branch name is valid and does not already exist"
            ),
            &output.stderr,
        ))
    }
}

/// Check out an existing local branch.
pub fn checkout_branch(branch: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["checkout", branch])
        .output()
        .map_err(|_| "failed to run `git checkout`; ensure this is a git repository".to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(with_stderr(
            &format!("failed to checkout branch {branch}; switch branches manually and retry"),
            &output.stderr,
        ))
    }
}

/// Return whether `head` contains commits not present on `base`.
pub fn has_commits_between(base: &str, head: &str) -> Result<bool, String> {
    let output = Command::new("git")
        .args([
            "rev-list",
            "--count",
            &format!("refs/heads/{base}..refs/heads/{head}"),
        ])
        .output()
        .map_err(|_| "failed to run `git rev-list --count`".to_string())?;

    if !output.status.success() {
        return Err(format!(
            "failed to compare branches {base} and {head}; ensure both branches exist locally"
        ));
    }

    let count = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()
        .map_err(|_| "failed to parse commit count from `git rev-list --count`".to_string())?;
    Ok(count > 0)
}

fn rev_parse(reference: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", reference])
        .output()
        .map_err(|_| format!("failed to run `git rev-parse` for `{reference}`"))?;

    if !output.status.success() {
        return Err(format!("could not resolve git reference `{reference}`"));
    }

    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        Err(format!("git reference `{reference}` resolved to empty SHA"))
    } else {
        Ok(sha)
    }
}

/// Return whether `name` is accepted by `git check-ref-format --allow-onelevel`.
pub fn is_valid_branch_name(name: &str) -> Result<bool, String> {
    let output = Command::new("git")
        .args(["check-ref-format", "--allow-onelevel", name])
        .output()
        .map_err(|_| "failed to run `git check-ref-format`".to_string())?;

    match output.status.code() {
        Some(0) => Ok(true),
        Some(_) => Ok(false),
        None => Err("failed to validate branch name".to_string()),
    }
}

fn ref_exists(reference: &str) -> Result<bool, String> {
    let output = Command::new("git")
        .args(["show-ref", "--verify", "--quiet", reference])
        .output()
        .map_err(|_| format!("failed to run `git show-ref` for `{reference}`"))?;

    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!(
            "failed to verify git reference `{reference}`; ensure this is a git repository"
        )),
    }
}

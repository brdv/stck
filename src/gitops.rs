use std::process::Command;
use std::{env, path::PathBuf};

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

pub fn resolve_ref(reference: &str) -> Result<String, String> {
    rev_parse(reference)
}

pub fn resolve_old_base_for_rebase(base_branch: &str) -> Result<String, String> {
    let local_ref = format!("refs/heads/{base_branch}");
    if let Ok(sha) = rev_parse(&local_ref) {
        return Ok(sha);
    }

    let remote_ref = format!("refs/remotes/origin/{base_branch}");
    if let Ok(sha) = rev_parse(&remote_ref) {
        return Ok(sha);
    }

    Err(format!(
        "could not resolve old base branch `{base_branch}` locally or on origin; fetch and/or restore the branch, then rerun `stck sync`"
    ))
}

pub fn derive_rebase_boundary(
    old_base_branch: &str,
    new_base_branch: &str,
    branch: &str,
) -> Result<String, String> {
    let branch_ref = format!("refs/heads/{branch}");
    let old_base_candidates = [
        format!("refs/heads/{old_base_branch}"),
        format!("refs/remotes/origin/{old_base_branch}"),
    ];

    for base_ref in old_base_candidates
        .iter()
        .filter(|candidate| rev_parse(candidate).is_ok())
    {
        if let Ok(sha) = merge_base_fork_point(base_ref, &branch_ref) {
            return Ok(sha);
        }
    }

    for base_ref in old_base_candidates
        .iter()
        .filter(|candidate| rev_parse(candidate).is_ok())
    {
        if let Ok(sha) = merge_base(base_ref, &branch_ref) {
            return Ok(sha);
        }
    }

    let new_base_candidates = [
        format!("refs/heads/{new_base_branch}"),
        format!("refs/remotes/origin/{new_base_branch}"),
    ];
    for base_ref in new_base_candidates
        .iter()
        .filter(|candidate| rev_parse(candidate).is_ok())
    {
        if let Ok(sha) = merge_base(base_ref, &branch_ref) {
            return Ok(sha);
        }
    }

    resolve_old_base_for_rebase(old_base_branch)
}

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

pub fn rebase_in_progress() -> Result<bool, String> {
    let git_dir = git_dir()?;
    Ok(git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists())
}

pub fn ref_is_ancestor(ancestor_ref: &str, descendant_ref: &str) -> Result<bool, String> {
    let output = Command::new("git")
        .args(["merge-base", "--is-ancestor", ancestor_ref, descendant_ref])
        .output()
        .map_err(|_| "failed to run `git merge-base --is-ancestor`".to_string())?;

    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!(
            "failed to compare {} against {}; ensure refs are available locally",
            ancestor_ref, descendant_ref
        )),
    }
}

pub fn commit_distance(ancestor_ref: &str, descendant_ref: &str) -> Result<usize, String> {
    let output = Command::new("git")
        .args([
            "rev-list",
            "--count",
            &format!("{ancestor_ref}..{descendant_ref}"),
        ])
        .output()
        .map_err(|_| "failed to run `git rev-list --count`".to_string())?;

    if !output.status.success() {
        return Err(format!(
            "failed to compare refs {} and {}; ensure refs are available locally",
            ancestor_ref, descendant_ref
        ));
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()
        .map_err(|_| "failed to parse commit count from `git rev-list --count`".to_string())
}

pub fn branch_needs_sync_with_default(default_branch: &str, branch: &str) -> Result<bool, String> {
    let default_ref = format!("refs/remotes/origin/{default_branch}");
    let branch_ref = format!("refs/heads/{branch}");
    ref_is_ancestor(&default_ref, &branch_ref).map(|is_ancestor| !is_ancestor)
}

pub fn rebase_onto(new_base: &str, old_base: &str, branch: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["rebase", "--onto", new_base, old_base, branch])
        .output()
        .map_err(|_| "failed to run `git rebase`; ensure this is a git repository".to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(with_stderr(
            &format!(
                "rebase failed for branch {branch}; resolve conflicts, run `git rebase --continue` or `git rebase --abort`, then rerun `stck sync`"
            ),
            &output.stderr,
        ))
    }
}

pub fn push_force_with_lease(branch: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["push", "--force-with-lease", "origin", branch])
        .output()
        .map_err(|_| "failed to run `git push`; ensure this is a git repository".to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(with_stderr(
            &format!("push failed for branch {branch}; fix the push error and rerun `stck push`"),
            &output.stderr,
        ))
    }
}

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

pub fn local_branch_exists(branch: &str) -> Result<bool, String> {
    ref_exists(&format!("refs/heads/{branch}"))
}

pub fn remote_branch_exists(branch: &str) -> Result<bool, String> {
    ref_exists(&format!("refs/remotes/origin/{branch}"))
}

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

fn merge_base_fork_point(base: &str, branch: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["merge-base", "--fork-point", base, branch])
        .output()
        .map_err(|_| "failed to run `git merge-base --fork-point`".to_string())?;

    if !output.status.success() {
        return Err("no valid fork-point found".to_string());
    }

    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        Err("`git merge-base --fork-point` returned empty output".to_string())
    } else {
        Ok(sha)
    }
}

fn merge_base(base: &str, branch: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["merge-base", base, branch])
        .output()
        .map_err(|_| "failed to run `git merge-base`".to_string())?;

    if !output.status.success() {
        return Err("failed to compute merge-base".to_string());
    }

    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        Err("`git merge-base` returned empty output".to_string())
    } else {
        Ok(sha)
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

fn with_stderr(base: &str, stderr: &[u8]) -> String {
    let detail = String::from_utf8_lossy(stderr).trim().to_string();
    if detail.is_empty() {
        base.to_string()
    } else {
        format!("{base}; stderr: {detail}")
    }
}

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
        Err("failed to fetch from `origin`; check remote connectivity and permissions".to_string())
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

pub fn rebase_onto(new_base: &str, old_base: &str, branch: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["rebase", "--onto", new_base, old_base, branch])
        .output()
        .map_err(|_| "failed to run `git rebase`; ensure this is a git repository".to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "rebase failed for branch {branch}; resolve conflicts, run `git rebase --continue` or `git rebase --abort`, then rerun `stck sync`"
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
        Err(format!(
            "push failed for branch {branch}; fix the push error and rerun `stck push`"
        ))
    }
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

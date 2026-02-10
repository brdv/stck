use std::process::Command;

#[derive(Debug, Clone)]
pub struct PreflightContext {
    pub default_branch: String,
}

pub fn run_preflight() -> Result<PreflightContext, String> {
    ensure_command_available("git")?;
    ensure_command_available("gh")?;
    ensure_gh_auth()?;
    ensure_origin_remote()?;
    ensure_on_branch()?;
    ensure_clean_working_tree()?;
    let default_branch = discover_default_branch()?;

    Ok(PreflightContext { default_branch })
}

fn ensure_command_available(command: &str) -> Result<(), String> {
    let output = Command::new(command)
        .arg("--version")
        .output()
        .map_err(|_| {
            format!("required command `{command}` was not found in PATH; install it and retry")
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to execute `{command} --version`; ensure `{command}` is installed and runnable"
        ))
    }
}

fn ensure_gh_auth() -> Result<(), String> {
    let output = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map_err(|_| {
            "failed to run `gh auth status`; install GitHub CLI and authenticate".to_string()
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err("GitHub CLI is not authenticated; run `gh auth login` and retry".to_string())
    }
}

fn ensure_origin_remote() -> Result<(), String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|_| {
            "failed to run `git remote get-url origin`; ensure this is a git repository".to_string()
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err("`origin` remote is missing; add it with `git remote add origin <url>`".to_string())
    }
}

fn ensure_on_branch() -> Result<(), String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
        .output()
        .map_err(|_| {
            "failed to determine current branch; ensure this is a git repository".to_string()
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err("not on a branch (detached HEAD); checkout a branch and retry".to_string())
    }
}

fn ensure_clean_working_tree() -> Result<(), String> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map_err(|_| {
            "failed to inspect working tree; ensure this is a git repository".to_string()
        })?;

    if !output.status.success() {
        return Err(
            "failed to check working tree status; ensure this is a git repository".to_string(),
        );
    }

    let is_clean = output.stdout.is_empty();
    if is_clean {
        Ok(())
    } else {
        Err(
            "working tree is not clean; commit, stash, or discard changes before running stck"
                .to_string(),
        )
    }
}

fn discover_default_branch() -> Result<String, String> {
    let output = Command::new("gh")
        .args([
            "repo",
            "view",
            "--json",
            "defaultBranchRef",
            "--jq",
            ".defaultBranchRef.name",
        ])
        .output()
        .map_err(|_| "failed to discover repository default branch from GitHub".to_string())?;

    if !output.status.success() {
        return Err(
            "could not discover default branch via GitHub CLI; ensure `origin` points to GitHub and `gh auth status` succeeds"
                .to_string(),
        );
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        Err(
            "default branch lookup returned empty result; verify repository metadata on GitHub"
                .to_string(),
        )
    } else {
        Ok(branch)
    }
}

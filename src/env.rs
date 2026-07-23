//! Repository and toolchain preflight checks required before running commands.

use std::process::Command;

/// Repository context gathered during preflight and reused by command handlers.
#[derive(Debug, Clone)]
pub struct PreflightContext {
    /// The canonical GitHub repository name in `owner/name` form.
    pub repository: String,
    /// The currently checked-out local branch.
    pub current_branch: String,
    /// The repository's default branch as reported by GitHub.
    pub default_branch: String,
}

/// Validate the local repository and discover branch context needed by `stck`.
///
/// This checks that `git` and `gh` are installed, GitHub authentication is
/// available, the repository has an `origin` remote, the current HEAD is on a
/// branch, the working tree is clean, and the default branch can be discovered.
pub fn run_preflight() -> Result<PreflightContext, String> {
    ensure_command_available("git")?;
    ensure_command_available("gh")?;
    ensure_gh_auth()?;
    ensure_origin_remote()?;
    let current_branch = ensure_on_branch()?;
    ensure_clean_working_tree()?;
    let (repository, default_branch) = discover_repository_context()?;

    Ok(PreflightContext {
        repository,
        current_branch,
        default_branch,
    })
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

fn ensure_on_branch() -> Result<String, String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
        .output()
        .map_err(|_| {
            "failed to determine current branch; ensure this is a git repository".to_string()
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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

fn discover_repository_context() -> Result<(String, String), String> {
    let output = Command::new("gh")
        .args([
            "repo",
            "view",
            "--json",
            "nameWithOwner,defaultBranchRef",
            "--jq",
            r#"[.nameWithOwner, .defaultBranchRef.name] | @tsv"#,
        ])
        .output()
        .map_err(|_| "failed to discover repository default branch from GitHub".to_string())?;

    if !output.status.success() {
        return Err(
            "could not discover default branch via GitHub CLI; ensure `origin` points to GitHub and `gh auth status` succeeds"
                .to_string(),
        );
    }

    let metadata = String::from_utf8_lossy(&output.stdout);
    let Some((repository, default_branch)) = metadata.trim().split_once('\t') else {
        return Err(
            "repository metadata lookup returned an invalid result; verify repository metadata on GitHub"
                .to_string(),
        );
    };
    if default_branch.is_empty() {
        Err(
            "default branch lookup returned empty result; verify repository metadata on GitHub"
                .to_string(),
        )
    } else if repository.is_empty() {
        Err(
            "repository identity lookup returned empty result; verify repository metadata on GitHub"
                .to_string(),
        )
    } else {
        Ok((repository.to_string(), default_branch.to_string()))
    }
}

use std::process::Command;

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

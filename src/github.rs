use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    #[serde(rename = "headRefName")]
    pub head_ref_name: String,
    #[serde(rename = "baseRefName")]
    pub base_ref_name: String,
    pub state: String,
    #[serde(rename = "mergedAt")]
    pub merged_at: Option<String>,
}

pub fn pr_for_head(branch: &str) -> Result<PullRequest, String> {
    let output = Command::new("gh")
        .args([
            "pr",
            "view",
            branch,
            "--json",
            "number,headRefName,baseRefName,state,mergedAt",
        ])
        .output()
        .map_err(|_| "failed to run `gh pr view`; ensure GitHub CLI is installed".to_string())?;

    if !output.status.success() {
        return Err(format!(
            "no PR found for branch {branch}; create a PR first"
        ));
    }

    serde_json::from_slice::<PullRequest>(&output.stdout).map_err(|_| {
        format!("failed to parse PR metadata for branch {branch} from GitHub CLI output")
    })
}

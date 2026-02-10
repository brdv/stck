use serde::Deserialize;
use std::collections::HashMap;
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

pub fn discover_linear_stack(
    current_branch: &str,
    default_branch: &str,
) -> Result<Vec<PullRequest>, String> {
    let prs = list_pull_requests()?;
    let by_head: HashMap<&str, &PullRequest> = prs
        .iter()
        .map(|pr| (pr.head_ref_name.as_str(), pr))
        .collect();

    let current = by_head
        .get(current_branch)
        .copied()
        .ok_or_else(|| format!("no PR found for branch {current_branch}; create a PR first"))?;

    let mut seen = vec![current.head_ref_name.clone()];
    let mut to_current = vec![current.clone()];
    let mut cursor = current;

    while cursor.base_ref_name != default_branch {
        let parent = by_head
            .get(cursor.base_ref_name.as_str())
            .copied()
            .ok_or_else(|| {
                format!(
                    "no PR found for branch {}; create a PR first",
                    cursor.base_ref_name
                )
            })?;

        if seen.iter().any(|branch| branch == &parent.head_ref_name) {
            return Err(format!(
                "cycle detected in stack at branch {}",
                parent.head_ref_name
            ));
        }

        seen.push(parent.head_ref_name.clone());
        to_current.push(parent.clone());
        cursor = parent;
    }

    let mut current_to_top = vec![current.clone()];
    cursor = current;
    loop {
        let mut children: Vec<&PullRequest> = prs
            .iter()
            .filter(|candidate| candidate.base_ref_name == cursor.head_ref_name)
            .collect();

        children.sort_by(|a, b| a.head_ref_name.cmp(&b.head_ref_name));

        match children.len() {
            0 => break,
            1 => {
                let child = children[0];
                if seen.iter().any(|branch| branch == &child.head_ref_name) {
                    return Err(format!(
                        "cycle detected in stack at branch {}",
                        child.head_ref_name
                    ));
                }
                seen.push(child.head_ref_name.clone());
                current_to_top.push(child.clone());
                cursor = child;
            }
            _ => {
                let candidates = children
                    .iter()
                    .map(|child| child.head_ref_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!(
                    "non-linear stack detected at {}; child candidates: {}",
                    cursor.head_ref_name, candidates
                ));
            }
        }
    }

    to_current.reverse();
    let mut stack = to_current;
    stack.extend(current_to_top.into_iter().skip(1));
    Ok(stack)
}

fn list_pull_requests() -> Result<Vec<PullRequest>, String> {
    let output = Command::new("gh")
        .args([
            "pr",
            "list",
            "--state",
            "all",
            "--limit",
            "500",
            "--json",
            "number,headRefName,baseRefName,state,mergedAt",
        ])
        .output()
        .map_err(|_| "failed to run `gh pr list`; ensure GitHub CLI is installed".to_string())?;

    if !output.status.success() {
        return Err("failed to list pull requests from GitHub; ensure `gh auth status` succeeds and the repository is accessible".to_string());
    }

    serde_json::from_slice::<Vec<PullRequest>>(&output.stdout)
        .map_err(|_| "failed to parse PR metadata from GitHub CLI output".to_string())
}

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
    build_linear_stack(&prs, current_branch, default_branch)
}

pub fn retarget_pr_base(branch: &str, new_base: &str) -> Result<(), String> {
    let output = Command::new("gh")
        .args(["pr", "edit", branch, "--base", new_base])
        .output()
        .map_err(|_| "failed to run `gh pr edit`; ensure GitHub CLI is installed".to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to retarget PR base for branch {branch} to {new_base}; fix the GitHub error and rerun `stck push`"
        ))
    }
}

pub fn pr_exists_for_head(branch: &str) -> Result<bool, String> {
    let output = Command::new("gh")
        .args(["pr", "view", branch, "--json", "number"])
        .output()
        .map_err(|_| "failed to run `gh pr view`; ensure GitHub CLI is installed".to_string())?;

    match output.status.code() {
        Some(0) => Ok(true),
        Some(_) => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
            if stderr.contains("no pull requests found")
                || stderr.contains("could not resolve to a pull request")
            {
                Ok(false)
            } else {
                Err(format!(
                    "failed to check PR for branch {branch}; ensure `gh auth status` succeeds and retry"
                ))
            }
        }
        None => Err("failed to determine PR presence for branch".to_string()),
    }
}

pub fn create_pr(base: &str, head: &str, title: &str) -> Result<(), String> {
    let output = Command::new("gh")
        .args([
            "pr", "create", "--base", base, "--head", head, "--title", title, "--body", "",
        ])
        .output()
        .map_err(|_| "failed to run `gh pr create`; ensure GitHub CLI is installed".to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to create PR for branch {head}; fix the GitHub error and retry"
        ))
    }
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

    parse_pull_requests_json(&output.stdout)
}

fn parse_pull_requests_json(bytes: &[u8]) -> Result<Vec<PullRequest>, String> {
    serde_json::from_slice::<Vec<PullRequest>>(bytes)
        .map_err(|_| "failed to parse PR metadata from GitHub CLI output".to_string())
}

pub fn build_linear_stack(
    prs: &[PullRequest],
    current_branch: &str,
    default_branch: &str,
) -> Result<Vec<PullRequest>, String> {
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

#[cfg(test)]
mod tests {
    use super::{build_linear_stack, PullRequest};

    fn pr(number: u64, head: &str, base: &str) -> PullRequest {
        PullRequest {
            number,
            head_ref_name: head.to_string(),
            base_ref_name: base.to_string(),
            state: "OPEN".to_string(),
            merged_at: None,
        }
    }

    #[test]
    fn builds_linear_stack_from_ancestor_to_descendant() {
        let prs = vec![
            pr(100, "feature-base", "main"),
            pr(101, "feature-mid", "feature-base"),
            pr(102, "feature-top", "feature-mid"),
        ];

        let stack = build_linear_stack(&prs, "feature-mid", "main").expect("stack should build");
        let heads = stack
            .iter()
            .map(|item| item.head_ref_name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(heads, vec!["feature-base", "feature-mid", "feature-top"]);
    }

    #[test]
    fn errors_when_current_branch_pr_is_missing() {
        let prs = vec![pr(100, "feature-base", "main")];

        let error = build_linear_stack(&prs, "feature-mid", "main")
            .expect_err("missing current branch PR should error");

        assert_eq!(
            error,
            "no PR found for branch feature-mid; create a PR first"
        );
    }

    #[test]
    fn errors_when_parent_pr_is_missing() {
        let prs = vec![pr(101, "feature-mid", "feature-base")];

        let error = build_linear_stack(&prs, "feature-mid", "main")
            .expect_err("missing parent PR should error");

        assert_eq!(
            error,
            "no PR found for branch feature-base; create a PR first"
        );
    }

    #[test]
    fn errors_on_non_linear_descendants() {
        let prs = vec![
            pr(100, "feature-base", "main"),
            pr(101, "feature-mid", "feature-base"),
            pr(102, "feature-child-a", "feature-mid"),
            pr(103, "feature-child-b", "feature-mid"),
        ];

        let error = build_linear_stack(&prs, "feature-mid", "main")
            .expect_err("non-linear descendants should error");

        assert_eq!(
            error,
            "non-linear stack detected at feature-mid; child candidates: feature-child-a, feature-child-b"
        );
    }

    #[test]
    fn errors_on_cycle_detection() {
        let prs = vec![
            pr(100, "feature-a", "feature-b"),
            pr(101, "feature-b", "feature-a"),
        ];

        let error =
            build_linear_stack(&prs, "feature-a", "main").expect_err("cycle should be detected");

        assert_eq!(error, "cycle detected in stack at branch feature-a");
    }

    #[test]
    fn parses_single_pull_request_json() {
        let raw = r#"{
            "number": 101,
            "headRefName": "feature-branch",
            "baseRefName": "main",
            "state": "OPEN",
            "mergedAt": null
        }"#;

        let parsed: PullRequest =
            serde_json::from_str(raw).expect("single pull request JSON should parse");

        assert_eq!(parsed.number, 101);
        assert_eq!(parsed.head_ref_name, "feature-branch");
        assert_eq!(parsed.base_ref_name, "main");
        assert_eq!(parsed.state, "OPEN");
        assert_eq!(parsed.merged_at, None);
    }

    #[test]
    fn fails_parsing_single_pull_request_when_required_fields_are_missing() {
        let raw = r#"{
            "number": 101,
            "headRefName": "feature-branch",
            "state": "OPEN"
        }"#;

        let parsed = serde_json::from_str::<PullRequest>(raw);
        assert!(parsed.is_err(), "missing required fields should fail parse");
    }

    #[test]
    fn parses_pull_request_list_json() {
        let raw = r#"[
            {
                "number": 100,
                "headRefName": "feature-base",
                "baseRefName": "main",
                "state": "MERGED",
                "mergedAt": "2026-01-01T00:00:00Z"
            },
            {
                "number": 101,
                "headRefName": "feature-branch",
                "baseRefName": "feature-base",
                "state": "OPEN",
                "mergedAt": null
            }
        ]"#;

        let parsed: Vec<PullRequest> =
            serde_json::from_str(raw).expect("pull request list JSON should parse");

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].head_ref_name, "feature-base");
        assert_eq!(parsed[1].head_ref_name, "feature-branch");
    }

    #[test]
    fn fails_parsing_pull_request_list_json_when_malformed() {
        let raw = r#"[
            {
                "number": 100,
                "headRefName": "feature-base"
        "#;

        let parsed = serde_json::from_str::<Vec<PullRequest>>(raw);
        assert!(parsed.is_err(), "malformed list JSON should fail parse");
    }
}

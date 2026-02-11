use crate::github::PullRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine {
    pub branch: String,
    pub number: u64,
    pub state: String,
    pub base: String,
    pub head: String,
    pub flags: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSummary {
    pub needs_sync: usize,
    pub needs_push: usize,
    pub base_mismatch: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusReport {
    pub lines: Vec<StatusLine>,
    pub summary: StatusSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStep {
    pub branch: String,
    pub old_base_ref: String,
    pub new_base_ref: String,
}

pub fn build_status_report(stack: &[PullRequest], default_branch: &str) -> StatusReport {
    let mut lines = Vec::with_capacity(stack.len());
    let mut needs_sync = 0usize;
    let mut needs_push = 0usize;
    let mut base_mismatch = 0usize;

    for (index, pr) in stack.iter().enumerate() {
        let expected_base = if index == 0 {
            default_branch
        } else {
            stack[index - 1].head_ref_name.as_str()
        };

        let has_base_mismatch = pr.base_ref_name != expected_base;
        let parent_is_merged = index > 0
            && (stack[index - 1].state == "MERGED" || stack[index - 1].merged_at.is_some());

        // Milestone 4 scope: compute stack-based indicators from PR graph.
        // Push divergence checks will be added in later milestones.
        let has_needs_push = false;
        let has_needs_sync = has_base_mismatch || parent_is_merged;

        let mut flags = Vec::new();
        if has_base_mismatch {
            flags.push("base_mismatch");
            base_mismatch += 1;
        }
        if has_needs_sync {
            flags.push("needs_sync");
            needs_sync += 1;
        }
        if has_needs_push {
            flags.push("needs_push");
            needs_push += 1;
        }

        lines.push(StatusLine {
            branch: pr.head_ref_name.clone(),
            number: pr.number,
            state: pr.state.clone(),
            base: pr.base_ref_name.clone(),
            head: pr.head_ref_name.clone(),
            flags,
        });
    }

    StatusReport {
        lines,
        summary: StatusSummary {
            needs_sync,
            needs_push,
            base_mismatch,
        },
    }
}

pub fn build_sync_plan(stack: &[PullRequest], default_branch: &str) -> Vec<SyncStep> {
    let mut steps = Vec::new();
    let mut previous_open_branch: Option<&str> = None;
    let mut previous_open_rewritten = false;

    for pr in stack {
        if pr.state == "MERGED" || pr.merged_at.is_some() {
            continue;
        }

        let target_base = previous_open_branch.unwrap_or(default_branch);
        let base_changed = pr.base_ref_name != target_base;
        let needs_rebase = base_changed || previous_open_rewritten;

        if needs_rebase {
            steps.push(SyncStep {
                branch: pr.head_ref_name.clone(),
                old_base_ref: pr.base_ref_name.clone(),
                new_base_ref: target_base.to_string(),
            });
        }

        previous_open_branch = Some(pr.head_ref_name.as_str());
        previous_open_rewritten = needs_rebase;
    }

    steps
}

#[cfg(test)]
mod tests {
    use super::{build_status_report, build_sync_plan, SyncStep};
    use crate::github::PullRequest;

    fn pr(number: u64, head: &str, base: &str, state: &str) -> PullRequest {
        PullRequest {
            number,
            head_ref_name: head.to_string(),
            base_ref_name: base.to_string(),
            state: state.to_string(),
            merged_at: None,
        }
    }

    #[test]
    fn reports_no_flags_for_aligned_open_stack() {
        let stack = vec![
            pr(100, "feature-a", "main", "OPEN"),
            pr(101, "feature-b", "feature-a", "OPEN"),
        ];

        let report = build_status_report(&stack, "main");

        assert_eq!(report.summary.needs_sync, 0);
        assert_eq!(report.summary.needs_push, 0);
        assert_eq!(report.summary.base_mismatch, 0);
        assert_eq!(report.lines[0].flags, Vec::<&str>::new());
        assert_eq!(report.lines[1].flags, Vec::<&str>::new());
    }

    #[test]
    fn reports_needs_sync_when_parent_is_merged() {
        let stack = vec![
            pr(100, "feature-a", "main", "MERGED"),
            pr(101, "feature-b", "feature-a", "OPEN"),
        ];

        let report = build_status_report(&stack, "main");

        assert_eq!(report.summary.needs_sync, 1);
        assert_eq!(report.summary.base_mismatch, 0);
        assert_eq!(report.lines[1].flags, vec!["needs_sync"]);
    }

    #[test]
    fn reports_base_mismatch_and_needs_sync_together() {
        let stack = vec![
            pr(100, "feature-a", "main", "OPEN"),
            pr(101, "feature-b", "main", "OPEN"),
        ];

        let report = build_status_report(&stack, "main");

        assert_eq!(report.summary.needs_sync, 1);
        assert_eq!(report.summary.base_mismatch, 1);
        assert_eq!(report.lines[1].flags, vec!["base_mismatch", "needs_sync"]);
    }

    #[test]
    fn sync_plan_is_empty_when_open_stack_is_aligned() {
        let stack = vec![
            pr(100, "feature-a", "main", "OPEN"),
            pr(101, "feature-b", "feature-a", "OPEN"),
        ];

        let plan = build_sync_plan(&stack, "main");
        assert!(plan.is_empty());
    }

    #[test]
    fn sync_plan_restacks_child_of_merged_parent_and_descendants() {
        let stack = vec![
            pr(100, "feature-a", "main", "MERGED"),
            pr(101, "feature-b", "feature-a", "OPEN"),
            pr(102, "feature-c", "feature-b", "OPEN"),
        ];

        let plan = build_sync_plan(&stack, "main");
        assert_eq!(
            plan,
            vec![
                SyncStep {
                    branch: "feature-b".to_string(),
                    old_base_ref: "feature-a".to_string(),
                    new_base_ref: "main".to_string(),
                },
                SyncStep {
                    branch: "feature-c".to_string(),
                    old_base_ref: "feature-b".to_string(),
                    new_base_ref: "feature-b".to_string(),
                },
            ]
        );
    }

    #[test]
    fn sync_plan_restacks_on_base_mismatch() {
        let stack = vec![
            pr(100, "feature-a", "main", "OPEN"),
            pr(101, "feature-b", "main", "OPEN"),
        ];

        let plan = build_sync_plan(&stack, "main");
        assert_eq!(
            plan,
            vec![SyncStep {
                branch: "feature-b".to_string(),
                old_base_ref: "main".to_string(),
                new_base_ref: "feature-a".to_string(),
            }]
        );
    }
}

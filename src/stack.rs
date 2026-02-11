use crate::github::PullRequest;

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

#[cfg(test)]
mod tests {
    use super::build_status_report;
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
}

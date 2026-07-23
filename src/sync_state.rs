//! Persistence for resumable `sync` and `push` workflows under `.git/stck/`.

use crate::github::PullRequest;
use crate::gitops;
use crate::stack::{RetargetStep, SyncStep};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Saved progress for an in-flight `stck sync` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// The full ordered list of rebase steps for the current sync run.
    pub steps: Vec<SyncStep>,
    /// Number of sync steps that completed successfully.
    pub completed_steps: usize,
    /// Index of the step that most recently failed, if any.
    pub failed_step: Option<usize>,
    /// Recorded branch head after the failed step began, used to validate resume behavior.
    #[serde(default)]
    pub failed_step_branch_head: Option<String>,
    /// Repository and PR stack the sync plan was computed for.
    #[serde(default)]
    pub(crate) plan_scope: Option<SyncPlanScope>,
}

/// Saved progress for an in-flight `stck push` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushState {
    /// Branches that still need to be pushed, in execution order.
    pub push_branches: Vec<String>,
    /// Number of branch pushes that completed successfully.
    pub completed_pushes: usize,
    /// Remote branch tips captured by the sync that authorized rewritten pushes.
    #[serde(default)]
    pub(crate) sync_push_leases: Vec<RemoteBranchLease>,
    /// PR base retarget operations to run after the branch pushes succeed.
    pub retargets: Vec<RetargetStep>,
    /// Number of retarget operations that completed successfully.
    pub completed_retargets: usize,
}

/// Cached retarget plan produced by the most recent successful sync run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastSyncPlan {
    /// The default branch that the cached plan was built against.
    pub default_branch: String,
    /// Repository and exact PR stack that produced the cached plan.
    #[serde(default)]
    pub(crate) scope: Option<SyncPlanScope>,
    /// PR retarget operations implied by the sync plan.
    pub retargets: Vec<RetargetStep>,
}

/// Expected remote state captured before sync rewrites a local branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RemoteBranchLease {
    /// Local branch whose rewritten history may be pushed.
    pub(crate) branch: String,
    /// Fetched remote tip expected at push time, or `None` when it was absent.
    pub(crate) expected_remote_head: Option<String>,
}

/// Identity required before a cached sync plan can be reused by `stck push`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SyncPlanScope {
    repository: String,
    stack: Vec<PullRequest>,
    #[serde(default)]
    push_leases: Vec<RemoteBranchLease>,
}

impl SyncPlanScope {
    /// Capture the repository, ordered PR metadata, and remote tips for a sync plan.
    pub(crate) fn new(
        repository: &str,
        stack: &[PullRequest],
        push_leases: Vec<RemoteBranchLease>,
    ) -> Self {
        Self {
            repository: repository.to_string(),
            stack: stack.to_vec(),
            push_leases,
        }
    }

    /// Return whether this scope still describes the current repository stack.
    pub(crate) fn matches(&self, repository: &str, stack: &[PullRequest]) -> bool {
        self.repository == repository && self.stack == stack
    }

    /// Return the remote tips that must still match before rewritten pushes.
    pub(crate) fn push_leases(&self) -> &[RemoteBranchLease] {
        &self.push_leases
    }
}

impl LastSyncPlan {
    /// Return whether this cached plan is safe to reuse for the current stack.
    pub(crate) fn matches(
        &self,
        repository: &str,
        default_branch: &str,
        stack: &[PullRequest],
    ) -> bool {
        self.default_branch == default_branch
            && self
                .scope
                .as_ref()
                .is_some_and(|scope| scope.matches(repository, stack))
    }

    /// Return the sync-time remote tips associated with this cached plan.
    pub(crate) fn push_leases(&self) -> &[RemoteBranchLease] {
        self.scope
            .as_ref()
            .map(SyncPlanScope::push_leases)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum LastPlanState {
    Sync(SyncState),
    Push(PushState),
}

/// Load the current saved sync state, if one exists.
///
/// If a push state file is present instead, this returns an error because sync
/// and push state share the same persistence slot.
pub fn load_sync() -> Result<Option<SyncState>, String> {
    let path = state_file_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let state = load_raw_state(&path)?;
    match state {
        LastPlanState::Sync(sync) => Ok(Some(sync)),
        LastPlanState::Push(_) => Err(
            "push operation state is in progress; run `stck push` before starting a new sync"
                .to_string(),
        ),
    }
}

/// Persist sync progress for later `stck sync --continue` or `--reset` flows.
pub fn save_sync(state: &SyncState) -> Result<(), String> {
    save_raw_state(LastPlanState::Sync(state.clone()))
}

/// Load the current saved push state, if one exists.
///
/// If a sync state file is present instead, this returns an error because push
/// cannot proceed until the sync workflow is resolved.
pub fn load_push() -> Result<Option<PushState>, String> {
    let path = state_file_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let state = load_raw_state(&path)?;
    match state {
        LastPlanState::Push(push) => Ok(Some(push)),
        LastPlanState::Sync(_) => Err(
            "sync operation state is in progress; run `stck sync --continue` before running push"
                .to_string(),
        ),
    }
}

/// Persist push progress for later resume attempts.
pub fn save_push(state: &PushState) -> Result<(), String> {
    save_raw_state(LastPlanState::Push(state.clone()))
}

/// Remove any saved sync or push state file.
pub fn clear() -> Result<(), String> {
    let path = state_file_path()?;
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(&path).map_err(|_| format!("failed to remove sync state at {}", path.display()))
}

/// Load the cached retarget plan from the last successful sync run.
pub fn load_last_sync_plan() -> Result<Option<LastSyncPlan>, String> {
    let path = last_sync_plan_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read(&path).map_err(|_| format!("failed to read state at {}", path.display()))?;
    let plan = serde_json::from_slice::<LastSyncPlan>(&raw)
        .map_err(|_| format!("failed to parse state at {}", path.display()))?;
    Ok(Some(plan))
}

/// Persist the retarget plan generated by the last successful sync run.
pub fn save_last_sync_plan(plan: &LastSyncPlan) -> Result<(), String> {
    let path = last_sync_plan_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| "failed to compute parent directory for state file".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|_| format!("failed to create state directory {}", parent.display()))?;

    let raw = serde_json::to_vec_pretty(plan)
        .map_err(|_| "failed to serialize sync plan state".to_string())?;
    fs::write(&path, raw).map_err(|_| format!("failed to write state at {}", path.display()))
}

/// Remove the cached retarget plan from the last successful sync run.
pub fn clear_last_sync_plan() -> Result<(), String> {
    let path = last_sync_plan_path()?;
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(&path).map_err(|_| format!("failed to remove sync state at {}", path.display()))
}

/// Return the path to the shared sync/push state file under `.git/stck/`.
pub fn state_file_path() -> Result<PathBuf, String> {
    Ok(gitops::git_dir()?.join("stck").join("last-plan.json"))
}

/// Return the path to the cached last-sync plan file under `.git/stck/`.
pub fn last_sync_plan_path() -> Result<PathBuf, String> {
    Ok(gitops::git_dir()?.join("stck").join("last-sync-plan.json"))
}

fn load_raw_state(path: &PathBuf) -> Result<LastPlanState, String> {
    let raw = fs::read(path).map_err(|_| format!("failed to read state at {}", path.display()))?;
    serde_json::from_slice::<LastPlanState>(&raw)
        .map_err(|_| format!("failed to parse state at {}", path.display()))
}

fn save_raw_state(state: LastPlanState) -> Result<(), String> {
    let path = state_file_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| "failed to compute parent directory for state file".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|_| format!("failed to create state directory {}", parent.display()))?;

    let raw = serde_json::to_vec_pretty(&state)
        .map_err(|_| "failed to serialize operation state".to_string())?;
    fs::write(&path, raw).map_err(|_| format!("failed to write state at {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::PrState;
    use crate::stack::{RetargetStep, SyncStep};

    fn stack() -> Vec<PullRequest> {
        vec![
            PullRequest {
                number: 101,
                head_ref_name: "feature-b".to_string(),
                base_ref_name: "main".to_string(),
                state: PrState::Open,
            },
            PullRequest {
                number: 102,
                head_ref_name: "feature-c".to_string(),
                base_ref_name: "feature-b".to_string(),
                state: PrState::Open,
            },
        ]
    }

    fn scope() -> SyncPlanScope {
        SyncPlanScope::new(
            "example/stck",
            &stack(),
            vec![
                RemoteBranchLease {
                    branch: "feature-b".to_string(),
                    expected_remote_head: Some("bbbb1234".to_string()),
                },
                RemoteBranchLease {
                    branch: "feature-c".to_string(),
                    expected_remote_head: None,
                },
            ],
        )
    }

    #[test]
    fn sync_state_round_trip() {
        let state = SyncState {
            steps: vec![
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
            ],
            completed_steps: 1,
            failed_step: Some(1),
            failed_step_branch_head: Some("abcd1234".to_string()),
            plan_scope: Some(scope()),
        };

        let wrapped = LastPlanState::Sync(state.clone());
        let json = serde_json::to_vec_pretty(&wrapped).expect("serialize should succeed");
        let restored: LastPlanState =
            serde_json::from_slice(&json).expect("deserialize should succeed");

        match restored {
            LastPlanState::Sync(s) => {
                assert_eq!(s.steps.len(), 2);
                assert_eq!(s.steps[0].branch, "feature-b");
                assert_eq!(s.steps[1].new_base_ref, "feature-b");
                assert_eq!(s.completed_steps, 1);
                assert_eq!(s.failed_step, Some(1));
                assert_eq!(s.failed_step_branch_head, Some("abcd1234".to_string()));
                assert_eq!(s.plan_scope, Some(scope()));
            }
            LastPlanState::Push(_) => panic!("expected Sync variant"),
        }
    }

    #[test]
    fn push_state_round_trip() {
        let state = PushState {
            push_branches: vec!["feature-b".to_string(), "feature-c".to_string()],
            completed_pushes: 1,
            sync_push_leases: scope().push_leases().to_vec(),
            retargets: vec![RetargetStep {
                branch: "feature-b".to_string(),
                new_base_ref: "main".to_string(),
            }],
            completed_retargets: 0,
        };

        let wrapped = LastPlanState::Push(state.clone());
        let json = serde_json::to_vec_pretty(&wrapped).expect("serialize should succeed");
        let restored: LastPlanState =
            serde_json::from_slice(&json).expect("deserialize should succeed");

        match restored {
            LastPlanState::Push(p) => {
                assert_eq!(p.push_branches, vec!["feature-b", "feature-c"]);
                assert_eq!(p.completed_pushes, 1);
                assert_eq!(p.sync_push_leases, scope().push_leases());
                assert_eq!(p.retargets.len(), 1);
                assert_eq!(p.retargets[0].branch, "feature-b");
                assert_eq!(p.completed_retargets, 0);
            }
            LastPlanState::Sync(_) => panic!("expected Push variant"),
        }
    }

    #[test]
    fn last_sync_plan_round_trip() {
        let plan = LastSyncPlan {
            default_branch: "main".to_string(),
            scope: Some(scope()),
            retargets: vec![
                RetargetStep {
                    branch: "feature-b".to_string(),
                    new_base_ref: "main".to_string(),
                },
                RetargetStep {
                    branch: "feature-c".to_string(),
                    new_base_ref: "feature-b".to_string(),
                },
            ],
        };

        let json = serde_json::to_vec_pretty(&plan).expect("serialize should succeed");
        let restored: LastSyncPlan =
            serde_json::from_slice(&json).expect("deserialize should succeed");

        assert_eq!(restored.default_branch, "main");
        assert_eq!(restored.scope, Some(scope()));
        assert_eq!(restored.retargets.len(), 2);
        assert_eq!(restored.retargets[0].branch, "feature-b");
        assert_eq!(restored.retargets[1].new_base_ref, "feature-b");
    }

    #[test]
    fn kind_tag_distinguishes_sync_from_push() {
        let sync = LastPlanState::Sync(SyncState {
            steps: vec![],
            completed_steps: 0,
            failed_step: None,
            failed_step_branch_head: None,
            plan_scope: Some(scope()),
        });
        let push = LastPlanState::Push(PushState {
            push_branches: vec![],
            completed_pushes: 0,
            sync_push_leases: vec![],
            retargets: vec![],
            completed_retargets: 0,
        });

        let sync_json = serde_json::to_string(&sync).expect("serialize sync");
        let push_json = serde_json::to_string(&push).expect("serialize push");

        assert!(sync_json.contains(r#""kind":"sync""#));
        assert!(push_json.contains(r#""kind":"push""#));

        // Deserializing sync JSON yields Sync variant
        let restored_sync: LastPlanState =
            serde_json::from_str(&sync_json).expect("deserialize sync");
        assert!(matches!(restored_sync, LastPlanState::Sync(_)));

        // Deserializing push JSON yields Push variant
        let restored_push: LastPlanState =
            serde_json::from_str(&push_json).expect("deserialize push");
        assert!(matches!(restored_push, LastPlanState::Push(_)));
    }

    #[test]
    fn sync_state_with_no_failed_step_round_trips() {
        let state = SyncState {
            steps: vec![SyncStep {
                branch: "feature-b".to_string(),
                old_base_ref: "feature-a".to_string(),
                new_base_ref: "main".to_string(),
            }],
            completed_steps: 1,
            failed_step: None,
            failed_step_branch_head: None,
            plan_scope: Some(scope()),
        };

        let wrapped = LastPlanState::Sync(state);
        let json = serde_json::to_vec_pretty(&wrapped).expect("serialize should succeed");
        let restored: LastPlanState =
            serde_json::from_slice(&json).expect("deserialize should succeed");

        match restored {
            LastPlanState::Sync(s) => {
                assert_eq!(s.completed_steps, 1);
                assert_eq!(s.failed_step, None);
                assert_eq!(s.failed_step_branch_head, None);
            }
            LastPlanState::Push(_) => panic!("expected Sync variant"),
        }
    }

    #[test]
    fn last_sync_plan_matches_only_the_exact_repository_stack() {
        let plan = LastSyncPlan {
            default_branch: "main".to_string(),
            scope: Some(scope()),
            retargets: vec![],
        };

        assert!(plan.matches("example/stck", "main", &stack()));
        assert!(!plan.matches("other/stck", "main", &stack()));
        assert!(!plan.matches("example/stck", "trunk", &stack()));

        let mut changed_stack = stack();
        changed_stack[1].number = 999;
        assert!(!plan.matches("example/stck", "main", &changed_stack));
    }

    #[test]
    fn legacy_unscoped_last_sync_plan_is_not_reusable() {
        let raw = r#"{
  "default_branch": "main",
  "retargets": [
    {"branch": "feature-b", "new_base_ref": "main"}
  ]
}"#;

        let plan: LastSyncPlan =
            serde_json::from_str(raw).expect("legacy cached plan should remain readable");

        assert_eq!(plan.scope, None);
        assert!(!plan.matches("example/stck", "main", &stack()));
    }
}

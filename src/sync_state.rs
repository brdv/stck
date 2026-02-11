use crate::gitops;
use crate::stack::{RetargetStep, SyncStep};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub steps: Vec<SyncStep>,
    pub completed_steps: usize,
    pub failed_step: Option<usize>,
    #[serde(default)]
    pub failed_step_branch_head: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushState {
    pub push_branches: Vec<String>,
    pub completed_pushes: usize,
    pub retargets: Vec<RetargetStep>,
    pub completed_retargets: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum LastPlanState {
    Sync(SyncState),
    Push(PushState),
}

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

pub fn save_sync(state: &SyncState) -> Result<(), String> {
    save_raw_state(LastPlanState::Sync(state.clone()))
}

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

pub fn save_push(state: &PushState) -> Result<(), String> {
    save_raw_state(LastPlanState::Push(state.clone()))
}

pub fn clear() -> Result<(), String> {
    let path = state_file_path()?;
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(&path).map_err(|_| format!("failed to remove sync state at {}", path.display()))
}

pub fn state_file_path() -> Result<PathBuf, String> {
    Ok(gitops::git_dir()?.join("stck").join("last-plan.json"))
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

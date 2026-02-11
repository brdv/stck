use crate::gitops;
use crate::stack::SyncStep;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub steps: Vec<SyncStep>,
    pub completed_steps: usize,
    pub failed_step: Option<usize>,
}

pub fn load() -> Result<Option<SyncState>, String> {
    let path = state_file_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let raw =
        fs::read(&path).map_err(|_| format!("failed to read sync state at {}", path.display()))?;
    serde_json::from_slice::<SyncState>(&raw)
        .map(Some)
        .map_err(|_| format!("failed to parse sync state at {}", path.display()))
}

pub fn save(state: &SyncState) -> Result<(), String> {
    let path = state_file_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| "failed to compute parent directory for sync state".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|_| format!("failed to create sync state directory {}", parent.display()))?;

    let raw = serde_json::to_vec_pretty(state)
        .map_err(|_| "failed to serialize sync state".to_string())?;
    fs::write(&path, raw).map_err(|_| format!("failed to write sync state at {}", path.display()))
}

pub fn remove() -> Result<(), String> {
    let path = state_file_path()?;
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(&path).map_err(|_| format!("failed to remove sync state at {}", path.display()))
}

pub fn state_file_path() -> Result<PathBuf, String> {
    Ok(gitops::git_dir()?.join("stck").join("last-plan.json"))
}

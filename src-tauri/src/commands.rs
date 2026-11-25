use crate::app_state::AppState;
use crate::config::Config;
use crate::restic_cmd::SnapshotRow;
use crate::restic_ls::DirEntry;
use crate::{restic_actions, restic_bin, restic_cmd, restic_killable, restic_ls};
use serde::Serialize;
use std::collections::HashMap;
use std::process::Command;
use std::sync::OnceLock;
use tauri::State;

static CONFIG: OnceLock<Result<Config, String>> = OnceLock::new();

pub(crate) fn cached_config() -> Result<&'static Config, String> {
    match CONFIG.get_or_init(crate::config::load_config) {
        Ok(cfg) => Ok(cfg),
        Err(e) => Err(e.clone()),
    }
}

#[derive(Serialize)]
pub struct RepoName {
    name: String,
}

#[tauri::command]
pub fn list_repos() -> Result<Vec<RepoName>, String> {
    let cfg = cached_config()?;
    Ok(cfg
        .repos
        .iter()
        .map(|r| RepoName {
            name: r.name.clone(),
        })
        .collect())
}

#[tauri::command]
pub async fn list_snapshots(
    state: State<'_, AppState>,
    repo_name: String,
) -> Result<Vec<SnapshotRow>, String> {
    let cache = state.tree_cache.clone();
    let generation = cache.begin_list_snapshots(&repo_name);
    state.prefetch.clear();
    let disk_on = cached_config()
        .ok()
        .and_then(|c| c.cache_dir.as_ref().map(|s| !s.trim().is_empty()))
        .unwrap_or(false);
    let rows = tauri::async_runtime::spawn_blocking({
        let repo_name = repo_name.clone();
        move || {
            let cfg = cached_config()?;
            let rows = restic_cmd::list_snapshots(cfg, &repo_name)?;
            let repo_id = if disk_on {
                crate::restic_tree::restic_repository_id(cfg, &repo_name).ok()
            } else {
                None
            };
            Ok::<_, String>((rows, repo_id))
        }
    })
    .await
    .map_err(|e| format!("list_snapshots task failed: {e}"))??;
    let (rows, repo_id) = rows;
    if cache.generation() != generation {
        return Ok(rows);
    }
    let _ = cache.set_session_repo_id(generation, repo_id);
    let roots: HashMap<_, _> = rows.iter().map(|s| (s.id.clone(), s.tree.clone())).collect();
    if cache.remember_snapshots(generation, roots) {
        state.prefetch.enqueue_newest_root(generation, &repo_name, &rows);
    }
    Ok(rows)
}

#[tauri::command]
pub async fn list_dir(
    state: State<'_, AppState>,
    repo_name: String,
    snapshot_id: String,
    path: String,
) -> Result<Vec<DirEntry>, String> {
    let cache = state.tree_cache.clone();
    let prefetch = state.prefetch.clone();
    let generation = cache.generation();
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = cached_config()?;
        if cache.generation() != generation {
            return Err("CANCELLED: stale cache generation".into());
        }
        let entries =
            restic_ls::list_dir(cfg, &cache, generation, &repo_name, &snapshot_id, &path)?;
        if let Some(tid) = crate::tree_walk::cached_tree_id_for_path(&cache, &snapshot_id, &path) {
            if let Some(listing) = cache.get_cached(&tid) {
                prefetch.enqueue_child_subtrees(generation, &repo_name, &listing);
            }
        }
        Ok(entries)
    })
    .await
    .map_err(|e| format!("list_dir task failed: {e}"))?
}

#[tauri::command]
pub fn restic_version() -> Result<String, String> {
    let cfg = cached_config().ok();
    let bin = restic_bin::resolve_restic(cfg)?;
    let output = Command::new(&bin)
        .arg("version")
        .output()
        .map_err(|e| format!("failed to run restic: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub fn begin_action(action_id: String) -> Result<(), String> {
    restic_killable::begin_action(action_id)
}

#[tauri::command]
pub fn cancel_action(action_id: String) -> Result<(), String> {
    restic_killable::cancel_action(action_id)
}

include!("commands_actions.in.rs");

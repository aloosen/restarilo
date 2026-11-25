use crate::app_state::AppState;
use crate::commands::cached_config;
use crate::folder_index::FolderChild;
use crate::folder_index_emit::ScanStatusKind;
use crate::restic_cmd::SnapshotRow;
use crate::restic_ls::DirEntry;
use crate::{restic_cmd, restic_ls};
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn list_snapshots(
    state: State<'_, AppState>,
    repo_name: String,
) -> Result<Vec<SnapshotRow>, String> {
    let cache = state.tree_cache.clone();
    let generation = cache.begin_list_snapshots(&repo_name);
    state.scan.on_generation_bump(generation);
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
    if let Some(row) = crate::tree_prefetch::newest_snapshot(&rows) {
        state.scan.set_identity(&repo_name, &row.id);
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
    let scan = state.scan.clone();
    let index = state.folder_index.clone();
    let generation = cache.generation();
    tauri::async_runtime::spawn_blocking(move || {
        prefetch.user_enter();
        scan.scan_user_enter();
        let result = (|| {
            let cfg = cached_config()?;
            if cache.generation() != generation {
                return Err("CANCELLED: stale cache generation".into());
            }
            let entries =
                restic_ls::list_dir(cfg, &cache, generation, &repo_name, &snapshot_id, &path)?;
            let dirs: Vec<FolderChild> = entries
                .iter()
                .filter(|e| e.is_dir)
                .map(|e| FolderChild {
                    name: e.name.clone(),
                    path: e.path.clone(),
                })
                .collect();
            index.upsert_dirs(
                generation,
                &repo_name,
                &snapshot_id,
                &path,
                &dirs,
                true,
            );
            scan.emit_dirs(
                generation,
                &repo_name,
                &snapshot_id,
                &path,
                &dirs,
                true,
            );
            if let Some(tid) =
                crate::tree_walk::cached_tree_id_for_path(&cache, &snapshot_id, &path)
            {
                if let Some(listing) = cache.get_cached(&tid) {
                    prefetch.enqueue_child_subtrees(generation, &repo_name, &listing);
                }
            }
            Ok(entries)
        })();
        scan.scan_user_leave();
        prefetch.user_leave();
        result
    })
    .await
    .map_err(|e| format!("list_dir task failed: {e}"))?
}

#[derive(Serialize)]
pub struct FolderDirsResult {
    pub dirs: Vec<FolderChild>,
    pub children_complete: bool,
    pub scan_status: ScanStatusKind,
}

#[tauri::command]
pub async fn list_folder_dirs(
    state: State<'_, AppState>,
    repo_name: String,
    snapshot_id: String,
    path: String,
    prioritize: bool,
) -> Result<FolderDirsResult, String> {
    let cache = state.tree_cache.clone();
    let scan = state.scan.clone();
    let index = state.folder_index.clone();
    let generation = cache.generation();
    if cache.generation() != generation {
        return Err("CANCELLED: stale cache generation".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        scan.scan_user_enter();
        let result = (|| {
            if cache.generation() != generation {
                return Err("CANCELLED: stale cache generation".into());
            }
            if prioritize {
                scan.prioritize(&path);
            }
            let mut complete = index
                .get(generation, &repo_name, &snapshot_id, &path)
                .map(|d| d.children_complete)
                .unwrap_or(false);
            if !complete {
                let cfg = cached_config()?;
                let entries =
                    restic_ls::list_dir(cfg, &cache, generation, &repo_name, &snapshot_id, &path)?;
                let dirs: Vec<FolderChild> = entries
                    .iter()
                    .filter(|e| e.is_dir)
                    .map(|e| FolderChild {
                        name: e.name.clone(),
                        path: e.path.clone(),
                    })
                    .collect();
                index.upsert_dirs(
                    generation,
                    &repo_name,
                    &snapshot_id,
                    &path,
                    &dirs,
                    true,
                );
                scan.emit_dirs(
                    generation,
                    &repo_name,
                    &snapshot_id,
                    &path,
                    &dirs,
                    true,
                );
                complete = true;
            }
            let dirs = index
                .get(generation, &repo_name, &snapshot_id, &path)
                .map(|d| d.dirs)
                .unwrap_or_default();
            Ok(FolderDirsResult {
                dirs,
                children_complete: complete,
                scan_status: scan.status(),
            })
        })();
        scan.scan_user_leave();
        if !prioritize {
            scan.maybe_idle_start();
        }
        result
    })
    .await
    .map_err(|e| format!("list_folder_dirs task failed: {e}"))?
}

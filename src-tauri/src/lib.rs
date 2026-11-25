mod config;
mod restic_actions;
mod restic_bin;
mod restic_cmd;
mod restic_killable;
mod restic_ls;
mod restic_tree;
mod ssh;

use config::Config;
use restic_cmd::SnapshotRow;
use restic_ls::DirEntry;
use serde::Serialize;
use std::process::Command;
use std::sync::OnceLock;

static CONFIG: OnceLock<Result<Config, String>> = OnceLock::new();

fn cached_config() -> Result<&'static Config, String> {
    match CONFIG.get_or_init(config::load_config) {
        Ok(cfg) => Ok(cfg),
        Err(e) => Err(e.clone()),
    }
}

#[derive(Serialize)]
struct RepoName {
    name: String,
}

#[tauri::command]
fn list_repos() -> Result<Vec<RepoName>, String> {
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
async fn list_snapshots(repo_name: String) -> Result<Vec<SnapshotRow>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = cached_config()?;
        restic_cmd::list_snapshots(cfg, &repo_name)
    })
    .await
    .map_err(|e| format!("list_snapshots task failed: {e}"))?
}

#[tauri::command]
async fn list_dir(
    repo_name: String,
    snapshot_id: String,
    path: String,
) -> Result<Vec<DirEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = cached_config()?;
        restic_ls::list_dir(cfg, &repo_name, &snapshot_id, &path)
    })
    .await
    .map_err(|e| format!("list_dir task failed: {e}"))?
}

#[tauri::command]
fn restic_version() -> Result<String, String> {
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
fn begin_action(action_id: String) -> Result<(), String> {
    restic_killable::begin_action(action_id)
}

#[tauri::command]
fn cancel_action(action_id: String) -> Result<(), String> {
    restic_killable::cancel_action(action_id)
}

#[tauri::command]
async fn restore_path(
    repo_name: String,
    snapshot_id: String,
    path: String,
    name: String,
    target_dir: String,
    overwrite: bool,
    action_id: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = cached_config()?;
        restic_actions::restore_path(
            cfg,
            &repo_name,
            &snapshot_id,
            &path,
            &name,
            &target_dir,
            overwrite,
            action_id,
        )
    })
    .await
    .map_err(|e| format!("restore_path task failed: {e}"))?
}

#[tauri::command]
async fn dump_path(
    repo_name: String,
    snapshot_id: String,
    path: String,
    name: String,
    is_dir: bool,
    target_dir: String,
    overwrite: bool,
    action_id: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = cached_config()?;
        restic_actions::dump_path(
            cfg,
            &repo_name,
            &snapshot_id,
            &path,
            &name,
            is_dir,
            &target_dir,
            overwrite,
            action_id,
        )
    })
    .await
    .map_err(|e| format!("dump_path task failed: {e}"))?
}

#[tauri::command]
async fn open_path(
    repo_name: String,
    snapshot_id: String,
    path: String,
    name: String,
    is_dir: bool,
    action_id: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = cached_config()?;
        restic_actions::open_path(
            cfg,
            &repo_name,
            &snapshot_id,
            &path,
            &name,
            is_dir,
            action_id,
        )
    })
    .await
    .map_err(|e| format!("open_path task failed: {e}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            restic_version,
            list_repos,
            list_snapshots,
            list_dir,
            begin_action,
            cancel_action,
            restore_path,
            dump_path,
            open_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

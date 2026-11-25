#[tauri::command]
pub async fn restore_path(
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
pub async fn dump_path(
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
pub async fn open_path(
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

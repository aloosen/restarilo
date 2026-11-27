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
pub async fn set_snapshot_tags(
    state: tauri::State<'_, crate::app_state::AppState>,
    repo_name: String,
    snapshot_id: String,
    tags: Vec<String>,
    previous_tags: Vec<String>,
    action_id: Option<String>,
) -> Result<restic_tag::TagChange, String> {
    let scan = state.scan.clone();
    let prefetch = state.prefetch.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let release_if_pending = |aid: &Option<String>| {
            if let Some(id) = aid {
                restic_killable::release_action(id);
            }
        };
        scan.cancel_running();
        let _scan = scan.scan_user_guard();
        let _prefetch = prefetch.user_guard();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while scan.is_running() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if scan.is_running() {
            release_if_pending(&action_id);
            return Err("background repository scan still running; try again".into());
        }
        let cfg = match cached_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                release_if_pending(&action_id);
                return Err(e);
            }
        };
        restic_tag::set_snapshot_tags(
            cfg,
            &repo_name,
            &snapshot_id,
            tags,
            previous_tags,
            action_id,
        )
    })
    .await
    .map_err(|e| format!("set_snapshot_tags task failed: {e}"))?
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

#[derive(serde::Serialize)]
pub struct FindPathsResult {
    matches: Vec<crate::restic_find::FindMatch>,
}

#[tauri::command]
pub async fn find_paths(
    repo_name: String,
    query: String,
    snapshot_id: Option<String>,
    action_id: Option<String>,
) -> Result<FindPathsResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let cfg = cached_config()?;
        let matches = crate::restic_find::find_paths(
            cfg,
            &repo_name,
            &query,
            snapshot_id.as_deref(),
            action_id,
        )?;
        Ok(FindPathsResult { matches })
    })
    .await
    .map_err(|e| format!("find_paths task failed: {e}"))?
}

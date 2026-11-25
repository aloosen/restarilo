use crate::commands::cached_config;
use crate::restic_cmd::find_repo;
use crate::smb_probe::{
    bump_share_epoch, current_share_epoch, is_stale, probe_repo_dir, PROBE_DEADLINE,
};
use crate::smb_signin::sign_in_share;
use crate::smb_unc::{share_root, ShareAccessStatus};
use tauri::WebviewWindow;

#[tauri::command]
pub async fn probe_share_access(repo_name: String) -> Result<ShareAccessStatus, String> {
    let epoch = bump_share_epoch();
    let path = {
        let cfg = cached_config()?;
        find_repo(cfg, &repo_name)?.path.clone()
    };
    let status = tauri::async_runtime::spawn_blocking(move || {
        let (status, echoed) = probe_repo_dir(&path, PROBE_DEADLINE, epoch);
        if is_stale(echoed) || echoed != current_share_epoch() {
            return ShareAccessStatus::CheckInProgress;
        }
        status
    })
    .await
    .map_err(|e| format!("probe_share_access task failed: {e}"))?;
    Ok(status)
}

#[tauri::command]
pub async fn sign_in_network_share(
    window: WebviewWindow,
    repo_name: String,
    prompt_alternate: bool,
) -> Result<ShareAccessStatus, String> {
    let epoch = bump_share_epoch();
    let path = {
        let cfg = cached_config()?;
        find_repo(cfg, &repo_name)?.path.clone()
    };
    let root = share_root(&path)?;
    let hwnd = window_hwnd(&window);
    let status = tauri::async_runtime::spawn_blocking(move || {
        let status = sign_in_share(&root, hwnd, prompt_alternate);
        if is_stale(epoch) {
            return ShareAccessStatus::CheckInProgress;
        }
        status
    })
    .await
    .map_err(|e| format!("sign_in_network_share task failed: {e}"))?;
    Ok(status)
}

#[cfg(windows)]
fn window_hwnd(window: &WebviewWindow) -> isize {
    window
        .hwnd()
        .map(|h| h.0 as isize)
        .unwrap_or(0)
}

#[cfg(not(windows))]
fn window_hwnd(_window: &WebviewWindow) -> isize {
    0
}

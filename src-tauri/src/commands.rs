use crate::config::Config;
use crate::{restic_actions, restic_bin, restic_killable};
use serde::Serialize;
use std::process::Command;
use std::sync::OnceLock;

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
pub fn restic_version() -> Result<String, String> {
    let cfg = cached_config().ok();
    let bin = restic_bin::resolve_restic(cfg)?;
    let mut cmd = Command::new(&bin);
    crate::restic_cmd::hide_console_window(&mut cmd);
    let output = cmd
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

mod config;
mod restic_bin;

use config::Config;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![restic_version, list_repos])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

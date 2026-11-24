use std::process::Command;

#[tauri::command]
fn restic_version() -> Result<String, String> {
    let output = Command::new("restic")
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
        .invoke_handler(tauri::generate_handler![restic_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

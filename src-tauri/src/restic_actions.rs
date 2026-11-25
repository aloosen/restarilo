use crate::config::Config;
use crate::restic_cmd::run_restic;
use std::path::{Path, PathBuf};

fn parent_snapshot_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return "/".into();
    }
    match trimmed.rfind('/') {
        None | Some(0) => "/".into(),
        Some(i) => trimmed[..i].to_string(),
    }
}

fn target_exists_err(path: &Path) -> String {
    format!("TARGET_EXISTS:{}", path.display())
}

fn ensure_writable(path: &Path, overwrite: bool, is_file: bool) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    if !overwrite {
        return Err(target_exists_err(path));
    }
    if is_file {
        std::fs::remove_file(path).map_err(|e| format!("failed to remove {}: {e}", path.display()))?;
    }
    Ok(())
}

pub fn restore_path(
    cfg: &Config,
    repo_name: &str,
    snapshot_id: &str,
    path: &str,
    name: &str,
    target_dir: &str,
    overwrite: bool,
) -> Result<String, String> {
    let chosen = PathBuf::from(target_dir);
    let dest = chosen.join(name);
    ensure_writable(&dest, overwrite, false)?;
    let parent = parent_snapshot_path(path);
    let snapshot_ref = format!("{snapshot_id}:{parent}");
    let include = format!("/{name}");
    let dest_s = chosen.to_string_lossy();
    run_restic(
        cfg,
        repo_name,
        &[
            "restore",
            &snapshot_ref,
            "--target",
            &dest_s,
            "--include",
            &include,
        ],
    )?;
    Ok(dest.to_string_lossy().into_owned())
}

pub fn dump_path(
    cfg: &Config,
    repo_name: &str,
    snapshot_id: &str,
    path: &str,
    name: &str,
    is_dir: bool,
    target_dir: &str,
    overwrite: bool,
) -> Result<String, String> {
    let out = if is_dir {
        PathBuf::from(target_dir).join(format!("{name}.zip"))
    } else {
        PathBuf::from(target_dir).join(name)
    };
    ensure_writable(&out, overwrite, true)?;
    let out_s = out.to_string_lossy();
    let mut args: Vec<&str> = vec!["dump"];
    if is_dir {
        args.push("-a");
        args.push("zip");
    }
    args.extend(["--target", &out_s, snapshot_id, path]);
    run_restic(cfg, repo_name, &args)?;
    Ok(out.to_string_lossy().into_owned())
}

pub fn open_path(
    cfg: &Config,
    repo_name: &str,
    snapshot_id: &str,
    path: &str,
    name: &str,
    is_dir: bool,
) -> Result<String, String> {
    if is_dir {
        return Err(format!("OPEN_DIRECTORY:{path}"));
    }
    let temp = std::env::temp_dir().join(format!(
        "restarilo-open-{}-{name}",
        std::process::id()
    ));
    if temp.exists() {
        std::fs::remove_file(&temp)
            .map_err(|e| format!("failed to clear temp {}: {e}", temp.display()))?;
    }
    let temp_s = temp.to_string_lossy();
    run_restic(
        cfg,
        repo_name,
        &["dump", "--target", &temp_s, snapshot_id, path],
    )?;
    open::that(&temp).map_err(|e| format!("failed to open {}: {e}", temp.display()))?;
    Ok(temp.to_string_lossy().into_owned())
}

use crate::config::{expand_tilde, exe_dir, Config};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn resolve_restic(cfg: Option<&Config>) -> Result<PathBuf, String> {
    if let Some(bin) = cfg.and_then(|c| c.restic.binary.as_ref()) {
        let expanded = expand_tilde(bin);
        let path = PathBuf::from(&expanded);
        if !path.is_absolute() {
            return Err(format!(
                "[restic].binary must be absolute after ~ expansion, got '{expanded}'"
            ));
        }
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "configured restic binary not found: {}",
            path.display()
        ));
    }

    let dir = exe_dir()?;
    let local = local_restic_name(&dir);
    if local.is_file() {
        return Ok(local);
    }

    which_restic().ok_or_else(|| {
        format!(
            "restic not found next to executable ({}) or on PATH",
            dir.display()
        )
    })
}

fn local_restic_name(exe_dir: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        exe_dir.join("restic.exe")
    }
    #[cfg(not(windows))]
    {
        exe_dir.join("restic")
    }
}

#[cfg(not(windows))]
fn which_restic() -> Option<PathBuf> {
    let output = Command::new("sh")
        .args(["-c", "command -v restic"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

#[cfg(windows)]
fn which_restic() -> Option<PathBuf> {
    let output = Command::new("where").arg("restic").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let first = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .trim()
        .to_string();
    if first.is_empty() {
        None
    } else {
        Some(PathBuf::from(first))
    }
}

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub fn target_exists_err(path: &Path) -> String {
    format!("TARGET_EXISTS:{}", path.display())
}

#[cfg(test)]
thread_local! {
    static FAIL_PRE_FINAL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static FORCE_SIDECAR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub fn set_fail_pre_final_place(fail: bool) {
    FAIL_PRE_FINAL.with(|c| c.set(fail));
}

#[cfg(test)]
pub fn set_force_sidecar(force: bool) {
    FORCE_SIDECAR.with(|c| c.set(force));
}

fn inject_pre_final_place() -> Result<(), String> {
    #[cfg(test)]
    {
        if FAIL_PRE_FINAL.with(|c| c.replace(false)) {
            return Err("test: injected pre-final-place failure".into());
        }
    }
    Ok(())
}

static SIDECAR_SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_sidecar(out: &Path) -> Result<PathBuf, String> {
    let parent = out
        .parent()
        .ok_or_else(|| format!("dump target has no parent: {}", out.display()))?;
    let pid = std::process::id();
    for _ in 0..64 {
        let n = SIDECAR_SEQ.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(".restarilo-dump-replace-{pid}-{n}"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(f) => {
                drop(f);
                let _ = std::fs::remove_file(&candidate);
                return Ok(candidate);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(format!(
                    "failed to reserve replace sidecar {}: {e}",
                    candidate.display()
                ));
            }
        }
    }
    Err("failed to allocate unique dump replace sidecar".into())
}

fn cleanup_temp(temp: &Path) {
    let _ = std::fs::remove_file(temp);
}

fn place_when_absent(temp: &Path, out: &Path) -> Result<(), String> {
    inject_pre_final_place().map_err(|e| {
        cleanup_temp(temp);
        e
    })?;
    std::fs::rename(temp, out).map_err(|e| {
        cleanup_temp(temp);
        format!("failed to move dump into place {}: {e}", out.display())
    })
}

fn replace_via_sidecar(temp: &Path, out: &Path) -> Result<(), String> {
    let sidecar = unique_sidecar(out)?;
    std::fs::rename(out, &sidecar).map_err(|e| {
        cleanup_temp(temp);
        format!("failed to park existing {}: {e}", out.display())
    })?;
    if let Err(e) = inject_pre_final_place() {
        let _ = std::fs::rename(&sidecar, out);
        cleanup_temp(temp);
        return Err(e);
    }
    if let Err(e) = std::fs::rename(temp, out) {
        let _ = std::fs::rename(&sidecar, out);
        cleanup_temp(temp);
        return Err(format!(
            "failed to move dump into place {}: {e}",
            out.display()
        ));
    }
    let _ = std::fs::remove_file(&sidecar);
    Ok(())
}

fn replace_existing(temp: &Path, out: &Path) -> Result<(), String> {
    #[cfg(test)]
    if FORCE_SIDECAR.with(|c| c.get()) {
        return replace_via_sidecar(temp, out);
    }
    #[cfg(unix)]
    {
        inject_pre_final_place().map_err(|e| {
            cleanup_temp(temp);
            e
        })?;
        std::fs::rename(temp, out).map_err(|e| {
            cleanup_temp(temp);
            format!("failed to move dump into place {}: {e}", out.display())
        })
    }
    #[cfg(not(unix))]
    {
        replace_via_sidecar(temp, out)
    }
}

/// Move a staged dump from `temp` to `out`, respecting `overwrite`.
pub fn publish_dump(temp: &Path, out: &Path, overwrite: bool) -> Result<(), String> {
    if out.exists() {
        if !overwrite {
            cleanup_temp(temp);
            return Err(target_exists_err(out));
        }
        return replace_existing(temp, out);
    }
    place_when_absent(temp, out)
}

#[cfg(test)]
#[path = "dump_publish_tests.rs"]
mod tests;

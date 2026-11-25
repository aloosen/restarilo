//! Opt-in file debug log for diagnosis.
//! Path: `%LOCALAPPDATA%/restarilo/debug.log` (or XDG data/cache on Linux).
//! Call [`log`] from Rust or `folder_debug_fe` from the UI; nothing writes until then.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

struct LogState {
    file: File,
    path: PathBuf,
    start: Instant,
}

static LOG: OnceLock<Mutex<Option<LogState>>> = OnceLock::new();

fn log_mutex() -> &'static Mutex<Option<LogState>> {
    LOG.get_or_init(|| Mutex::new(None))
}

pub fn log_path() -> PathBuf {
    let base = dirs::data_local_dir()
        .or_else(dirs::cache_dir)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
        });
    base.join("restarilo").join("debug.log")
}

/// Truncate and write a session header.
pub fn init_session() {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut file = match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(
        file,
        "=== restarilo debug session start ===\npath={}\npid={}\n",
        path.display(),
        std::process::id()
    );
    let _ = file.flush();
    if let Ok(mut g) = log_mutex().lock() {
        *g = Some(LogState {
            file,
            path,
            start: Instant::now(),
        });
    }
}

fn ensure_open(g: &mut Option<LogState>) -> Option<&mut LogState> {
    if g.is_some() {
        return g.as_mut();
    }
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;
    let _ = writeln!(
        file,
        "=== restarilo debug (lazy open) ===\npath={}\npid={}\n",
        path.display(),
        std::process::id()
    );
    *g = Some(LogState {
        file,
        path,
        start: Instant::now(),
    });
    g.as_mut()
}

pub fn log(component: &str, msg: &str) {
    let Ok(mut guard) = log_mutex().lock() else {
        return;
    };
    let Some(state) = ensure_open(&mut guard) else {
        return;
    };
    let elapsed_ms = state.start.elapsed().as_millis();
    let wall_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let _ = writeln!(
        state.file,
        "{elapsed_ms:>8}ms wall={wall_ms} [{component}] {msg}"
    );
    let _ = state.file.flush();
}

pub fn path_string() -> String {
    if let Ok(g) = log_mutex().lock() {
        if let Some(state) = g.as_ref() {
            return state.path.display().to_string();
        }
    }
    log_path().display().to_string()
}

#[tauri::command]
pub fn folder_debug_path() -> String {
    init_session_if_needed();
    path_string()
}

#[tauri::command]
pub fn folder_debug_fe(message: String) {
    init_session_if_needed();
    log("fe", &message);
}

fn init_session_if_needed() {
    let needs = log_mutex()
        .lock()
        .map(|g| g.is_none())
        .unwrap_or(true);
    if needs {
        init_session();
    }
}

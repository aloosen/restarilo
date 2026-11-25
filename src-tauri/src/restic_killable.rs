use crate::config::Config;
use crate::restic_cmd::prepare_restic_command;
use std::io::Read;
use std::process::{Child, ExitStatus, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const POLL_MS: u64 = 20;

enum Slot {
    Pending { id: String },
    Running { id: String, child: Child },
}

static ACTIVE: OnceLock<Mutex<Option<Slot>>> = OnceLock::new();

fn active() -> &'static Mutex<Option<Slot>> {
    ACTIVE.get_or_init(|| Mutex::new(None))
}

pub fn begin_action(action_id: String) -> Result<(), String> {
    if action_id.is_empty() {
        return Err("action id must not be empty".into());
    }
    let mut guard = active().lock().map_err(|_| "action lock poisoned")?;
    if guard.is_some() {
        return Err("another action is already in progress".into());
    }
    *guard = Some(Slot::Pending { id: action_id });
    Ok(())
}

pub fn cancel_action(action_id: String) -> Result<(), String> {
    let mut guard = active().lock().map_err(|_| "action lock poisoned")?;
    match guard.as_mut() {
        Some(Slot::Pending { id }) if *id == action_id => {
            *guard = None;
            Ok(())
        }
        Some(Slot::Running { id, child }) if *id == action_id => {
            let _ = child.kill();
            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn release_action(action_id: &str) {
    let Ok(mut guard) = active().lock() else {
        return;
    };
    let matches = match guard.as_ref() {
        Some(Slot::Pending { id }) | Some(Slot::Running { id, .. }) => id == action_id,
        None => false,
    };
    if !matches {
        return;
    }
    if let Some(Slot::Running { mut child, .. }) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Adopt Pending `action_id`, or create ephemeral Pending when `None`.
pub fn ensure_slot(action_id: Option<String>) -> Result<String, String> {
    match action_id {
        Some(id) => {
            let guard = active().lock().map_err(|_| "action lock poisoned")?;
            match guard.as_ref() {
                Some(Slot::Pending { id: pending }) if *pending == id => Ok(id),
                Some(Slot::Pending { .. }) => Err("pending action id mismatch".into()),
                Some(Slot::Running { .. }) => Err("action already running".into()),
                None => Err(format!("no pending action for id '{id}'")),
            }
        }
        None => {
            let id = format!("ephemeral-{}", std::process::id());
            begin_action(id.clone())?;
            Ok(id)
        }
    }
}

pub fn run_restic_killable(
    cfg: &Config,
    repo_name: &str,
    args: &[&str],
    action_id: &str,
) -> Result<String, String> {
    let mut cmd = prepare_restic_command(cfg, repo_name)?;
    cmd.args(args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("failed to spawn restic: {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    {
        let mut guard = active().lock().map_err(|_| "action lock poisoned")?;
        let ok = matches!(guard.as_ref(), Some(Slot::Pending { id }) if id == action_id);
        if !ok {
            let _ = child.kill();
            let _ = child.wait();
            return Err("CANCELLED: action was cancelled before spawn".into());
        }
        *guard = Some(Slot::Running {
            id: action_id.to_string(),
            child,
        });
    }

    let out_h = thread::spawn(move || read_pipe(stdout));
    let err_h = thread::spawn(move || read_pipe(stderr));
    let status = poll_exit(action_id)?;
    clear_running_slot(action_id);
    let stdout_b = out_h.join().unwrap_or_default();
    let stderr_b = err_h.join().unwrap_or_default();
    finish_output(status, &stdout_b, &stderr_b)
}

fn read_pipe(pipe: Option<impl Read>) -> Vec<u8> {
    let mut buf = Vec::new();
    if let Some(mut r) = pipe {
        let _ = r.read_to_end(&mut buf);
    }
    buf
}

fn poll_exit(action_id: &str) -> Result<ExitStatus, String> {
    loop {
        let exited = {
            let mut guard = active().lock().map_err(|_| "action lock poisoned")?;
            match guard.as_mut() {
                Some(Slot::Running { id, child }) if id == action_id => child.try_wait(),
                _ => return Err("CANCELLED: action cancelled".into()),
            }
        };
        match exited {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => thread::sleep(Duration::from_millis(POLL_MS)),
            Err(e) => return Err(format!("failed to wait for restic: {e}")),
        }
    }
}

fn clear_running_slot(action_id: &str) {
    if let Ok(mut guard) = active().lock() {
        if matches!(guard.as_ref(), Some(Slot::Running { id, .. }) if id == action_id) {
            *guard = None;
        }
    }
}

fn finish_output(status: ExitStatus, stdout_b: &[u8], stderr_b: &[u8]) -> Result<String, String> {
    if status.success() {
        return Ok(String::from_utf8_lossy(stdout_b).into_owned());
    }
    if killed_by_signal(&status) {
        return Err("CANCELLED: restic process killed".into());
    }
    let err = String::from_utf8_lossy(stderr_b);
    let out = String::from_utf8_lossy(stdout_b);
    let msg = if err.trim().is_empty() {
        out.trim().to_string()
    } else {
        err.trim().to_string()
    };
    Err(if msg.is_empty() {
        format!("restic exited with {status}")
    } else {
        msg
    })
}

fn killed_by_signal(status: &ExitStatus) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.signal().is_some()
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        false
    }
}

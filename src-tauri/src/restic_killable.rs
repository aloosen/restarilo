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
    Running { id: String, child: Child, cancelled: bool },
    Completing { id: String, cancelled: bool },
    Finishing { id: String },
}

static ACTIVE: OnceLock<Mutex<Option<Slot>>> = OnceLock::new();

fn active() -> &'static Mutex<Option<Slot>> {
    ACTIVE.get_or_init(|| Mutex::new(None))
}

fn same_id(s: &Slot, action_id: &str) -> bool {
    matches!(s, Slot::Pending { id } | Slot::Running { id, .. } | Slot::Completing { id, .. } | Slot::Finishing { id } if id == action_id)
}

pub fn begin_action(action_id: String) -> Result<(), String> {
    if action_id.is_empty() {
        return Err("action id must not be empty".into());
    }
    let mut g = active().lock().map_err(|_| "action lock poisoned")?;
    if g.is_some() {
        return Err("another action is already in progress".into());
    }
    *g = Some(Slot::Pending { id: action_id });
    Ok(())
}

pub fn cancel_action(action_id: String) -> Result<(), String> {
    let mut g = active().lock().map_err(|_| "action lock poisoned")?;
    match g.as_mut() {
        Some(Slot::Pending { id }) if *id == action_id => *g = None,
        Some(Slot::Running { id, child, cancelled }) if *id == action_id => {
            *cancelled = true;
            let _ = child.kill();
        }
        Some(Slot::Completing { id, cancelled }) if *id == action_id => *cancelled = true,
        _ => {}
    }
    Ok(())
}

pub fn release_action(action_id: &str) {
    let Ok(mut g) = active().lock() else { return };
    if !g.as_ref().is_some_and(|s| same_id(s, action_id)) {
        return;
    }
    if let Some(Slot::Running { mut child, .. }) = g.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub fn commit_finish(action_id: &str) -> Result<(), String> {
    let mut g = active().lock().map_err(|_| "action lock poisoned")?;
    match g.as_ref() {
        Some(Slot::Completing { id, cancelled: true }) if id == action_id => {
            Err("CANCELLED: cancelled before finish".into())
        }
        Some(Slot::Completing { id, cancelled: false }) if id == action_id => {
            *g = Some(Slot::Finishing { id: action_id.to_string() });
            Ok(())
        }
        None => Err("CANCELLED: action cancelled".into()),
        _ => Err("action not ready to finish".into()),
    }
}

pub fn ensure_slot(action_id: Option<String>) -> Result<String, String> {
    match action_id {
        Some(id) => {
            let g = active().lock().map_err(|_| "action lock poisoned")?;
            match g.as_ref() {
                Some(Slot::Pending { id: p }) if *p == id => Ok(id),
                Some(Slot::Pending { .. }) => Err("pending action id mismatch".into()),
                Some(Slot::Running { .. } | Slot::Completing { .. } | Slot::Finishing { .. }) => {
                    Err("action already running".into())
                }
                None => Err(format!("CANCELLED: no pending action for id '{id}'")),
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
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("failed to spawn restic: {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    {
        let mut g = active().lock().map_err(|_| "action lock poisoned")?;
        if !matches!(g.as_ref(), Some(Slot::Pending { id }) if id == action_id) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("CANCELLED: action was cancelled before spawn".into());
        }
        *g = Some(Slot::Running {
            id: action_id.to_string(),
            child,
            cancelled: false,
        });
    }
    let out_h = thread::spawn(move || read_pipe(stdout));
    let err_h = thread::spawn(move || read_pipe(stderr));
    let (status, at_exit) = poll_exit(action_id)?;
    let out_b = out_h.join().unwrap_or_default();
    let err_b = err_h.join().unwrap_or_default();
    let late = matches!(
        active().lock().ok().as_deref().and_then(|g| g.as_ref()),
        Some(Slot::Completing { id, cancelled: true }) if id == action_id
    );
    #[cfg(unix)]
    let signaled = {
        use std::os::unix::process::ExitStatusExt;
        status.signal().is_some()
    };
    #[cfg(not(unix))]
    let signaled = false;
    if at_exit || late || signaled {
        return Err("CANCELLED: restic process killed".into());
    }
    if status.success() {
        return Ok(String::from_utf8_lossy(&out_b).into_owned());
    }
    let err = String::from_utf8_lossy(&err_b);
    let out = String::from_utf8_lossy(&out_b);
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

fn read_pipe(pipe: Option<impl Read>) -> Vec<u8> {
    let mut buf = Vec::new();
    if let Some(mut r) = pipe {
        let _ = r.read_to_end(&mut buf);
    }
    buf
}

fn poll_exit(action_id: &str) -> Result<(ExitStatus, bool), String> {
    loop {
        let done = {
            let mut g = active().lock().map_err(|_| "action lock poisoned")?;
            match g.as_mut() {
                Some(Slot::Running { id, child, cancelled }) if id == action_id => {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            let c = *cancelled;
                            *g = Some(Slot::Completing {
                                id: action_id.to_string(),
                                cancelled: c,
                            });
                            Some(Ok((status, c)))
                        }
                        Ok(None) => None,
                        Err(e) => Some(Err(format!("failed to wait for restic: {e}"))),
                    }
                }
                _ => Some(Err("CANCELLED: action cancelled".into())),
            }
        };
        match done {
            Some(r) => return r,
            None => thread::sleep(Duration::from_millis(POLL_MS)),
        }
    }
}

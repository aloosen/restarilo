use crate::config::Config;
use crate::restic_cmd::{self, find_repo, SnapshotRow};
use crate::restic_tree;
use crate::smb_unc::is_unc_path;
use serde::Deserialize;
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const SHARE_TIMEOUT_PREFIX: &str = "SHARE_TIMEOUT:";
pub const SNAPSHOT_BUDGET: Duration = Duration::from_secs(60);
const POLL_MS: u64 = 20;
pub fn share_timeout_err(detail: &str) -> String {
    format!("{SHARE_TIMEOUT_PREFIX} {detail}")
}

/// UNC-on-Windows uses a shared budget; other paths keep untimed restic.
pub fn load_snapshot_rows(
    cfg: &Config,
    repo_name: &str,
    disk_on: bool,
) -> Result<(Vec<SnapshotRow>, Option<String>), String> {
    let unc = cfg!(windows) && is_unc_path(&find_repo(cfg, repo_name)?.path);
    if unc {
        return list_snapshots_with_budget(cfg, repo_name, disk_on, SNAPSHOT_BUDGET);
    }
    let rows = restic_cmd::list_snapshots(cfg, repo_name)?;
    let repo_id = if disk_on {
        restic_tree::restic_repository_id(cfg, repo_name).ok()
    } else {
        None
    };
    Ok((rows, repo_id))
}

fn list_snapshots_with_budget(
    cfg: &Config,
    repo_name: &str,
    disk_on: bool,
    budget: Duration,
) -> Result<(Vec<SnapshotRow>, Option<String>), String> {
    let start = Instant::now();
    let raw = run_restic_budget(cfg, repo_name, &["snapshots", "--json"], start, budget)?;
    let parsed = restic_cmd::parse_snapshot_rows_json(&raw)?;
    if !disk_on {
        return Ok((parsed, None));
    }
    if start.elapsed() >= budget {
        return Err(share_timeout_err("budget exhausted before cat config"));
    }
    let raw = run_restic_budget(cfg, repo_name, &["cat", "config"], start, budget)?;
    Ok((parsed, parse_repo_id(&raw).ok()))
}

fn run_restic_budget(
    cfg: &Config,
    repo_name: &str,
    args: &[&str],
    start: Instant,
    budget: Duration,
) -> Result<String, String> {
    let remaining = budget.saturating_sub(start.elapsed());
    if remaining.is_zero() {
        return Err(share_timeout_err("budget exhausted"));
    }
    let mut cmd = restic_cmd::prepare_restic_command(cfg, repo_name)?;
    cmd.args(args);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run_command_with_budget(cmd, remaining)
}

pub fn run_command_with_budget(mut cmd: Command, budget: Duration) -> Result<String, String> {
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn process: {e}"))?;
    #[cfg(windows)]
    let _job = assign_job(&mut child);
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_h = thread::spawn(move || read_pipe(stdout));
    let err_h = thread::spawn(move || read_pipe(stderr));
    let deadline = Instant::now() + budget;
    let status = loop {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(share_timeout_err("process killed after deadline"));
        }
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => thread::sleep(Duration::from_millis(POLL_MS)),
            Err(e) => return Err(format!("failed to wait: {e}")),
        }
    };
    let out_b = out_h.join().unwrap_or_default();
    let err_b = err_h.join().unwrap_or_default();
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
        format!("process exited with {status}")
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

#[derive(Deserialize)]
struct RepoConfigJson {
    #[serde(default)]
    id: String,
}

fn parse_repo_id(raw: &str) -> Result<String, String> {
    let parsed: RepoConfigJson = serde_json::from_str(raw.trim())
        .map_err(|e| format!("failed to parse restic config JSON: {e}"))?;
    if parsed.id.trim().is_empty() {
        return Err("restic config missing id".into());
    }
    Ok(parsed.id)
}

#[cfg(windows)]
struct JobHandle(windows::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Drop for JobHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
fn assign_job(child: &mut std::process::Child) -> Option<JobHandle> {
    use std::os::windows::io::AsRawHandle;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    unsafe {
        let job = CreateJobObjectW(None, PCWSTR::null()).ok()?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of_val(&info) as u32,
        )
        .is_err()
        {
            let _ = windows::Win32::Foundation::CloseHandle(job);
            return None;
        }
        let proc = HANDLE(child.as_raw_handle() as *mut _);
        if AssignProcessToJobObject(job, proc).is_err() {
            let _ = windows::Win32::Foundation::CloseHandle(job);
            return None;
        }
        Some(JobHandle(job))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_kills_sleeping_process() {
        assert!(share_timeout_err("x").starts_with(SHARE_TIMEOUT_PREFIX));
        let mut cmd = Command::new("sleep");
        cmd.arg("30")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let err = run_command_with_budget(cmd, Duration::from_millis(80)).unwrap_err();
        assert!(err.contains(SHARE_TIMEOUT_PREFIX), "got {err}");
    }
}

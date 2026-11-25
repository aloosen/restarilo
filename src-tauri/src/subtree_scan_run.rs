use crate::folder_index::{FolderChild, FolderIndex};
use crate::subtree_scan::{ScanStop, MAX_DIR_NODES};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const DEFAULT_WALL_TIME: Duration = Duration::from_secs(15);

pub struct SpawnOpts {
    pub wall_time: Duration,
    pub node_budget: usize,
    pub cancel: Option<Arc<AtomicBool>>,
}

impl Default for SpawnOpts {
    fn default() -> Self {
        Self {
            wall_time: DEFAULT_WALL_TIME,
            node_budget: MAX_DIR_NODES,
            cancel: None,
        }
    }
}

fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        // Avoid taskkill.exe — it is a console app and flashes a terminal.
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{
            OpenProcess, TerminateProcess, PROCESS_TERMINATE,
        };
        unsafe {
            if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                let _ = TerminateProcess(handle, 1);
                let _ = CloseHandle(handle);
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
}

/// Hard-kill on wall_time even if stdout is silent (kill unblocks `read_line`).
#[allow(dead_code)] // wired by scan_scheduler (sub-task B)
pub fn run_scan_command(
    mut child: Child,
    scan_root: &str,
    generation: u64,
    repo: &str,
    snapshot_id: &str,
    index: &FolderIndex,
    opts: SpawnOpts,
    mut on_batch: impl FnMut(&str, &[FolderChild]),
) -> ScanStop {
    let pid = child.id();
    let deadline = Instant::now() + opts.wall_time;
    let cancel = opts.cancel.clone();
    let timed_out = Arc::new(AtomicBool::new(false));
    let timed_flag = timed_out.clone();
    let cancel_w = cancel.clone();
    let watchdog = std::thread::spawn(move || {
        while Instant::now() < deadline {
            if cancel_w.as_ref().is_some_and(|c| c.load(Ordering::Relaxed)) {
                kill_pid(pid);
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        timed_flag.store(true, Ordering::Relaxed);
        kill_pid(pid);
    });

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            kill_pid(pid);
            let _ = child.wait();
            let _ = watchdog.join();
            return ScanStop::Cancelled;
        }
    };
    let mut stop = ScanStop::Eof;
    let mut nodes = 0usize;
    for line in BufReader::new(stdout).lines() {
        if cancel.as_ref().is_some_and(|c| c.load(Ordering::Relaxed)) {
            stop = ScanStop::Cancelled;
            break;
        }
        if timed_out.load(Ordering::Relaxed) {
            stop = ScanStop::WallTime;
            break;
        }
        let Ok(line) = line else {
            break;
        };
        let Some((parent, child_fc)) =
            crate::subtree_scan::edge_from_ls_line(&line, scan_root)
        else {
            continue;
        };
        index.upsert_dirs(
            generation,
            repo,
            snapshot_id,
            &parent,
            &[child_fc.clone()],
            false,
        );
        on_batch(&parent, &[child_fc]);
        nodes += 1;
        if nodes >= opts.node_budget {
            stop = ScanStop::NodeBudget;
            break;
        }
    }
    if timed_out.load(Ordering::Relaxed) {
        stop = ScanStop::WallTime;
    } else if cancel.as_ref().is_some_and(|c| c.load(Ordering::Relaxed)) {
        stop = ScanStop::Cancelled;
    }
    let _ = child.kill();
    let _ = child.wait();
    let _ = watchdog.join();
    stop
}

#[cfg(test)]
pub fn spawn_silent_deadline_test(wall: Duration) -> ScanStop {
    let mut cmd = Command::new("sleep");
    cmd.arg("60")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    let child = cmd.spawn().expect("spawn sleep");
    let index = FolderIndex::new();
    index.begin(1);
    run_scan_command(
        child,
        "/",
        1,
        "r",
        "s",
        &index,
        SpawnOpts {
            wall_time: wall,
            node_budget: MAX_DIR_NODES,
            cancel: None,
        },
        |_, _| {},
    )
}

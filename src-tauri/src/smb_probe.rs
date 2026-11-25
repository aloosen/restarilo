use crate::smb_unc::{
    is_unc_path, normalize_unc_path, status_from_raw_os_error, ShareAccessStatus,
};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub const PROBE_DEADLINE: Duration = Duration::from_secs(15);

static WORKER_BUSY: AtomicBool = AtomicBool::new(false);
static SHARE_EPOCH: AtomicU64 = AtomicU64::new(1);

pub fn bump_share_epoch() -> u64 {
    SHARE_EPOCH.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn current_share_epoch() -> u64 {
    SHARE_EPOCH.load(Ordering::SeqCst)
}

pub fn is_stale(echoed_epoch: u64) -> bool {
    echoed_epoch < current_share_epoch()
}

type Checker = Box<dyn FnOnce(&str) -> ShareAccessStatus + Send>;

/// Returns status + epoch used for the attempt. Callers discard when `is_stale(epoch)`.
pub fn probe_repo_dir(path: &str, deadline: Duration, epoch: u64) -> (ShareAccessStatus, u64) {
    probe_with_checker(path, deadline, epoch, Box::new(|p| default_check(p)))
}

fn probe_with_checker(
    path: &str,
    deadline: Duration,
    epoch: u64,
    checker: Checker,
) -> (ShareAccessStatus, u64) {
    if WORKER_BUSY
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return (ShareAccessStatus::CheckInProgress, epoch);
    }
    let path = path.to_string();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let status = checker(&path);
        let _ = tx.send(status);
        WORKER_BUSY.store(false, Ordering::SeqCst);
    });
    match rx.recv_timeout(deadline) {
        Ok(status) => (status, epoch),
        Err(mpsc::RecvTimeoutError::Timeout) => (ShareAccessStatus::Timeout, epoch),
        Err(mpsc::RecvTimeoutError::Disconnected) => (
            ShareAccessStatus::Unknown {
                raw_os_error: None,
                message: "probe worker disconnected".into(),
            },
            epoch,
        ),
    }
}

fn default_check(path: &str) -> ShareAccessStatus {
    if !is_unc_path(path) || !cfg!(windows) {
        return ShareAccessStatus::Ok;
    }
    let normalized = normalize_unc_path(path);
    match std::fs::metadata(&normalized) {
        Ok(_) => ShareAccessStatus::Ok,
        Err(e) => match e.raw_os_error() {
            Some(code) => status_from_raw_os_error(code),
            None => ShareAccessStatus::Unknown {
                raw_os_error: None,
                message: e.to_string(),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier, Mutex};
    use std::time::Instant;

    /// Probe tests share process-wide WORKER_BUSY; run them one at a time.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn wait_idle() {
        for _ in 0..200 {
            if !WORKER_BUSY.load(Ordering::SeqCst) {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("probe worker still busy");
    }

    #[test]
    fn blocked_probe_times_out_then_check_in_progress() {
        let _lock = TEST_LOCK.lock().unwrap();
        wait_idle();
        let epoch = bump_share_epoch();
        let release = Arc::new(Barrier::new(2));
        let release_w = release.clone();
        let start = Instant::now();
        let (status, e) = probe_with_checker(
            r"\\nas\backups\repo",
            Duration::from_millis(40),
            epoch,
            Box::new(move |_| {
                release_w.wait();
                ShareAccessStatus::Ok
            }),
        );
        assert!(start.elapsed() < Duration::from_millis(500));
        assert_eq!(status, ShareAccessStatus::Timeout);
        assert_eq!(e, epoch);

        let (again, _) = probe_with_checker(
            r"\\nas\backups\repo",
            Duration::from_millis(40),
            epoch,
            Box::new(|_| ShareAccessStatus::Ok),
        );
        assert_eq!(again, ShareAccessStatus::CheckInProgress);

        release.wait();
        wait_idle();
    }

    #[test]
    fn epoch_bump_discards_late_and_local_ok() {
        let _lock = TEST_LOCK.lock().unwrap();
        wait_idle();
        let epoch1 = bump_share_epoch();
        let release = Arc::new(Barrier::new(2));
        let release_w = release.clone();
        let (_status, echoed) = probe_with_checker(
            r"\\nas\backups\repo",
            Duration::from_millis(40),
            epoch1,
            Box::new(move |_| {
                release_w.wait();
                ShareAccessStatus::NeedsSignIn
            }),
        );
        assert_eq!(echoed, epoch1);
        let epoch2 = bump_share_epoch();
        assert!(is_stale(epoch1));
        assert!(!is_stale(epoch2));

        release.wait();
        wait_idle();

        let (local, e) = probe_repo_dir("/tmp/local-repo", Duration::from_secs(1), epoch2);
        assert_eq!(local, ShareAccessStatus::Ok);
        assert_eq!(e, epoch2);
        assert!(!is_stale(e));
    }
}

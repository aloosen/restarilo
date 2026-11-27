use crate::folder_index::FolderIndex;
use crate::folder_index_emit::{to_dtos, FolderIndexUpdated, ScanStatusKind};
use crate::folder_index::FolderChild;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[path = "scan_scheduler_lifecycle.rs"]
mod lifecycle;
pub use lifecycle::ScanJob;
use lifecycle::Inner;

pub const IDLE_MS: u64 = 300;
pub const COOLDOWN_BOUNDS: Duration = Duration::from_secs(60);
pub const COOLDOWN_FAILED: Duration = Duration::from_secs(30);
pub(crate) type EmitFn = Arc<dyn Fn(FolderIndexUpdated) + Send + Sync>;
type RunFn = Arc<dyn Fn(ScanJob) -> crate::subtree_scan::ScanStop + Send + Sync>;

pub struct ScanScheduler {
    pub index: Arc<FolderIndex>,
    activity: AtomicU32,
    inner: Mutex<Inner>,
    emit_webview: Mutex<Option<EmitFn>>,
    test_sink: Mutex<Option<EmitFn>>,
    runner: Mutex<Option<RunFn>>,
    pub cooldown_bounds: Mutex<Duration>,
    pub cooldown_failed: Mutex<Duration>,
}

impl ScanScheduler {
    pub fn new(index: Arc<FolderIndex>) -> Arc<Self> {
        Arc::new(Self {
            index,
            activity: AtomicU32::new(0),
            inner: Mutex::new(Inner {
                preferred_root: "/".into(),
                repo: String::new(),
                snapshot_id: String::new(),
                status: ScanStatusKind::Idle,
                running: false,
                pending_root: None,
                last: None,
                cancel: Arc::new(AtomicBool::new(false)),
                active: None,
            }),
            emit_webview: Mutex::new(None),
            test_sink: Mutex::new(None),
            runner: Mutex::new(None),
            cooldown_bounds: Mutex::new(COOLDOWN_BOUNDS),
            cooldown_failed: Mutex::new(COOLDOWN_FAILED),
        })
    }
    pub fn set_webview_emit(&self, f: Option<EmitFn>) {
        *self.emit_webview.lock().unwrap() = f;
    }
    pub fn set_test_sink(&self, f: Option<EmitFn>) {
        *self.test_sink.lock().unwrap() = f;
    }
    pub fn set_runner(&self, f: Option<RunFn>) {
        *self.runner.lock().unwrap() = f;
    }
    pub fn emit(&self, ev: FolderIndexUpdated) {
        if let Some(f) = self.test_sink.lock().unwrap().as_ref() {
            f(ev.clone());
        }
        if let Some(f) = self.emit_webview.lock().unwrap().as_ref() {
            f(ev);
        }
    }
    pub fn emit_dirs(
        &self,
        generation: u64,
        repo: &str,
        snapshot_id: &str,
        parent: &str,
        dirs: &[FolderChild],
        children_complete: bool,
    ) {
        let status = self.inner.lock().unwrap().status;
        self.emit(FolderIndexUpdated {
            repo_name: repo.into(),
            snapshot_id: snapshot_id.into(),
            generation,
            parent_path: parent.into(),
            dirs: to_dtos(dirs),
            children_complete,
            scan_status: status,
        });
    }
    pub fn scan_user_enter(&self) {
        self.activity.fetch_add(1, Ordering::SeqCst);
    }
    pub fn scan_user_leave(&self) {
        self.activity.fetch_sub(1, Ordering::SeqCst);
    }
    pub fn scan_user_guard(self: &Arc<Self>) -> ScanUserGuard {
        self.scan_user_enter();
        ScanUserGuard(self.clone())
    }
    pub fn activity(&self) -> u32 {
        self.activity.load(Ordering::SeqCst)
    }
    pub fn status(&self) -> ScanStatusKind {
        self.inner.lock().unwrap().status
    }
    pub fn set_identity(&self, repo: &str, snapshot_id: &str) {
        let mut g = self.inner.lock().unwrap();
        g.repo = repo.into();
        g.snapshot_id = snapshot_id.into();
        g.preferred_root = "/".into();
    }

    /// Stop a running background scan and drop any pending restart.
    pub fn cancel_running(&self) {
        let mut g = self.inner.lock().unwrap();
        g.cancel.store(true, Ordering::SeqCst);
        g.pending_root = None;
    }

    pub fn is_running(&self) -> bool {
        self.inner.lock().unwrap().running
    }
}

pub struct ScanUserGuard(Arc<ScanScheduler>);

impl Drop for ScanUserGuard {
    fn drop(&mut self) {
        self.0.scan_user_leave();
    }
}

#[cfg(test)]
#[path = "scan_scheduler_tests.rs"]
mod tests;

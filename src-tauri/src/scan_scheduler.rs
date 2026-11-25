use crate::folder_index::FolderIndex;
use crate::folder_index_emit::{to_dtos, FolderIndexUpdated, ScanStatusKind};
use crate::folder_index::FolderChild;
use crate::subtree_scan::ScanStop;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
pub const IDLE_MS: u64 = 300;
pub const COOLDOWN_BOUNDS: Duration = Duration::from_secs(60);
pub const COOLDOWN_FAILED: Duration = Duration::from_secs(30);
type EmitFn = Arc<dyn Fn(FolderIndexUpdated) + Send + Sync>;
type RunFn = Arc<dyn Fn(ScanJob) -> ScanStop + Send + Sync>;
#[derive(Clone)]
pub struct ScanJob {
    pub generation: u64,
    pub repo: String,
    pub snapshot_id: String,
    pub root: String,
    pub index: Arc<FolderIndex>,
    pub emit: EmitFn,
}
struct Finished {
    root: String,
    kind: ScanStatusKind,
    at: Instant,
}
struct Inner {
    preferred_root: String,
    repo: String,
    snapshot_id: String,
    status: ScanStatusKind,
    running: bool,
    pending_root: Option<String>,
    last: Option<Finished>,
    cancel: Arc<AtomicBool>,
}
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
    pub fn on_generation_bump(&self, generation: u64) {
        self.index.begin(generation);
        let mut g = self.inner.lock().unwrap();
        g.cancel.store(true, Ordering::SeqCst);
        g.running = false;
        g.status = ScanStatusKind::Idle;
        g.last = None;
        g.cancel = Arc::new(AtomicBool::new(false));
    }
    pub fn prioritize(self: &Arc<Self>, root: &str) {
        let root = crate::restic_ls::normalize_dir_path(root);
        let start_now = {
            let mut g = self.inner.lock().unwrap();
            g.preferred_root = root.clone();
            g.last = None;
            if g.running {
                g.cancel.store(true, Ordering::SeqCst);
                g.pending_root = Some(root.clone());
                false
            } else {
                true
            }
        };
        if start_now {
            self.start_scan(&root);
        }
    }
    pub fn maybe_idle_start(self: &Arc<Self>) {
        if self.activity() > 0 {
            return;
        }
        let root = {
            let g = self.inner.lock().unwrap();
            if g.running || g.repo.is_empty() || !self.rerun_ok(&g, &g.preferred_root) {
                return;
            }
            g.preferred_root.clone()
        };
        self.start_scan(&root);
    }
    fn rerun_ok(&self, g: &Inner, root: &str) -> bool {
        let Some(last) = &g.last else {
            return true;
        };
        if last.root != root {
            return true;
        }
        match last.kind {
            ScanStatusKind::Success => false,
            ScanStatusKind::StoppedBounds => {
                last.at.elapsed() >= *self.cooldown_bounds.lock().unwrap()
            }
            ScanStatusKind::Failed => last.at.elapsed() >= *self.cooldown_failed.lock().unwrap(),
            _ => true,
        }
    }
    fn start_scan(self: &Arc<Self>, root: &str) {
        let job = {
            let mut g = self.inner.lock().unwrap();
            if g.running || g.repo.is_empty() {
                return;
            }
            g.cancel.store(true, Ordering::SeqCst);
            g.cancel = Arc::new(AtomicBool::new(false));
            g.running = true;
            g.status = ScanStatusKind::Running;
            let me = self.clone();
            ScanJob {
                generation: self.index.generation(),
                repo: g.repo.clone(),
                snapshot_id: g.snapshot_id.clone(),
                root: root.to_string(),
                index: self.index.clone(),
                emit: Arc::new(move |ev| me.emit(ev)),
            }
        };
        let runner = self.runner.lock().unwrap().clone();
        let me = self.clone();
        std::thread::spawn(move || {
            let stop = runner.map(|r| r(job.clone())).unwrap_or(ScanStop::Eof);
            me.finish(job.root, stop);
        });
    }
    fn finish(self: &Arc<Self>, root: String, stop: ScanStop) {
        let pending = {
            let mut g = self.inner.lock().unwrap();
            g.running = false;
            let pending = g.pending_root.take();
            if matches!(stop, ScanStop::Cancelled) {
                g.status = ScanStatusKind::Idle;
            } else {
                let kind = match stop {
                    ScanStop::Eof => ScanStatusKind::Success,
                    ScanStop::NodeBudget | ScanStop::WallTime => ScanStatusKind::StoppedBounds,
                    ScanStop::Cancelled => ScanStatusKind::Idle,
                };
                g.status = kind;
                g.last = Some(Finished {
                    root,
                    kind,
                    at: Instant::now(),
                });
            }
            pending
        };
        if let Some(p) = pending {
            self.start_scan(&p);
        }
    }
}
#[cfg(test)]
#[path = "scan_scheduler_tests.rs"]
mod tests;

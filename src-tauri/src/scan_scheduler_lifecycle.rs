use super::{EmitFn, ScanScheduler};
use crate::folder_index::FolderIndex;
use crate::folder_index_emit::ScanStatusKind;
use crate::subtree_scan::ScanStop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct ScanJob {
    pub generation: u64,
    pub repo: String,
    pub snapshot_id: String,
    pub root: String,
    pub index: Arc<FolderIndex>,
    pub emit: EmitFn,
    pub cancel: Arc<AtomicBool>,
}

pub(super) struct Finished {
    pub(super) root: String,
    pub(super) kind: ScanStatusKind,
    pub(super) at: Instant,
}

pub(super) struct Inner {
    pub(super) preferred_root: String,
    pub(super) repo: String,
    pub(super) snapshot_id: String,
    pub(super) status: ScanStatusKind,
    pub(super) running: bool,
    pub(super) pending_root: Option<String>,
    pub(super) last: Option<Finished>,
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) active: Option<Arc<AtomicBool>>,
}

impl ScanScheduler {
    pub fn on_generation_bump(&self, generation: u64) {
        self.index.begin(generation);
        let mut g = self.inner.lock().unwrap();
        g.cancel.store(true, Ordering::SeqCst);
        g.pending_root = None;
        g.last = None;
        g.status = ScanStatusKind::Idle;
        g.cancel = Arc::new(AtomicBool::new(false));
    }

    pub fn prioritize(self: &Arc<Self>, repo: &str, snapshot_id: &str, root: &str) {
        let root = crate::restic_ls::normalize_dir_path(root);
        let start_now = {
            let mut g = self.inner.lock().unwrap();
            g.repo = repo.into();
            g.snapshot_id = snapshot_id.into();
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
            let cancel = Arc::new(AtomicBool::new(false));
            g.cancel = cancel.clone();
            g.active = Some(cancel.clone());
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
                cancel,
            }
        };
        let runner = self.runner.lock().unwrap().clone();
        let me = self.clone();
        std::thread::spawn(move || {
            let stop = runner.map(|r| r(job.clone())).unwrap_or(ScanStop::Eof);
            me.finish(job, stop);
        });
    }

    fn finish(self: &Arc<Self>, job: ScanJob, stop: ScanStop) {
        let pending = {
            let mut g = self.inner.lock().unwrap();
            let matching = Arc::ptr_eq(&job.cancel, &g.cancel);
            let is_active = g
                .active
                .as_ref()
                .is_some_and(|c| Arc::ptr_eq(c, &job.cancel));
            if matching {
                g.running = false;
                g.active = None;
                let pending = g.pending_root.take();
                if matches!(stop, ScanStop::Cancelled) {
                    g.status = ScanStatusKind::Idle;
                } else {
                    let kind = match stop {
                        ScanStop::Eof => ScanStatusKind::Success,
                        ScanStop::NodeBudget | ScanStop::WallTime => ScanStatusKind::StoppedBounds,
                        ScanStop::Failed => ScanStatusKind::Failed,
                        ScanStop::Cancelled => ScanStatusKind::Idle,
                    };
                    g.status = kind;
                    g.last = Some(Finished {
                        root: job.root,
                        kind,
                        at: Instant::now(),
                    });
                }
                pending
            } else if is_active {
                g.running = false;
                g.active = None;
                g.status = ScanStatusKind::Idle;
                g.pending_root.take()
            } else {
                None
            }
        };
        if let Some(p) = pending {
            self.start_scan(&p);
        }
    }
}

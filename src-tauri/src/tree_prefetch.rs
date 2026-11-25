use crate::restic_cmd::SnapshotRow;
use crate::tree_cache::{CachedListing, TreeCache};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

#[path = "tree_prefetch_time.rs"]
mod time;
pub use time::newest_snapshot;

const MAX_CHILD_PREFETCH: usize = 4;

struct State {
    queue: VecDeque<(u64, String, String)>,
    busy: bool,
    user_active: u32,
}

pub struct PrefetchQueue {
    state: Mutex<State>,
    wake: Condvar,
    cache: Arc<TreeCache>,
}

impl PrefetchQueue {
    pub fn new(cache: Arc<TreeCache>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                queue: VecDeque::new(),
                busy: false,
                user_active: 0,
            }),
            wake: Condvar::new(),
            cache,
        })
    }

    pub fn clear(&self) {
        self.state.lock().unwrap().queue.clear();
    }

    pub fn user_enter(&self) {
        self.state.lock().unwrap().user_active += 1;
    }

    pub fn user_leave(&self) {
        let mut g = self.state.lock().unwrap();
        g.user_active = g.user_active.saturating_sub(1);
        if g.user_active == 0 {
            self.wake.notify_all();
        }
    }

    pub fn user_guard(self: &Arc<Self>) -> PrefetchUserGuard {
        self.user_enter();
        PrefetchUserGuard(self.clone())
    }

    #[cfg(test)]
    pub fn user_active_count(&self) -> u32 {
        self.state.lock().unwrap().user_active
    }

    pub fn enqueue_newest_root(self: &Arc<Self>, generation: u64, repo: &str, rows: &[SnapshotRow]) {
        if let Some(row) = newest_snapshot(rows) {
            self.enqueue(generation, repo, &row.tree);
        }
    }

    pub fn enqueue_child_subtrees(
        self: &Arc<Self>,
        generation: u64,
        repo: &str,
        listing: &CachedListing,
    ) {
        let mut n = 0;
        for child in listing.children.iter() {
            if n >= MAX_CHILD_PREFETCH {
                break;
            }
            let Some(sub) = child.subtree.as_deref().filter(|s| !s.is_empty()) else {
                continue;
            };
            self.enqueue(generation, repo, sub);
            n += 1;
        }
    }

    fn enqueue(self: &Arc<Self>, generation: u64, repo: &str, tree_id: &str) {
        let start = {
            let mut g = self.state.lock().unwrap();
            if !g.queue.iter().any(|(_, _, t)| t == tree_id) {
                g.queue
                    .push_back((generation, repo.to_string(), tree_id.to_string()));
            }
            if g.busy {
                return;
            }
            g.busy = true;
            true
        };
        if start {
            let me = self.clone();
            thread::spawn(move || me.drain());
        }
    }

    fn drain(self: Arc<Self>) {
        loop {
            let next = {
                let mut g = self.state.lock().unwrap();
                while g.user_active > 0 {
                    g = self.wake.wait(g).unwrap();
                }
                match g.queue.pop_front() {
                    Some(item) => item,
                    None => {
                        g.busy = false;
                        return;
                    }
                }
            };
            let (generation, repo, tree_id) = next;
            if self.cache.generation() == generation {
                let _ = self.cache.get_or_fetch(generation, &repo, &tree_id);
            }
        }
    }
}

pub struct PrefetchUserGuard(Arc<PrefetchQueue>);

impl Drop for PrefetchUserGuard {
    fn drop(&mut self) {
        self.0.user_leave();
    }
}

#[cfg(test)]
#[path = "tree_prefetch_tests.rs"]
mod tests;

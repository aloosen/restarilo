use crate::restic_tree::TreeChild;
use crate::tree_disk_cache::TreeDiskCache;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};

const MEMORY_BUDGET: usize = 32 * 1024 * 1024;

#[derive(Clone)]
pub struct CachedListing {
    pub children: Arc<Vec<TreeChild>>,
    pub size_est: usize,
}

pub fn listing_size_est(children: &[TreeChild]) -> usize {
    children
        .iter()
        .map(|c| {
            c.name.len()
                + c.entry_type.len()
                + c.mtime.as_ref().map(|s| s.len()).unwrap_or(0)
                + 1
                + usize::from(c.size.is_some()) * 8
                + usize::from(c.subtree.is_some()) * 64
        })
        .sum()
}

type FetchFn = Arc<dyn Fn(&str, &str) -> Result<Vec<TreeChild>, String> + Send + Sync>;
type Flight = Arc<(Mutex<FlightState>, Condvar)>;

struct FlightState {
    generation: u64,
    done: bool,
    result: Option<Result<CachedListing, String>>,
}

pub struct TreeCache {
    inner: Mutex<Inner>,
}

struct Inner {
    generation: u64,
    snapshot_roots: HashMap<String, String>,
    entries: HashMap<String, CachedListing>,
    lru: VecDeque<String>,
    bytes: usize,
    in_flight: HashMap<String, Flight>,
    fetch: Option<FetchFn>,
    disk: Option<Arc<TreeDiskCache>>,
    session_repo_id: Option<String>,
}

impl TreeCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                generation: 0,
                snapshot_roots: HashMap::new(),
                entries: HashMap::new(),
                lru: VecDeque::new(),
                bytes: 0,
                in_flight: HashMap::new(),
                fetch: None,
                disk: None,
                session_repo_id: None,
            }),
        }
    }

    pub fn set_disk(&self, disk: Option<Arc<TreeDiskCache>>) {
        self.inner.lock().unwrap().disk = disk;
    }

    pub fn set_session_repo_id(&self, generation: u64, repo_id: Option<String>) -> bool {
        let mut g = self.inner.lock().unwrap();
        if g.generation != generation {
            return false;
        }
        g.session_repo_id = repo_id;
        true
    }

    pub fn set_fetcher<F>(&self, f: F)
    where
        F: Fn(&str, &str) -> Result<Vec<TreeChild>, String> + Send + Sync + 'static,
    {
        self.inner.lock().unwrap().fetch = Some(Arc::new(f));
    }

    pub fn begin_list_snapshots(&self, _repo: &str) -> u64 {
        let mut g = self.inner.lock().unwrap();
        g.generation = g.generation.wrapping_add(1);
        g.snapshot_roots.clear();
        g.entries.clear();
        g.lru.clear();
        g.bytes = 0;
        g.in_flight.clear();
        g.session_repo_id = None;
        g.generation
    }

    pub fn generation(&self) -> u64 {
        self.inner.lock().unwrap().generation
    }

    pub fn remember_snapshots(&self, generation: u64, roots: HashMap<String, String>) -> bool {
        let mut g = self.inner.lock().unwrap();
        if g.generation != generation {
            return false;
        }
        g.snapshot_roots = roots;
        true
    }

    pub fn root_tree(&self, snapshot_id: &str) -> Option<String> {
        self.inner.lock().unwrap().snapshot_roots.get(snapshot_id).cloned()
    }

    fn touch_lru(g: &mut Inner, tree_id: &str) {
        if let Some(pos) = g.lru.iter().position(|id| id == tree_id) {
            g.lru.remove(pos);
        }
        g.lru.push_back(tree_id.to_string());
    }

    pub fn get_cached(&self, tree_id: &str) -> Option<CachedListing> {
        let mut g = self.inner.lock().unwrap();
        let hit = g.entries.get(tree_id).cloned()?;
        Self::touch_lru(&mut g, tree_id);
        Some(hit)
    }

    pub fn insert_listing(
        &self,
        generation: u64,
        tree_id: &str,
        children: Vec<TreeChild>,
    ) -> Result<CachedListing, String> {
        let mut g = self.inner.lock().unwrap();
        if g.generation != generation {
            return Err("CANCELLED: stale cache generation".into());
        }
        let size_est = listing_size_est(&children);
        let listing = CachedListing { children: Arc::new(children), size_est };
        if size_est > MEMORY_BUDGET {
            return Ok(listing);
        }
        while g.bytes + size_est > MEMORY_BUDGET {
            let Some(old) = g.lru.pop_front() else { break };
            if let Some(removed) = g.entries.remove(&old) {
                g.bytes = g.bytes.saturating_sub(removed.size_est);
            }
        }
        if let Some(prev) = g.entries.insert(tree_id.to_string(), listing.clone()) {
            g.bytes = g.bytes.saturating_sub(prev.size_est);
        }
        Self::touch_lru(&mut g, tree_id);
        g.bytes += size_est;
        Ok(listing)
    }

    pub fn get_or_fetch(
        &self,
        generation: u64,
        repo: &str,
        tree_id: &str,
    ) -> Result<CachedListing, String> {
        // Generation + memory hit under one lock (no stale cross-repo RAM hits).
        {
            let mut g = self.inner.lock().unwrap();
            if g.generation != generation {
                return Err("CANCELLED: stale cache generation".into());
            }
            if let Some(hit) = g.entries.get(tree_id).cloned() {
                Self::touch_lru(&mut g, tree_id);
                return Ok(hit);
            }
            let disk = g.disk.clone();
            let rid = g.session_repo_id.clone();
            drop(g);
            if let (Some(disk), Some(rid)) = (disk, rid) {
                if let Some(children) = disk.get(&rid, tree_id) {
                    return self.insert_listing(generation, tree_id, children);
                }
            }
        }
        let (slot, is_leader, disk, rid, fetch) = {
            let mut g = self.inner.lock().unwrap();
            if g.generation != generation {
                return Err("CANCELLED: stale cache generation".into());
            }
            if let Some(hit) = g.entries.get(tree_id).cloned() {
                Self::touch_lru(&mut g, tree_id);
                return Ok(hit);
            }
            let disk = g.disk.clone();
            let rid = g.session_repo_id.clone();
            let fetch = g.fetch.clone();
            if let Some(existing) = g.in_flight.get(tree_id).cloned() {
                let same_gen = existing.0.lock().unwrap().generation == generation;
                if same_gen {
                    (existing, false, disk, rid, fetch)
                } else {
                    let slot = Self::new_flight(generation);
                    g.in_flight.insert(tree_id.to_string(), slot.clone());
                    (slot, true, disk, rid, fetch)
                }
            } else {
                let slot = Self::new_flight(generation);
                g.in_flight.insert(tree_id.to_string(), slot.clone());
                (slot, true, disk, rid, fetch)
            }
        };
        if !is_leader {
            let (lock, cvar) = &*slot;
            let mut guard = lock.lock().unwrap();
            while !guard.done {
                guard = cvar.wait(guard).unwrap();
            }
            return guard.result.clone().unwrap_or_else(|| {
                Err("CANCELLED: in-flight fetch missing result".into())
            });
        }
        let outcome = match fetch {
            Some(f) => match f(repo, tree_id) {
                Ok(children) => {
                    if self.generation() == generation {
                        if let (Some(disk), Some(rid)) = (disk, rid) {
                            disk.put(&rid, tree_id, &children);
                        }
                    }
                    self.insert_listing(generation, tree_id, children)
                }
                Err(e) => Err(e),
            },
            None => Err("tree fetch not configured".into()),
        };
        {
            let mut g = self.inner.lock().unwrap();
            if g.in_flight.get(tree_id).is_some_and(|e| Arc::ptr_eq(e, &slot)) {
                g.in_flight.remove(tree_id);
            }
        }
        let (lock, cvar) = &*slot;
        let mut guard = lock.lock().unwrap();
        guard.done = true;
        guard.result = Some(outcome.clone());
        cvar.notify_all();
        outcome
    }

    fn new_flight(generation: u64) -> Flight {
        Arc::new((
            Mutex::new(FlightState {
                generation,
                done: false,
                result: None,
            }),
            Condvar::new(),
        ))
    }
}

#[cfg(test)]
#[path = "tree_cache_tests.rs"]
mod tests;

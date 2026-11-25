use crate::restic_ls::normalize_dir_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderChild {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderDirs {
    pub dirs: Vec<FolderChild>,
    pub children_complete: bool,
}

type Key = (String, String, String);

pub struct FolderIndex {
    inner: Mutex<Inner>,
}

struct Inner {
    generation: u64,
    entries: HashMap<Key, FolderDirs>,
}

impl FolderIndex {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                generation: 0,
                entries: HashMap::new(),
            }),
        }
    }

    /// Align with `TreeCache::begin_list_snapshots` — same generation counter.
    pub fn begin(&self, generation: u64) {
        let mut g = self.inner.lock().unwrap();
        g.generation = generation;
        g.entries.clear();
    }

    #[allow(dead_code)] // used by scan_scheduler / commands (sub-task B)
    pub fn generation(&self) -> u64 {
        self.inner.lock().unwrap().generation
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut g = self.inner.lock().unwrap();
        g.entries.clear();
    }

    pub fn get(
        &self,
        generation: u64,
        repo: &str,
        snapshot_id: &str,
        parent_path: &str,
    ) -> Option<FolderDirs> {
        let g = self.inner.lock().unwrap();
        if g.generation != generation {
            return None;
        }
        g.entries
            .get(&key(repo, snapshot_id, parent_path))
            .cloned()
    }

    /// Merge child dirs. Scan upserts must pass `mark_complete: false`.
    pub fn upsert_dirs(
        &self,
        generation: u64,
        repo: &str,
        snapshot_id: &str,
        parent_path: &str,
        children: &[FolderChild],
        mark_complete: bool,
    ) -> bool {
        let mut g = self.inner.lock().unwrap();
        if g.generation != generation {
            return false;
        }
        let k = key(repo, snapshot_id, parent_path);
        let entry = g.entries.entry(k).or_insert_with(|| FolderDirs {
            dirs: Vec::new(),
            children_complete: false,
        });
        for c in children {
            if !entry.dirs.iter().any(|e| e.name == c.name) {
                entry.dirs.push(c.clone());
            }
        }
        entry.dirs.sort_by(|a, b| a.name.cmp(&b.name));
        if mark_complete {
            entry.children_complete = true;
        }
        true
    }

    pub fn mark_complete(
        &self,
        generation: u64,
        repo: &str,
        snapshot_id: &str,
        parent_path: &str,
    ) -> bool {
        let mut g = self.inner.lock().unwrap();
        if g.generation != generation {
            return false;
        }
        let k = key(repo, snapshot_id, parent_path);
        let entry = g.entries.entry(k).or_insert_with(|| FolderDirs {
            dirs: Vec::new(),
            children_complete: false,
        });
        entry.children_complete = true;
        true
    }
}

fn key(repo: &str, snapshot_id: &str, parent_path: &str) -> Key {
    (
        repo.to_string(),
        snapshot_id.to_string(),
        normalize_dir_path(parent_path),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn child(name: &str, path: &str) -> FolderChild {
        FolderChild {
            name: name.into(),
            path: path.into(),
        }
    }

    #[test]
    fn merge_sort_and_complete() {
        let idx = FolderIndex::new();
        idx.begin(1);
        assert!(idx.upsert_dirs(
            1,
            "r",
            "s",
            "/",
            &[child("b", "/b"), child("a", "/a")],
            false,
        ));
        let got = idx.get(1, "r", "s", "/").unwrap();
        assert_eq!(got.dirs[0].name, "a");
        assert_eq!(got.dirs[1].name, "b");
        assert!(!got.children_complete);
        assert!(idx.mark_complete(1, "r", "s", "/"));
        assert!(idx.get(1, "r", "s", "/").unwrap().children_complete);
    }

    #[test]
    fn scan_upsert_does_not_set_complete() {
        let idx = FolderIndex::new();
        idx.begin(1);
        idx.upsert_dirs(1, "r", "s", "/", &[child("a", "/a")], false);
        assert!(!idx.get(1, "r", "s", "/").unwrap().children_complete);
        idx.upsert_dirs(1, "r", "s", "/", &[child("a", "/a")], true);
        assert!(idx.get(1, "r", "s", "/").unwrap().children_complete);
    }

    #[test]
    fn stale_generation_noop() {
        let idx = FolderIndex::new();
        idx.begin(1);
        idx.upsert_dirs(1, "r", "s", "/", &[child("a", "/a")], true);
        idx.begin(2);
        assert!(!idx.upsert_dirs(1, "r", "s", "/", &[child("b", "/b")], true));
        assert!(idx.get(1, "r", "s", "/").is_none());
        assert!(idx.get(2, "r", "s", "/").is_none());
    }

    #[test]
    fn snapshot_a_invisible_to_b() {
        let idx = FolderIndex::new();
        idx.begin(1);
        idx.upsert_dirs(1, "r", "snapA", "/", &[child("onlyA", "/onlyA")], true);
        assert!(idx.get(1, "r", "snapB", "/").is_none());
        assert_eq!(
            idx.get(1, "r", "snapA", "/").unwrap().dirs[0].name,
            "onlyA"
        );
    }
}

use crate::restic_tree::TreeChild;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const FORMAT_VERSION: u32 = 1;
#[cfg(not(test))]
const DISK_BUDGET: u64 = 256 * 1024 * 1024;
/// Small budget in tests so eviction can be exercised without writing hundreds of MiB.
#[cfg(test)]
const DISK_BUDGET: u64 = 4 * 1024;

#[derive(Serialize, Deserialize)]
struct DiskListing {
    format_version: u32,
    children: Vec<DiskChild>,
}

#[derive(Serialize, Deserialize)]
struct DiskChild {
    name: String,
    is_dir: bool,
    #[serde(default)]
    entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mtime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtree: Option<String>,
}

pub struct TreeDiskCache {
    root: PathBuf,
    lock: Mutex<()>,
}

impl TreeDiskCache {
    pub fn open(root: PathBuf) -> Self {
        Self {
            root,
            lock: Mutex::new(()),
        }
    }

    fn entry_path(&self, repo_id: &str, tree_id: &str) -> PathBuf {
        self.root
            .join("v1")
            .join(repo_id.to_lowercase())
            .join(format!("{}.json", tree_id.to_lowercase()))
    }

    pub fn get(&self, repo_id: &str, tree_id: &str) -> Option<Vec<TreeChild>> {
        let _g = self.lock.lock().unwrap();
        let path = self.entry_path(repo_id, tree_id);
        let raw = fs::read_to_string(&path).ok()?;
        let parsed: DiskListing = serde_json::from_str(&raw).ok()?;
        if parsed.format_version != FORMAT_VERSION {
            return None;
        }
        let _ = touch_mtime(&path);
        Some(
            parsed
                .children
                .into_iter()
                .map(|c| {
                    let entry_type = if c.entry_type.is_empty() {
                        if c.is_dir {
                            "dir".into()
                        } else {
                            "file".into()
                        }
                    } else {
                        c.entry_type
                    };
                    TreeChild {
                        name: c.name,
                        is_dir: c.is_dir,
                        entry_type,
                        size: c.size,
                        mtime: c.mtime,
                        subtree: c.subtree,
                    }
                })
                .collect(),
        )
    }

    pub fn put(&self, repo_id: &str, tree_id: &str, children: &[TreeChild]) {
        let _g = self.lock.lock().unwrap();
        if !is_tree_id(tree_id) {
            return;
        }
        let listing = DiskListing {
            format_version: FORMAT_VERSION,
            children: children
                .iter()
                .map(|c| DiskChild {
                    name: c.name.clone(),
                    is_dir: c.is_dir,
                    entry_type: c.entry_type.clone(),
                    size: c.size,
                    mtime: c.mtime.clone(),
                    subtree: c.subtree.clone(),
                })
                .collect(),
        };
        let Ok(bytes) = serde_json::to_vec_pretty(&listing) else {
            return;
        };
        if bytes.len() as u64 > DISK_BUDGET {
            return;
        }
        let path = self.entry_path(repo_id, tree_id);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, &bytes).is_err() {
            return;
        }
        if fs::rename(&tmp, &path).is_err() {
            let _ = fs::remove_file(&tmp);
            return;
        }
        self.evict_until_under(repo_id);
    }

    fn layout_files(&self, repo_id: &str) -> Vec<(PathBuf, u64, SystemTime)> {
        let dir = self.root.join("v1").join(repo_id.to_lowercase());
        let Ok(rd) = fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for ent in rd.flatten() {
            let path = ent.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Some(stem) = name.strip_suffix(".json") else {
                continue;
            };
            if !is_tree_id(stem) {
                continue;
            }
            let Ok(meta) = ent.metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            out.push((path, meta.len(), meta.modified().unwrap_or(UNIX_EPOCH)));
        }
        out
    }

    fn evict_until_under(&self, repo_id: &str) {
        let mut files = self.layout_files(repo_id);
        let mut total: u64 = files.iter().map(|(_, n, _)| *n).sum();
        if total <= DISK_BUDGET {
            return;
        }
        files.sort_by_key(|(_, _, m)| *m);
        for (path, len, _) in files {
            if total <= DISK_BUDGET {
                break;
            }
            if fs::remove_file(&path).is_ok() {
                total = total.saturating_sub(len);
            }
        }
    }
}

fn is_tree_id(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn touch_mtime(path: &Path) -> std::io::Result<()> {
    let file = fs::OpenOptions::new().write(true).open(path)?;
    file.set_modified(SystemTime::now())
}

#[cfg(test)]
#[path = "tree_disk_cache_tests.rs"]
mod tests;

use crate::restic_tree::TreeChild;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const FORMAT_VERSION: u32 = 1;
const DISK_BUDGET: u64 = 256 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct DiskListing {
    format_version: u32,
    children: Vec<DiskChild>,
}

#[derive(Serialize, Deserialize)]
struct DiskChild {
    name: String,
    is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtree: Option<String>,
}

pub struct TreeDiskCache {
    root: PathBuf,
}

impl TreeDiskCache {
    pub fn open(root: PathBuf) -> Self {
        Self { root }
    }

    fn entry_path(&self, repo_id: &str, tree_id: &str) -> PathBuf {
        self.root
            .join("v1")
            .join(repo_id.to_lowercase())
            .join(format!("{}.json", tree_id.to_lowercase()))
    }

    pub fn get(&self, repo_id: &str, tree_id: &str) -> Option<Vec<TreeChild>> {
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
                .map(|c| TreeChild {
                    name: c.name,
                    is_dir: c.is_dir,
                    size: c.size,
                    subtree: c.subtree,
                })
                .collect(),
        )
    }

    pub fn put(&self, repo_id: &str, tree_id: &str, children: &[TreeChild]) {
        let listing = DiskListing {
            format_version: FORMAT_VERSION,
            children: children
                .iter()
                .map(|c| DiskChild {
                    name: c.name.clone(),
                    is_dir: c.is_dir,
                    size: c.size,
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
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(meta) = ent.metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let mtime = meta.modified().unwrap_or(UNIX_EPOCH);
            out.push((path, meta.len(), mtime));
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

fn touch_mtime(path: &Path) -> std::io::Result<()> {
    let file = fs::OpenOptions::new().write(true).open(path)?;
    file.set_modified(SystemTime::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn round_trip_and_ignore_junk() {
        let dir = std::env::temp_dir().join(format!("restarilo-disk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("junk.bin"), vec![0u8; 10]).unwrap();
        let cache = TreeDiskCache::open(dir.clone());
        let kids = vec![TreeChild {
            name: "a".into(),
            is_dir: true,
            size: None,
            subtree: Some("abc".into()),
        }];
        cache.put("repo1", "tree1", &kids);
        let got = cache.get("repo1", "tree1").unwrap();
        assert_eq!(got[0].name, "a");
        assert!(dir.join("junk.bin").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn skip_oversized_single_entry() {
        let dir = std::env::temp_dir().join(format!("restarilo-disk-big-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let cache = TreeDiskCache::open(dir.clone());
        let kids = vec![TreeChild {
            name: "x".repeat((DISK_BUDGET as usize) + 8),
            is_dir: false,
            size: None,
            subtree: None,
        }];
        cache.put("repo1", "huge", &kids);
        assert!(cache.get("repo1", "huge").is_none());
        let _ = fs::remove_dir_all(&dir);
        let _ = Duration::from_secs(0);
    }
}

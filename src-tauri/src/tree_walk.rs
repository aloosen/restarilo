use crate::config::Config;
use crate::restic_ls::{normalize_dir_path, DirEntry};
use crate::tree_cache::TreeCache;

pub fn list_dir(
    _cfg: &Config,
    cache: &TreeCache,
    generation: u64,
    repo_name: &str,
    snapshot_id: &str,
    path: &str,
) -> Result<Vec<DirEntry>, String> {
    if snapshot_id.trim().is_empty() {
        return Err("snapshot id is required".into());
    }
    if cache.generation() != generation {
        return Err("CANCELLED: stale cache generation".into());
    }
    let abs = normalize_dir_path(path);
    let root = cache
        .root_tree(snapshot_id)
        .ok_or_else(|| format!("unknown snapshot root for id '{snapshot_id}'"))?;
    let mut tree_id = root;
    let mut parent_path = String::from("/");
    if abs != "/" {
        for segment in abs.trim_start_matches('/').split('/') {
            if segment.is_empty() {
                continue;
            }
            let listing = cache.get_or_fetch(generation, repo_name, &tree_id)?;
            let child = listing
                .children
                .iter()
                .find(|c| c.name == segment)
                .ok_or_else(|| format!("path not found: {abs}"))?;
            let subtree = child
                .subtree
                .as_ref()
                .ok_or_else(|| format!("not a directory: {abs}"))?;
            tree_id = subtree.clone();
            parent_path = if parent_path == "/" {
                format!("/{segment}")
            } else {
                format!("{parent_path}/{segment}")
            };
        }
    }
    let listing = cache.get_or_fetch(generation, repo_name, &tree_id)?;
    let mut entries: Vec<DirEntry> = listing
        .children
        .iter()
        .map(|c| {
            let path = if parent_path == "/" {
                format!("/{}", c.name)
            } else {
                format!("{parent_path}/{}", c.name)
            };
            DirEntry {
                name: c.name.clone(),
                path,
                is_dir: c.is_dir,
                size: c.size,
            }
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

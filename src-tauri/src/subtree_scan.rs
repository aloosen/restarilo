use crate::folder_index::{FolderChild, FolderIndex};
use crate::restic_ls::normalize_dir_path;
use serde::Deserialize;
use std::io::BufRead;

pub const MAX_DIR_NODES: usize = 2_000;
pub const MAX_DEPTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanStop {
    Eof,
    NodeBudget,
    WallTime,
    Cancelled,
    Failed,
}

#[derive(Deserialize)]
struct LsNode {
    #[serde(default)]
    name: String,
    #[serde(rename = "type")]
    #[serde(default)]
    node_type: String,
    #[serde(default)]
    path: String,
}

/// Parse one NDJSON line into a parent→child dir edge, or None if skipped.
pub fn edge_from_ls_line(line: &str, scan_root: &str) -> Option<(String, FolderChild)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let node: LsNode = serde_json::from_str(line).ok()?;
    if node.node_type != "dir" || node.path.is_empty() {
        return None;
    }
    let root = normalize_dir_path(scan_root);
    let path = normalize_dir_path(&node.path);
    if path == root {
        return None;
    }
    let depth = depth_below(&root, &path)?;
    if depth > MAX_DEPTH {
        return None;
    }
    let parent = parent_path(&path)?;
    let name = if node.name.is_empty() {
        path.rsplit('/').next()?.to_string()
    } else {
        node.name
    };
    Some((parent, FolderChild { name, path }))
}

fn depth_below(root: &str, path: &str) -> Option<usize> {
    if path == root {
        return Some(0);
    }
    if root != "/" && !(path.starts_with(root) && path[root.len()..].starts_with('/')) {
        return None;
    }
    let rest = if root == "/" {
        path.trim_start_matches('/')
    } else {
        &path[root.len() + 1..]
    };
    if rest.is_empty() {
        return Some(0);
    }
    Some(rest.split('/').filter(|s| !s.is_empty()).count())
}

fn parent_path(path: &str) -> Option<String> {
    let p = normalize_dir_path(path);
    if p == "/" {
        return None;
    }
    let idx = p.rfind('/')?;
    if idx == 0 {
        Some("/".into())
    } else {
        Some(p[..idx].to_string())
    }
}

pub fn consume_ls_reader<R: BufRead>(
    reader: R,
    scan_root: &str,
    generation: u64,
    repo: &str,
    snapshot_id: &str,
    index: &FolderIndex,
    node_budget: usize,
    mut on_batch: impl FnMut(&str, &[FolderChild]),
) -> ScanStop {
    let mut nodes = 0usize;
    for line in reader.lines() {
        let Ok(line) = line else {
            break;
        };
        let Some((parent, child)) = edge_from_ls_line(&line, scan_root) else {
            continue;
        };
        index.upsert_dirs(generation, repo, snapshot_id, &parent, &[child.clone()], false);
        on_batch(&parent, &[child]);
        nodes += 1;
        if nodes >= node_budget {
            return ScanStop::NodeBudget;
        }
    }
    ScanStop::Eof
}

#[cfg(test)]
#[path = "subtree_scan_tests.rs"]
mod tests;

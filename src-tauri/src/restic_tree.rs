use crate::config::Config;
use crate::restic_cmd::run_restic;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static FETCH_TOTAL: AtomicU64 = AtomicU64::new(0);
static FETCH_BY_TREE: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeChild {
    pub name: String,
    pub is_dir: bool,
    /// restic node type: file, dir, symlink, …
    pub entry_type: String,
    pub size: Option<u64>,
    pub mtime: Option<String>,
    pub subtree: Option<String>,
}

#[derive(Deserialize)]
struct TreeBlob {
    #[serde(default)]
    nodes: Vec<TreeNode>,
}

#[derive(Deserialize)]
struct TreeNode {
    #[serde(default)]
    name: String,
    #[serde(rename = "type")]
    #[serde(default)]
    node_type: String,
    size: Option<u64>,
    #[serde(default)]
    mtime: Option<String>,
    subtree: Option<String>,
}

pub fn decode_restic_name(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' || i + 1 >= bytes.len() {
            out.push(raw[i..].chars().next().unwrap());
            i += raw[i..].chars().next().unwrap().len_utf8();
            continue;
        }
        match bytes[i + 1] {
            b'\\' => {
                out.push('\\');
                i += 2;
            }
            b'"' => {
                out.push('"');
                i += 2;
            }
            b'\'' => {
                out.push('\'');
                i += 2;
            }
            b'n' => {
                out.push('\n');
                i += 2;
            }
            b't' => {
                out.push('\t');
                i += 2;
            }
            b'r' => {
                out.push('\r');
                i += 2;
            }
            b'a' => {
                out.push('\u{0007}');
                i += 2;
            }
            b'b' => {
                out.push('\u{0008}');
                i += 2;
            }
            b'v' => {
                out.push('\u{000b}');
                i += 2;
            }
            b'f' => {
                out.push('\u{000c}');
                i += 2;
            }
            b'x' if i + 4 <= bytes.len() => {
                if let Ok(v) = u8::from_str_radix(
                    std::str::from_utf8(&bytes[i + 2..i + 4]).unwrap_or(""),
                    16,
                ) {
                    out.push(char::from(v));
                    i += 4;
                } else {
                    out.push('\\');
                    i += 1;
                }
            }
            b'u' if i + 6 <= bytes.len() => {
                if let Ok(v) = u32::from_str_radix(
                    std::str::from_utf8(&bytes[i + 2..i + 6]).unwrap_or(""),
                    16,
                ) {
                    out.push(char::from_u32(v).unwrap_or('\u{FFFD}'));
                    i += 6;
                } else {
                    out.push('\\');
                    i += 1;
                }
            }
            b'U' if i + 10 <= bytes.len() => {
                if let Ok(v) = u32::from_str_radix(
                    std::str::from_utf8(&bytes[i + 2..i + 10]).unwrap_or(""),
                    16,
                ) {
                    out.push(char::from_u32(v).unwrap_or('\u{FFFD}'));
                    i += 10;
                } else {
                    out.push('\\');
                    i += 1;
                }
            }
            _ => {
                out.push('\\');
                i += 1;
            }
        }
    }
    out
}

pub fn parse_tree_json(raw: &str) -> Result<Vec<TreeChild>, String> {
    let blob: TreeBlob =
        serde_json::from_str(raw.trim()).map_err(|e| format!("failed to parse tree JSON: {e}"))?;
    Ok(blob
        .nodes
        .into_iter()
        .map(|n| {
            let entry_type = if n.node_type.is_empty() {
                "file".into()
            } else {
                n.node_type
            };
            TreeChild {
                name: decode_restic_name(&n.name),
                is_dir: entry_type == "dir",
                entry_type,
                size: n.size,
                mtime: n.mtime.filter(|s| !s.is_empty()),
                subtree: n.subtree.filter(|s| !s.is_empty()),
            }
        })
        .collect())
}

pub fn cat_tree(cfg: &Config, repo_name: &str, tree_id: &str) -> Result<Vec<TreeChild>, String> {
    if tree_id.trim().is_empty() {
        return Err("tree id is required".into());
    }
    let raw = run_restic(cfg, repo_name, &["cat", "blob", tree_id])?;
    let children = parse_tree_json(&raw)?;
    record_fetch(tree_id);
    Ok(children)
}

#[derive(Deserialize)]
struct RepoConfigJson {
    #[serde(default)]
    id: String,
}

pub fn restic_repository_id(cfg: &Config, repo_name: &str) -> Result<String, String> {
    let raw = run_restic(cfg, repo_name, &["cat", "config"])?;
    let parsed: RepoConfigJson = serde_json::from_str(raw.trim())
        .map_err(|e| format!("failed to parse restic config JSON: {e}"))?;
    if parsed.id.trim().is_empty() {
        return Err("restic config missing id".into());
    }
    Ok(parsed.id)
}

pub(crate) fn record_fetch(tree_id: &str) {
    FETCH_TOTAL.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut guard) = FETCH_BY_TREE.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        *map.entry(tree_id.to_string()).or_insert(0) += 1;
    }
}

#[cfg(test)]
pub fn tree_fetch_count_for_tests(tree_id: &str) -> u64 {
    FETCH_BY_TREE
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|m| *m.get(tree_id).unwrap_or(&0)))
        .unwrap_or(0)
}

#[cfg(test)]
pub fn reset_tree_fetch_counts_for_tests() {
    FETCH_TOTAL.store(0, Ordering::Relaxed);
    if let Ok(mut g) = FETCH_BY_TREE.lock() {
        *g = Some(HashMap::new());
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_restic_name, parse_tree_json};

    #[test]
    fn decode_newline_quote_backslash() {
        assert_eq!(decode_restic_name("a\\nb"), "a\nb");
        assert_eq!(decode_restic_name("say \\\"hi\\\""), "say \"hi\"");
        assert_eq!(decode_restic_name("a\\\\b"), "a\\b");
        assert_eq!(decode_restic_name("plain"), "plain");
        assert_eq!(decode_restic_name("\\x7f"), "\u{7f}");
        assert_eq!(decode_restic_name("\\u2028"), "\u{2028}");
    }

    #[test]
    fn parse_tree_fixture_decodes_names() {
        let raw = r#"{"nodes":[
            {"name":"a\\nb","type":"dir","subtree":"abc"},
            {"name":"file.txt","type":"file","size":3},
            {"name":"say \"hi\"","type":"file","size":1}
        ]}"#;
        let nodes = parse_tree_json(raw).expect("parse");
        assert_eq!(nodes.len(), 3);
        assert!(nodes[0].is_dir);
        assert_eq!(nodes[0].name, "a\nb");
        assert_eq!(nodes[0].subtree.as_deref(), Some("abc"));
        assert!(!nodes[1].is_dir);
        assert_eq!(nodes[1].name, "file.txt");
        // JSON already yields a real quote; decode leaves it unchanged.
        assert_eq!(nodes[2].name, "say \"hi\"");
    }

    #[test]
    fn parse_empty_nodes() {
        let nodes = parse_tree_json(r#"{"nodes":[]}"#).expect("parse");
        assert!(nodes.is_empty());
    }
}

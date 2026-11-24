use crate::config::Config;
use crate::restic_cmd;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Deserialize)]
struct LsLine {
    #[serde(default)]
    message_type: String,
    #[serde(default)]
    struct_type: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
    #[serde(rename = "type")]
    #[serde(default)]
    node_type: String,
    size: Option<u64>,
}

pub fn normalize_dir_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return "/".into();
    }
    let with_slash = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };
    with_slash.trim_end_matches('/').to_string()
}

pub fn list_dir(
    cfg: &Config,
    repo_name: &str,
    snapshot_id: &str,
    path: &str,
) -> Result<Vec<DirEntry>, String> {
    if snapshot_id.trim().is_empty() {
        return Err("snapshot id is required".into());
    }
    let abs = normalize_dir_path(path);
    let raw = restic_cmd::run_restic(cfg, repo_name, &["ls", snapshot_id, &abs, "--json"])?;
    Ok(parse_ls_json(&raw, &abs))
}

pub fn parse_ls_json(raw: &str, requested_dir: &str) -> Vec<DirEntry> {
    let req = normalize_dir_path(requested_dir);
    let mut entries: Vec<DirEntry> = raw
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let msg: LsLine = serde_json::from_str(line).ok()?;
            let is_node = msg.message_type == "node" || msg.struct_type == "node";
            if !is_node {
                return None;
            }
            let path = normalize_dir_path(&msg.path);
            if path == req {
                return None;
            }
            let name = if msg.name.is_empty() {
                path.rsplit('/').next().unwrap_or("").to_string()
            } else {
                msg.name
            };
            Some(DirEntry {
                name,
                path,
                is_dir: msg.node_type == "dir",
                size: msg.size,
            })
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_snapshot_and_self_sorts_by_name() {
        let raw = r#"
{"message_type":"snapshot","struct_type":"snapshot","id":"abc"}
{"message_type":"node","struct_type":"node","name":"z","type":"file","path":"/z"}
{"message_type":"node","struct_type":"node","name":"","type":"dir","path":"/"}
{"message_type":"node","struct_type":"node","name":"a","type":"dir","path":"/a"}
"#;
        let entries = parse_ls_json(raw, "/");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "a");
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "z");
        assert!(!entries[1].is_dir);
    }

    #[test]
    fn normalize_empty_and_trailing() {
        assert_eq!(normalize_dir_path(""), "/");
        assert_eq!(normalize_dir_path("/foo/"), "/foo");
        assert_eq!(normalize_dir_path("foo/bar"), "/foo/bar");
    }
}

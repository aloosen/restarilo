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
    // Do not trim whitespace — trailing/leading spaces can be part of names.
    if path.is_empty() || path == "/" {
        return "/".into();
    }
    let with_slash = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    // Strip trailing slashes only (not spaces).
    let mut end = with_slash.len();
    while end > 1 && with_slash.as_bytes()[end - 1] == b'/' {
        end -= 1;
    }
    with_slash[..end].to_string()
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
        assert_eq!(normalize_dir_path("/folder "), "/folder ");
        assert_eq!(normalize_dir_path("/folder /"), "/folder ");
    }

    #[test]
    fn live_browse_local_and_sftp() {
        use crate::config::{ResticSection, RepoConfig};
        use crate::restic_cmd;
        use std::path::Path;

        let fix = Path::new("/tmp/restarilo-z1");
        let local = fix.join("local-repo");
        let sftp_path = fix.join("sftp-repo");
        let key = fix.join("my keys/id_ed25519");
        assert!(
            local.exists() && sftp_path.exists() && key.exists(),
            "missing MVP fixtures under {} (see README)",
            fix.display()
        );
        let user = std::env::var("USER").expect("USER");
        let cfg = Config {
            restic: ResticSection {
                binary: Some("/usr/bin/restic".into()),
            },
            repos: vec![
                RepoConfig {
                    name: "local".into(),
                    path: local.to_string_lossy().into_owned(),
                    password_file: None,
                    password_command: None,
                    insecure_no_password: true,
                    ssh_key: None,
                },
                RepoConfig {
                    name: "sftp".into(),
                    path: format!("sftp:{user}@127.0.0.1:{}", sftp_path.to_string_lossy()),
                    password_file: None,
                    password_command: None,
                    insecure_no_password: true,
                    ssh_key: Some(key.to_string_lossy().into_owned()),
                },
            ],
        };

        for repo in ["local", "sftp"] {
            let snaps = restic_cmd::list_snapshots(&cfg, repo).expect("snapshots");
            let snap = snaps.first().expect("snapshot");
            let root = list_dir(&cfg, repo, &snap.id, "/").expect("root ls");
            assert!(!root.is_empty(), "{repo} root empty");
            let dir = root.iter().find(|e| e.is_dir).expect("dir at root");
            let child = list_dir(&cfg, repo, &snap.id, &dir.path).expect("child ls");
            assert!(
                child.iter().any(|e| e.is_dir || !e.is_dir),
                "{repo} child listing empty"
            );
            // walk until a file appears (fixture: …/data/file.txt)
            let mut path = dir.path.clone();
            let mut saw_file = false;
            for _ in 0..8 {
                let entries = list_dir(&cfg, repo, &snap.id, &path).expect("walk");
                if let Some(f) = entries.iter().find(|e| !e.is_dir) {
                    saw_file = true;
                    assert!(!f.name.is_empty());
                    break;
                }
                if let Some(d) = entries.iter().find(|e| e.is_dir) {
                    path = d.path.clone();
                    continue;
                }
                break;
            }
            assert!(saw_file, "{repo}: expected a file under snapshot");
        }

        let err = list_dir(&cfg, "local", "deadbeefdeadbeefdeadbeefdeadbeef", "/");
        assert!(err.is_err(), "bogus snapshot should fail");
    }
}

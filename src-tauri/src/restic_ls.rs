use crate::config::Config;
use crate::tree_cache::TreeCache;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

pub fn normalize_dir_path(path: &str) -> String {
    if path.is_empty() || path == "/" {
        return "/".into();
    }
    let with_slash = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut end = with_slash.len();
    while end > 1 && with_slash.as_bytes()[end - 1] == b'/' {
        end -= 1;
    }
    with_slash[..end].to_string()
}

pub fn list_dir(
    cfg: &Config,
    cache: &TreeCache,
    generation: u64,
    repo_name: &str,
    snapshot_id: &str,
    path: &str,
) -> Result<Vec<DirEntry>, String> {
    crate::tree_walk::list_dir(cfg, cache, generation, repo_name, snapshot_id, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::restic_cmd;
    use crate::restic_tree;
    use std::collections::HashMap;
    use std::sync::Arc;

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
        let cfg = Arc::new(cfg);

        for repo in ["local", "sftp"] {
            let cache = TreeCache::new();
            let cfg_f = cfg.clone();
            cache.set_fetcher(move |repo_name, tree_id| {
                restic_tree::cat_tree(&cfg_f, repo_name, tree_id)
            });
            let gen = cache.begin_list_snapshots(repo);
            let snaps = restic_cmd::list_snapshots(&cfg, repo).expect("snapshots");
            let roots: HashMap<_, _> = snaps
                .iter()
                .map(|s| (s.id.clone(), s.tree.clone()))
                .collect();
            assert!(cache.remember_snapshots(gen, roots));
            let snap = snaps.first().expect("snapshot");
            let root = list_dir(&cfg, &cache, gen, repo, &snap.id, "/").expect("root");
            assert!(!root.is_empty(), "{repo} root empty");
            let dir = root.iter().find(|e| e.is_dir).expect("dir at root");
            let child = list_dir(&cfg, &cache, gen, repo, &snap.id, &dir.path).expect("child");
            assert!(
                child.iter().any(|e| e.is_dir || !e.is_dir),
                "{repo} child listing empty"
            );
            let mut path = dir.path.clone();
            let mut saw_file = false;
            for _ in 0..8 {
                let entries = list_dir(&cfg, &cache, gen, repo, &snap.id, &path).expect("walk");
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

        let cache = TreeCache::new();
        let cfg_f = cfg.clone();
        cache.set_fetcher(move |repo_name, tree_id| {
            restic_tree::cat_tree(&cfg_f, repo_name, tree_id)
        });
        let gen = cache.begin_list_snapshots("local");
        let err = list_dir(
            &cfg,
            &cache,
            gen,
            "local",
            "deadbeefdeadbeefdeadbeefdeadbeef",
            "/",
        );
        assert!(err.is_err(), "bogus snapshot should fail");
    }
}

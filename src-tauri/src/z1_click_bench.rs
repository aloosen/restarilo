//! Click-sequence Z1 bench (temp RESTIC_CACHE_DIR only — never wipe ~/.cache/restic).
//! Run: `RESTARILO_Z1_BENCH=1 cargo test --lib z1_click -- --ignored --nocapture --test-threads=1`

use crate::config::{Config, RepoConfig, ResticSection};
use crate::restic_cmd;
use crate::restic_ls::list_dir;
use crate::restic_tree;
use crate::tree_cache::TreeCache;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// Parent path → next target child name on the deep Z1 route.
pub const Z1_CLICK_SEGMENTS: &[(&str, &str)] = &[
    ("/", "tmp"),
    ("/tmp", "restarilo-z1"),
    ("/tmp/restarilo-z1", "data"),
    ("/tmp/restarilo-z1/data", "shared"),
    ("/tmp/restarilo-z1/data/shared", "a"),
    ("/tmp/restarilo-z1/data/shared/a", "b"),
    ("/tmp/restarilo-z1/data/shared/a/b", "c"),
    ("/tmp/restarilo-z1/data/shared/a/b/c", "d"),
];

pub fn z1_cfg() -> Config {
    let fix = Path::new("/tmp/restarilo-z1");
    let user = std::env::var("USER").expect("USER");
    Config {
        restic: ResticSection {
            binary: Some("/usr/bin/restic".into()),
        },
        repos: vec![
            RepoConfig {
                name: "local".into(),
                path: fix.join("local-repo").to_string_lossy().into_owned(),
                password_file: None,
                password_command: None,
                insecure_no_password: true,
                ssh_key: None,
            },
            RepoConfig {
                name: "sftp".into(),
                path: format!(
                    "sftp:{user}@127.0.0.1:{}",
                    fix.join("sftp-repo").to_string_lossy()
                ),
                password_file: None,
                password_command: None,
                insecure_no_password: true,
                ssh_key: Some(fix.join("my keys/id_ed25519").to_string_lossy().into_owned()),
            },
        ],
        cache_dir: None,
    }
}

pub fn temp_restic_cache() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "restarilo-z1-restic-cache-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp restic cache");
    std::env::set_var("RESTIC_CACHE_DIR", &dir);
    dir
}

pub fn setup_cache(cfg: Arc<Config>, repo: &str, snap_id: &str) -> (Arc<TreeCache>, u64) {
    let cache = Arc::new(TreeCache::new());
    let cfg_f = cfg.clone();
    cache.set_fetcher(move |repo_name, tree_id| {
        restic_tree::cat_tree(&cfg_f, repo_name, tree_id)
    });
    let gen = cache.begin_list_snapshots(repo);
    let snaps = restic_cmd::list_snapshots(&cfg, repo).expect("snapshots");
    let roots: HashMap<_, _> = snaps
        .into_iter()
        .map(|s| (s.id.clone(), s.tree.clone()))
        .collect();
    assert!(roots.contains_key(snap_id), "snap missing from map");
    assert!(cache.remember_snapshots(gen, roots));
    (cache, gen)
}

pub fn data_snapshot_id(cfg: &Config, repo: &str) -> String {
    let snaps = restic_cmd::list_snapshots(cfg, repo).expect("snaps");
    snaps
        .into_iter()
        .rev()
        .find(|s| s.paths.iter().any(|p| p.ends_with("/data") || p.ends_with("data")))
        .map(|s| s.id)
        .expect("no data snapshot")
}

pub fn path_check(cfg: &Config, repo: &str, snap: &str) {
    let (cache, gen) = setup_cache(Arc::new(cfg.clone()), repo, snap);
    for &(parent, child) in Z1_CLICK_SEGMENTS {
        let entries = list_dir(cfg, &cache, gen, repo, snap, parent).expect("list");
        assert!(
            entries.iter().any(|e| e.name == child && e.is_dir),
            "missing child {child} under {parent}"
        );
    }
    println!("path-check ok snap={}", &snap[..8.min(snap.len())]);
}

#[test]
#[ignore]
fn z1_click_before_list_dir_sequence() {
    if std::env::var("RESTARILO_Z1_BENCH").is_err() {
        return;
    }
    let cfg = z1_cfg();
    let snap = data_snapshot_id(&cfg, "sftp");
    path_check(&cfg, "sftp", &snap);
    let _cache_dir = temp_restic_cache();
    let cfg = Arc::new(cfg);
    let (cache, gen) = setup_cache(cfg.clone(), "sftp", &snap);
    println!("| step | parent | child | ms |");
    println!("| --- | --- | --- | ---: |");
    let mut total = 0u128;
    for &(parent, child) in Z1_CLICK_SEGMENTS {
        let t0 = Instant::now();
        let entries = list_dir(&cfg, &cache, gen, "sftp", &snap, parent).expect("step");
        let ms = t0.elapsed().as_millis();
        assert!(entries.iter().any(|e| e.name == child && e.is_dir));
        total += ms;
        println!("| before | `{parent}` | `{child}` | {ms} |");
    }
    println!("| before | **total** | | {total} |");
}

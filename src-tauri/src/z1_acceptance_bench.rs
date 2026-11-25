//! Manual Z1 acceptance timings / portable wrapper probe.
//! Run: `RESTARILO_Z1_BENCH=1 cargo test -p restarilo --lib z1_acceptance -- --ignored --nocapture --test-threads=1`

use crate::config::{Config, RepoConfig, ResticSection};
use crate::restic_cmd;
use crate::restic_ls::list_dir;
use crate::restic_tree;
use crate::tree_cache::TreeCache;
use crate::tree_disk_cache::TreeDiskCache;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

fn z1_cfg(restic_binary: &str) -> Config {
    let fix = Path::new("/tmp/restarilo-z1");
    let user = std::env::var("USER").expect("USER");
    Config {
        restic: ResticSection {
            binary: Some(restic_binary.into()),
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

/// Use a disposable RESTIC_CACHE_DIR — never delete ~/.cache/restic.
fn clear_restic_cache() {
    let dir = std::env::temp_dir().join(format!(
        "restarilo-z1-restic-cache-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp restic cache");
    std::env::set_var("RESTIC_CACHE_DIR", &dir);
}

fn timed_ms<F: FnOnce() -> Result<(), String>>(label: &str, f: F) -> Result<u128, String> {
    let t0 = Instant::now();
    f()?;
    let ms = t0.elapsed().as_millis();
    println!("{label}: {ms} ms");
    Ok(ms)
}

fn deep_path() -> &'static str {
    // Fixture layout: backup path is /tmp/restarilo-z1/data/… → snapshot root lists `data`
    // restic stores absolute paths under the backup target's parent chain.
    // For Z1 backups of `$FIX/data`, root entries include the path segments.
    // Live browse test finds first dir then walks — deep target is shared/a/b/c/d under data.
    "/tmp/restarilo-z1/data/shared/a/b/c/d"
}

fn setup_cache(cfg: Arc<Config>, repo: &str) -> (TreeCache, u64, String) {
    let cache = TreeCache::new();
    let cfg_f = cfg.clone();
    cache.set_fetcher(move |repo_name, tree_id| {
        restic_tree::cat_tree(&cfg_f, repo_name, tree_id)
    });
    let gen = cache.begin_list_snapshots(repo);
    let snaps = restic_cmd::list_snapshots(&cfg, repo).expect("snapshots");
    let latest = snaps.last().expect("snap").clone();
    let roots: HashMap<_, _> = snaps
        .into_iter()
        .map(|s| (s.id.clone(), s.tree.clone()))
        .collect();
    assert!(cache.remember_snapshots(gen, roots));
    (cache, gen, latest.id)
}

#[test]
#[ignore]
fn z1_sftp_cold_warm_wall_times() {
    if std::env::var("RESTARILO_Z1_BENCH").is_err() {
        return;
    }
    let cfg = Arc::new(z1_cfg("/usr/bin/restic"));
    let path = deep_path();
    let snaps = restic_cmd::list_snapshots(&cfg, "sftp").expect("sftp snaps");
    let snap = snaps.last().expect("snap");
    println!("snapshot={} path={path}", &snap.id[..8.min(snap.id.len())]);

    // BEFORE (old restic ls browse): one `ls` per directory open.
    clear_restic_cache();
    let before_cold = timed_ms("BEFORE cold restic ls", || {
        let _ = restic_cmd::run_restic(&cfg, "sftp", &["ls", &snap.id, path, "--json"])?;
        Ok(())
    })
    .unwrap();
    let before_warm = timed_ms("BEFORE warm restic ls", || {
        let _ = restic_cmd::run_restic(&cfg, "sftp", &["ls", &snap.id, path, "--json"])?;
        Ok(())
    })
    .unwrap();

    // AFTER cold: empty memory + cleared restic cache → serial cat blob along walk.
    clear_restic_cache();
    let (cache, gen, snap_id) = setup_cache(cfg.clone(), "sftp");
    let after_cold = timed_ms("AFTER cold tree walk", || {
        let entries = list_dir(&cfg, &cache, gen, "sftp", &snap_id, path)?;
        assert!(
            entries.iter().any(|e| e.name == "leaf.txt"),
            "missing leaf.txt: {entries:?}"
        );
        Ok(())
    })
    .unwrap();

    // AFTER warm: same cache generation, memory hits only.
    let after_warm = timed_ms("AFTER warm tree walk", || {
        let entries = list_dir(&cfg, &cache, gen, "sftp", &snap_id, path)?;
        assert!(entries.iter().any(|e| e.name == "leaf.txt"));
        Ok(())
    })
    .unwrap();

    println!("--- Z1 SFTP wall-time table (ms) ---");
    println!("| case | cold | warm |");
    println!("| --- | ---: | ---: |");
    println!("| before (restic ls) | {before_cold} | {before_warm} |");
    println!("| after (tree cache) | {after_cold} | {after_warm} |");
}

fn write_wrapper(portable: &Path, log: &Path) -> PathBuf {
    let wrapper = portable.join("restic-wrapper.sh");
    let real = "/usr/bin/restic";
    let body = format!(
        r#"#!/bin/sh
log="{}"
# Log every `cat blob <id>` (tree fetches).
prev=""
for a in "$@"; do
  if [ "$prev" = "blob" ]; then
    echo "cat blob $a" >> "$log"
  fi
  prev="$a"
done
exec {real} "$@"
"#,
        log.display()
    );
    fs::write(&wrapper, body).unwrap();
    let _ = Command::new("chmod").args(["+x", wrapper.to_str().unwrap()]).status();
    wrapper
}

#[test]
#[ignore]
fn z1_portable_disk_wrapper_log() {
    if std::env::var("RESTARILO_Z1_BENCH").is_err() {
        return;
    }
    let base = PathBuf::from(format!(
        "/tmp/restarilo-portable-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base);
    let portable_a = base.join("app-a");
    let portable_b = base.join("app-b");
    fs::create_dir_all(portable_a.join("cache/tree")).unwrap();

    let log = portable_a.join("cat-blob.log");
    fs::write(&log, "").unwrap();
    let wrapper = write_wrapper(&portable_a, &log);

    let mut cfg = z1_cfg(wrapper.to_str().unwrap());
    cfg.cache_dir = Some(
        portable_a
            .join("cache/tree")
            .to_string_lossy()
            .into_owned(),
    );
    let cfg = Arc::new(cfg);
    let disk = Arc::new(TreeDiskCache::open(portable_a.join("cache/tree")));

    let path = deep_path();

    // Session 1: populate disk via SFTP browse.
    {
        let cache = TreeCache::new();
        cache.set_disk(Some(disk.clone()));
        let cfg_f = cfg.clone();
        cache.set_fetcher(move |repo_name, tree_id| {
            restic_tree::cat_tree(&cfg_f, repo_name, tree_id)
        });
        let gen = cache.begin_list_snapshots("sftp");
        let snaps = restic_cmd::list_snapshots(&cfg, "sftp").expect("snaps");
        let latest = snaps.last().unwrap().clone();
        let rid = restic_cmd::run_restic(&cfg, "sftp", &["cat", "config"])
            .ok()
            .and_then(|raw| {
                serde_json::from_str::<serde_json::Value>(&raw)
                    .ok()
                    .and_then(|v| v.get("id")?.as_str().map(|s| s.to_string()))
            });
        cache.set_session_repo_id(gen, rid);
        let roots: HashMap<_, _> = snaps
            .into_iter()
            .map(|s| (s.id.clone(), s.tree.clone()))
            .collect();
        assert!(cache.remember_snapshots(gen, roots));
        let entries = list_dir(&cfg, &cache, gen, "sftp", &latest.id, path).expect("populate");
        assert!(entries.iter().any(|e| e.name == "leaf.txt"));
    }

    let log_after_populate = fs::read_to_string(&log).unwrap();
    let lines_after_populate: Vec<_> = log_after_populate
        .lines()
        .filter(|l| l.starts_with("cat blob "))
        .collect();
    assert!(
        !lines_after_populate.is_empty(),
        "expected cat blob lines while populating"
    );
    println!("populate cat blob lines:\n{log_after_populate}");

    // Copy portable folder (exe+config+cache+wrapper+log).
    let status = Command::new("cp")
        .args(["-a", portable_a.to_str().unwrap(), portable_b.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());

    let log_b = portable_b.join("cat-blob.log");
    let wrapper_b = portable_b.join("restic-wrapper.sh");
    // Rewrite wrapper log path inside copied script.
    write_wrapper(&portable_b, &log_b);
    let before_restart = fs::read_to_string(&log_b).unwrap();
    let before_lines: Vec<_> = before_restart
        .lines()
        .filter(|l| l.starts_with("cat blob "))
        .map(|s| s.to_string())
        .collect();

    let mut cfg2 = z1_cfg(wrapper_b.to_str().unwrap());
    cfg2.cache_dir = Some(
        portable_b
            .join("cache/tree")
            .to_string_lossy()
            .into_owned(),
    );
    let cfg2 = Arc::new(cfg2);
    let disk2 = Arc::new(TreeDiskCache::open(portable_b.join("cache/tree")));

    // Session 2 from copy: reopen same deep dir — disk hits, no new cat blob for warm trees.
    {
        let cache = TreeCache::new();
        cache.set_disk(Some(disk2.clone()));
        let cfg_f = cfg2.clone();
        cache.set_fetcher(move |repo_name, tree_id| {
            restic_tree::cat_tree(&cfg_f, repo_name, tree_id)
        });
        let gen = cache.begin_list_snapshots("sftp");
        let snaps = restic_cmd::list_snapshots(&cfg2, "sftp").expect("snaps");
        let latest = snaps.last().unwrap().clone();
        let rid = restic_cmd::run_restic(&cfg2, "sftp", &["cat", "config"])
            .ok()
            .and_then(|raw| {
                serde_json::from_str::<serde_json::Value>(&raw)
                    .ok()
                    .and_then(|v| v.get("id")?.as_str().map(|s| s.to_string()))
            });
        cache.set_session_repo_id(gen, rid);
        let roots: HashMap<_, _> = snaps
            .iter()
            .map(|s| (s.id.clone(), s.tree.clone()))
            .collect();
        assert!(cache.remember_snapshots(gen, roots));
        let entries = list_dir(&cfg2, &cache, gen, "sftp", &latest.id, path).expect("warm reopen");
        assert!(entries.iter().any(|e| e.name == "leaf.txt"));

        let after = fs::read_to_string(&log_b).unwrap();
        let after_lines: Vec<_> = after
            .lines()
            .filter(|l| l.starts_with("cat blob "))
            .map(|s| s.to_string())
            .collect();
        println!("wrapper log after portable reopen:\n{after}");
        assert_eq!(
            before_lines, after_lines,
            "portable reopen must not append cat blob for warm tree ids"
        );

        // Changed tree in earlier snapshot lineage: open changed/ from previous snap if present.
        if snaps.len() >= 2 {
            let older = &snaps[snaps.len() - 2];
            let changed = "/tmp/restarilo-z1/data/changed";
            let _ = list_dir(&cfg2, &cache, gen, "sftp", &older.id, changed);
            let after2 = fs::read_to_string(&log_b).unwrap();
            println!("wrapper log after opening changed tree:\n{after2}");
            assert!(
                after2.lines().count() > after_lines.len(),
                "opening uncached changed tree must append cat blob line(s)"
            );
        }
    }

    println!("portable dir A={} B={}", portable_a.display(), portable_b.display());
}

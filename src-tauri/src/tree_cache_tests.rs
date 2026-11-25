use super::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

fn child(name: &str) -> TreeChild {
    TreeChild {
        name: name.into(),
        is_dir: false,
        size: Some(1),
        subtree: None,
    }
}

#[test]
fn coalesce_single_fetch() {
    let cache = Arc::new(TreeCache::new());
    let calls = Arc::new(AtomicUsize::new(0));
    let c2 = calls.clone();
    cache.set_fetcher(move |_r, _t| {
        c2.fetch_add(1, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(40));
        Ok(vec![child("a")])
    });
    let gen = cache.begin_list_snapshots("local");
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let c = cache.clone();
            thread::spawn(move || c.get_or_fetch(gen, "local", "tree1"))
        })
        .collect();
    for h in handles {
        assert!(h.join().unwrap().is_ok());
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn generation_rejects_stale_insert() {
    let cache = TreeCache::new();
    let gen = cache.begin_list_snapshots("a");
    cache.begin_list_snapshots("b");
    assert!(cache.insert_listing(gen, "t", vec![child("x")]).is_err());
}

#[test]
fn disk_hit_survives_memory_clear() {
    use crate::tree_disk_cache::TreeDiskCache;
    let dir = std::env::temp_dir().join(format!("restarilo-memdisk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let disk = Arc::new(TreeDiskCache::open(dir.clone()));
    let cache = TreeCache::new();
    cache.set_disk(Some(disk));
    let gen = cache.begin_list_snapshots("local");
    assert!(cache.set_session_repo_id(gen, Some("repoid".into())));
    let calls = Arc::new(AtomicUsize::new(0));
    let c2 = calls.clone();
    cache.set_fetcher(move |_r, _t| {
        c2.fetch_add(1, Ordering::SeqCst);
        Ok(vec![child("from-fetch")])
    });
    cache.get_or_fetch(gen, "local", "t1").unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let gen2 = cache.begin_list_snapshots("local");
    assert!(cache.set_session_repo_id(gen2, Some("repoid".into())));
    cache.set_fetcher(move |_r, _t| {
        Err("should not fetch".into())
    });
    let listing = cache.get_or_fetch(gen2, "local", "t1").unwrap();
    assert_eq!(listing.children[0].name, "from-fetch");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn late_list_snapshots_discarded() {
    let cache = TreeCache::new();
    let gen_a = cache.begin_list_snapshots("a");
    let gen_b = cache.begin_list_snapshots("b");
    let mut roots_a = HashMap::new();
    roots_a.insert("snapA".into(), "treeA".into());
    assert!(!cache.remember_snapshots(gen_a, roots_a));
    let mut roots_b = HashMap::new();
    roots_b.insert("snapB".into(), "treeB".into());
    assert!(cache.remember_snapshots(gen_b, roots_b));
    assert_eq!(cache.root_tree("snapB").as_deref(), Some("treeB"));
    assert!(cache.root_tree("snapA").is_none());
}

#[test]
fn skip_oversized_listing() {
    let cache = TreeCache::new();
    let gen = cache.begin_list_snapshots("local");
    let listing = cache
        .insert_listing(
            gen,
            "big",
            vec![TreeChild {
                name: "x".repeat(MEMORY_BUDGET + 10),
                is_dir: false,
                size: None,
                subtree: None,
            }],
        )
        .unwrap();
    assert!(cache.get_cached("big").is_none());
    assert_eq!(listing.children.len(), 1);
}

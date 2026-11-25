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
fn stale_generation_cannot_read_ram_hit() {
    let cache = TreeCache::new();
    let gen_a = cache.begin_list_snapshots("a");
    cache
        .insert_listing(gen_a, "shared", vec![child("from-a")])
        .unwrap();
    let gen_b = cache.begin_list_snapshots("b");
    cache
        .insert_listing(gen_b, "shared", vec![child("from-b")])
        .unwrap();
    match cache.get_or_fetch(gen_a, "a", "shared") {
        Err(err) => assert!(err.contains("stale"), "{err}"),
        Ok(_) => panic!("stale generation must not hit RAM from newer repo"),
    }
    let hit = cache.get_or_fetch(gen_b, "b", "shared").unwrap();
    assert_eq!(hit.children[0].name, "from-b");
}

#[test]
fn stale_fetch_does_not_write_new_repo_disk() {
    use crate::tree_disk_cache::TreeDiskCache;
    use std::sync::Barrier;
    let dir = std::env::temp_dir().join(format!("restarilo-stale-disk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let disk = Arc::new(TreeDiskCache::open(dir.clone()));
    let cache = Arc::new(TreeCache::new());
    cache.set_disk(Some(disk.clone()));
    let gen_a = cache.begin_list_snapshots("a");
    assert!(cache.set_session_repo_id(gen_a, Some("repoA".into())));
    let barrier = Arc::new(Barrier::new(2));
    let barrier_f = barrier.clone();
    cache.set_fetcher(move |_r, _t| {
        barrier_f.wait();
        Ok(vec![child("from-a")])
    });
    let c_fetch = cache.clone();
    let tid = "dd".repeat(32);
    let tid2 = tid.clone();
    let h = thread::spawn(move || c_fetch.get_or_fetch(gen_a, "a", &tid2));
    thread::sleep(Duration::from_millis(20));
    let gen_b = cache.begin_list_snapshots("b");
    assert!(cache.set_session_repo_id(gen_b, Some("repoB".into())));
    barrier.wait();
    let err = h.join().unwrap();
    assert!(err.is_err(), "stale generation must fail insert");
    assert!(disk.get("repoB", &tid).is_none());
    assert!(disk.get("repoA", &tid).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn disk_hit_survives_memory_clear() {
    use crate::tree_disk_cache::TreeDiskCache;
    let dir = std::env::temp_dir().join(format!("restarilo-memdisk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let disk = Arc::new(TreeDiskCache::open(dir.clone()));
    let cache = TreeCache::new();
    cache.set_disk(Some(disk.clone()));
    let gen = cache.begin_list_snapshots("local");
    assert!(cache.set_session_repo_id(gen, Some("repoid".into())));
    let calls = Arc::new(AtomicUsize::new(0));
    let c2 = calls.clone();
    cache.set_fetcher(move |_r, _t| {
        c2.fetch_add(1, Ordering::SeqCst);
        Ok(vec![child("from-fetch")])
    });
    let tid = "ee".repeat(32);
    cache.get_or_fetch(gen, "local", &tid).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(disk.get("repoid", &tid).is_some(), "expected disk write");
    let gen2 = cache.begin_list_snapshots("local");
    assert!(cache.set_session_repo_id(gen2, Some("repoid".into())));
    cache.set_fetcher(move |_r, _t| Err("should not fetch".into()));
    let listing = cache.get_or_fetch(gen2, "local", &tid).unwrap();
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

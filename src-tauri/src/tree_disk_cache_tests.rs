use super::*;
use std::sync::Arc;
use std::thread;

#[test]
fn round_trip_and_ignore_junk() {
    let dir = std::env::temp_dir().join(format!("restarilo-disk-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("junk.bin"), vec![0u8; 10]).unwrap();
    let cache = TreeDiskCache::open(dir.clone());
    let kids = vec![TreeChild {
        name: "a".into(),
        is_dir: true,
        entry_type: "dir".into(),
        size: None,
        mtime: None,
        subtree: Some("abc".into()),
    }];
    let tid = "aa".repeat(32);
    cache.put("repo1", &tid, &kids);
    let got = cache.get("repo1", &tid).unwrap();
    assert_eq!(got[0].name, "a");
    assert!(dir.join("junk.bin").exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn eviction_only_removes_valid_64hex_entries() {
    let dir = std::env::temp_dir().join(format!("restarilo-disk-evict-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let repo_dir = dir.join("v1").join("repo1");
    fs::create_dir_all(&repo_dir).unwrap();
    let foreign = repo_dir.join("deadbeef.json");
    fs::write(&foreign, vec![b'x'; 128]).unwrap();
    let cache = TreeDiskCache::open(dir.clone());
    let bulky = "n".repeat(1800);
    let mut tids = Vec::new();
    for i in 0..3u8 {
        let tid: String = format!("{i:02x}").repeat(32);
        cache.put(
            "repo1",
            &tid,
            &[TreeChild {
                name: bulky.clone(),
                is_dir: false,
                entry_type: "file".into(),
                size: None,
                mtime: None,
                subtree: None,
            }],
        );
        tids.push(tid);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(foreign.exists(), "deadbeef.json must survive eviction");
    let present: usize = tids.iter().filter(|t| cache.get("repo1", t).is_some()).count();
    assert!(
        present < tids.len(),
        "budget overflow must evict at least one valid entry (present={present})"
    );
    assert!(
        present >= 1,
        "at least one valid entry should remain after eviction"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn skip_oversized_single_entry() {
    let dir = std::env::temp_dir().join(format!("restarilo-disk-big-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let cache = TreeDiskCache::open(dir.clone());
    let kids = vec![TreeChild {
        name: "x".repeat((DISK_BUDGET as usize) + 8),
        is_dir: false,
        entry_type: "file".into(),
        size: None,
        mtime: None,
        subtree: None,
    }];
    let tid = "cc".repeat(32);
    cache.put("repo1", &tid, &kids);
    assert!(cache.get("repo1", &tid).is_none());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn concurrent_get_put_evict_no_panic() {
    let dir = std::env::temp_dir().join(format!(
        "restarilo-disk-conc-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let cache = Arc::new(TreeDiskCache::open(dir.clone()));
    let bulky = "n".repeat(1800);
    let mut handles = Vec::new();
    for t in 0..4u8 {
        let cache = cache.clone();
        let bulky = bulky.clone();
        handles.push(thread::spawn(move || {
            for i in 0..8u8 {
                let tid: String = format!("{:02x}", t * 16 + i).repeat(32);
                cache.put(
                    "repo1",
                    &tid,
                    &[TreeChild {
                        name: bulky.clone(),
                        is_dir: false,
                        entry_type: "file".into(),
                        size: None,
                        mtime: None,
                        subtree: None,
                    }],
                );
                let _ = cache.get("repo1", &tid);
            }
        }));
    }
    for h in handles {
        h.join().expect("worker panicked");
    }
    let _ = fs::remove_dir_all(&dir);
}

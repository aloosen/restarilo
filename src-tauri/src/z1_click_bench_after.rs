//! After-#8 click sequence: first-seen via emit sink or fallback response.
use crate::folder_index::{FolderChild, FolderIndex};
use crate::folder_index_emit::FolderIndexUpdated;
use crate::restic_ls::list_dir;
use crate::scan_scheduler::ScanScheduler;
use crate::z1_click_bench::{
    data_snapshot_id, path_check, setup_cache, temp_restic_cache, z1_cfg, Z1_CLICK_SEGMENTS,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn wait_first_seen(
    parent: &str,
    child: &str,
    sink: &Mutex<Vec<FolderIndexUpdated>>,
    done: &Mutex<Option<Vec<FolderChild>>>,
    t0: Instant,
) -> u128 {
    loop {
        if let Some(dirs) = done.lock().unwrap().as_ref() {
            if dirs.iter().any(|d| d.name == child) {
                return t0.elapsed().as_millis();
            }
        }
        {
            let g = sink.lock().unwrap();
            if g.iter().any(|ev| {
                ev.parent_path == parent && ev.dirs.iter().any(|d| d.name == child)
            }) {
                return t0.elapsed().as_millis();
            }
        }
        std::thread::sleep(Duration::from_millis(2));
        if t0.elapsed() > Duration::from_secs(120) {
            panic!("timeout waiting for {child} under {parent}");
        }
    }
}

#[test]
#[ignore]
fn z1_click_after_folder_dirs_sequence() {
    if std::env::var("RESTARILO_Z1_BENCH").is_err() {
        return;
    }
    let cfg = z1_cfg();
    let snap = data_snapshot_id(&cfg, "sftp");
    path_check(&cfg, "sftp", &snap);
    let _cache_dir = temp_restic_cache();
    let cfg = Arc::new(cfg);
    let (cache, gen) = setup_cache(cfg.clone(), "sftp", &snap);
    let index = Arc::new(FolderIndex::new());
    index.begin(gen);
    let sched = ScanScheduler::new(index.clone());
    sched.set_identity("sftp", &snap);
    let sink: Arc<Mutex<Vec<FolderIndexUpdated>>> = Arc::new(Mutex::new(Vec::new()));
    let sink2 = sink.clone();
    sched.set_test_sink(Some(Arc::new(move |ev| {
        sink2.lock().unwrap().push(ev);
    })));

    println!("| step | parent | child | ms |");
    println!("| --- | --- | --- | ---: |");
    let mut total = 0u128;
    for &(parent, child) in Z1_CLICK_SEGMENTS {
        sink.lock().unwrap().clear();
        let done = Arc::new(Mutex::new(None));
        let done2 = done.clone();
        let cfg2 = cfg.clone();
        let cache2 = cache.clone();
        let t0 = Instant::now();
        sched.prioritize(parent);
        let index2 = index.clone();
        let sched2 = sched.clone();
        let snap2 = snap.clone();
        let parent2 = parent.to_string();
        std::thread::spawn(move || {
            let entries =
                list_dir(&cfg2, &cache2, gen, "sftp", &snap2, &parent2).expect("fallback");
            let dirs: Vec<FolderChild> = entries
                .iter()
                .filter(|e| e.is_dir)
                .map(|e| FolderChild {
                    name: e.name.clone(),
                    path: e.path.clone(),
                })
                .collect();
            index2.upsert_dirs(gen, "sftp", &snap2, &parent2, &dirs, true);
            sched2.emit_dirs(gen, "sftp", &snap2, &parent2, &dirs, true);
            *done2.lock().unwrap() = Some(dirs);
        });
        let ms = wait_first_seen(parent, child, &sink, &done, t0);
        total += ms;
        println!("| after | `{parent}` | `{child}` | {ms} |");
    }
    println!("| after | **total** | | {total} |");
    println!("compare to D0 before total 8059 ms");
}

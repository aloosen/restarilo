use super::*;
use crate::folder_index::FolderChild;
use std::sync::Mutex;

#[test]
fn idle_starts_root_once_no_spam_after_success() {
    let idx = Arc::new(FolderIndex::new());
    idx.begin(1);
    let sched = ScanScheduler::new(idx);
    sched.set_identity("r", "s");
    let runs = Arc::new(Mutex::new(0u32));
    let runs2 = runs.clone();
    sched.set_runner(Some(Arc::new(move |job| {
        *runs2.lock().unwrap() += 1;
        job.index.upsert_dirs(
            job.generation,
            &job.repo,
            &job.snapshot_id,
            &job.root,
            &[FolderChild {
                name: "a".into(),
                path: "/a".into(),
            }],
            false,
        );
        ScanStop::Eof
    })));
    sched.maybe_idle_start();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(*runs.lock().unwrap(), 1);
    assert_eq!(sched.status(), ScanStatusKind::Success);
    sched.maybe_idle_start();
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(*runs.lock().unwrap(), 1);
}

#[test]
fn prioritize_cancels_and_reruns() {
    let idx = Arc::new(FolderIndex::new());
    idx.begin(1);
    let sched = ScanScheduler::new(idx);
    sched.set_identity("r", "s");
    let roots = Arc::new(Mutex::new(Vec::<String>::new()));
    let roots2 = roots.clone();
    sched.set_runner(Some(Arc::new(move |job| {
        roots2.lock().unwrap().push(job.root.clone());
        std::thread::sleep(Duration::from_millis(80));
        ScanStop::Eof
    })));
    sched.prioritize("/");
    std::thread::sleep(Duration::from_millis(20));
    sched.prioritize("/b");
    std::thread::sleep(Duration::from_millis(150));
    let r = roots.lock().unwrap().clone();
    assert!(r.iter().any(|x| x == "/"));
    assert!(r.iter().any(|x| x == "/b"));
}

#[test]
fn cooldown_blocks_idle_after_bounds() {
    let idx = Arc::new(FolderIndex::new());
    idx.begin(1);
    let sched = ScanScheduler::new(idx);
    *sched.cooldown_bounds.lock().unwrap() = Duration::from_secs(60);
    sched.set_identity("r", "s");
    let runs = Arc::new(Mutex::new(0u32));
    let runs2 = runs.clone();
    sched.set_runner(Some(Arc::new(move |_| {
        *runs2.lock().unwrap() += 1;
        ScanStop::NodeBudget
    })));
    sched.prioritize("/");
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(*runs.lock().unwrap(), 1);
    sched.maybe_idle_start();
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(*runs.lock().unwrap(), 1);
}

#[test]
fn emit_reaches_test_sink() {
    let idx = Arc::new(FolderIndex::new());
    idx.begin(1);
    let sched = ScanScheduler::new(idx);
    let seen = Arc::new(Mutex::new(0u32));
    let seen2 = seen.clone();
    sched.set_test_sink(Some(Arc::new(move |_| {
        *seen2.lock().unwrap() += 1;
    })));
    sched.emit_dirs(1, "r", "s", "/", &[], false);
    assert_eq!(*seen.lock().unwrap(), 1);
}

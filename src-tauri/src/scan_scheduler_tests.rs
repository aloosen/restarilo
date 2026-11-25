use super::*;
use crate::folder_index::FolderChild;
use crate::subtree_scan::ScanStop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::Duration;

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
    let first_saw_cancel = Arc::new(AtomicBool::new(false));
    let entered = Arc::new(Barrier::new(2));
    let roots2 = roots.clone();
    let cancel_flag = first_saw_cancel.clone();
    let entered2 = entered.clone();
    sched.set_runner(Some(Arc::new(move |job| {
        let is_first = {
            let mut r = roots2.lock().unwrap();
            r.push(job.root.clone());
            r.len() == 1
        };
        if is_first {
            entered2.wait();
            while !job.cancel.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(1));
            }
            cancel_flag.store(true, Ordering::SeqCst);
            return ScanStop::Cancelled;
        }
        ScanStop::Eof
    })));
    sched.prioritize("/");
    entered.wait();
    sched.prioritize("/b");
    std::thread::sleep(Duration::from_millis(100));
    assert!(first_saw_cancel.load(Ordering::SeqCst));
    let r = roots.lock().unwrap().clone();
    assert!(r.iter().any(|x| x == "/"));
    assert!(r.iter().any(|x| x == "/b"));
}

#[test]
fn bump_then_prioritize_hands_off_without_success_poison() {
    let idx = Arc::new(FolderIndex::new());
    idx.begin(1);
    let sched = ScanScheduler::new(idx);
    sched.set_identity("r", "s");
    let roots = Arc::new(Mutex::new(Vec::<String>::new()));
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let roots2 = roots.clone();
    let entered2 = entered.clone();
    let release2 = release.clone();
    sched.set_runner(Some(Arc::new(move |job| {
        roots2.lock().unwrap().push(job.root.clone());
        if job.root == "/" {
            entered2.wait();
            release2.wait();
            return ScanStop::Eof;
        }
        ScanStop::Eof
    })));
    sched.prioritize("/");
    entered.wait();
    sched.on_generation_bump(2);
    sched.prioritize("/new");
    release.wait();
    std::thread::sleep(Duration::from_millis(80));
    assert_eq!(roots.lock().unwrap().iter().filter(|x| *x == "/new").count(), 1);
    // Obsolete Eof for "/" must not leave Success cooldown that blocks a later prefer of "/".
    sched.prioritize("/");
    std::thread::sleep(Duration::from_millis(50));
    assert!(roots.lock().unwrap().iter().any(|x| x == "/"));
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

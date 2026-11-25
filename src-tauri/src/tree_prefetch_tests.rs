use super::*;
use crate::restic_cmd::SnapshotRow;
use crate::tree_cache::TreeCache;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

#[test]
fn newest_picks_parsed_instant_not_string() {
    let mk = |time: &str, tree: &str| SnapshotRow {
        id: tree.into(),
        short_id: tree.into(),
        time: time.into(),
        hostname: String::new(),
        paths: vec![],
        tags: vec![],
        tree: tree.into(),
    };
    let rows = vec![
        mk("2024-01-01T12:00:00+00:00", "old"),
        mk("2024-01-01T10:00:00-05:00", "new"),
    ];
    assert_eq!(newest_snapshot(&rows).unwrap().tree, "new");
}

#[test]
fn newest_uses_fractional_seconds() {
    let mk = |time: &str, tree: &str| SnapshotRow {
        id: tree.into(),
        short_id: tree.into(),
        time: time.into(),
        hostname: String::new(),
        paths: vec![],
        tags: vec![],
        tree: tree.into(),
    };
    let rows = vec![
        mk("2024-01-01T12:00:00.100Z", "earlier"),
        mk("2024-01-01T12:00:00.200Z", "later"),
    ];
    assert_eq!(newest_snapshot(&rows).unwrap().tree, "later");
}

#[test]
fn prefetch_user_guard_leaves_on_drop_and_panic() {
    let q = PrefetchQueue::new(Arc::new(TreeCache::new()));
    assert_eq!(q.user_active_count(), 0);
    {
        let _g = q.user_guard();
        assert_eq!(q.user_active_count(), 1);
    }
    assert_eq!(q.user_active_count(), 0);

    let q2 = q.clone();
    let caught = catch_unwind(AssertUnwindSafe(|| {
        let _g = q2.user_guard();
        panic!("force drop");
    }));
    assert!(caught.is_err());
    assert_eq!(q.user_active_count(), 0);
}

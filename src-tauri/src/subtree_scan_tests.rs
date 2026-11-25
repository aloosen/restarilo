use super::*;
use crate::subtree_scan_run::spawn_silent_deadline_test;
use std::io::Cursor;
use std::time::{Duration, Instant};

#[test]
fn multi_level_skips_root_keeps_siblings() {
    let root = "/tmp/data/shared";
    let json = format!(
        concat!(
            r#"{{"name":"shared","type":"dir","path":"{0}"}}"#,
            "\n",
            r#"{{"name":"a","type":"dir","path":"{0}/a"}}"#,
            "\n",
            r#"{{"name":"deep","type":"dir","path":"{0}/a/b/c/d/e/f/g/h/i/j"}}"#,
            "\n",
            r#"{{"name":"sib","type":"dir","path":"{0}/sib"}}"#,
            "\n",
            r#"{{"name":"f","type":"file","path":"{0}/a/f.txt"}}"#,
            "\n"
        ),
        root
    );
    let idx = FolderIndex::new();
    idx.begin(1);
    let stop = consume_ls_reader(
        Cursor::new(json),
        root,
        1,
        "r",
        "s",
        &idx,
        MAX_DIR_NODES,
        |_, _| {},
    );
    assert_eq!(stop, ScanStop::Eof);
    let under = idx.get(1, "r", "s", root).unwrap();
    assert!(under.dirs.iter().any(|d| d.name == "a"));
    assert!(under.dirs.iter().any(|d| d.name == "sib"));
    assert!(!under.dirs.iter().any(|d| d.name == "shared"));
    assert!(idx
        .get(1, "r", "s", &format!("{root}/a/b/c/d/e/f/g/h"))
        .is_none());
}

#[test]
fn node_budget_stops() {
    let root = "/r";
    let mut json = String::new();
    for i in 0..5 {
        json.push_str(&format!(
            "{{\"name\":\"d{i}\",\"type\":\"dir\",\"path\":\"{root}/d{i}\"}}\n"
        ));
    }
    let idx = FolderIndex::new();
    idx.begin(1);
    let stop = consume_ls_reader(Cursor::new(json), root, 1, "r", "s", &idx, 3, |_, _| {});
    assert_eq!(stop, ScanStop::NodeBudget);
    assert_eq!(idx.get(1, "r", "s", root).unwrap().dirs.len(), 3);
}

#[test]
fn silent_stdout_hard_kill_short_deadline() {
    let t0 = Instant::now();
    let stop = spawn_silent_deadline_test(Duration::from_millis(150));
    assert_eq!(stop, ScanStop::WallTime);
    assert!(t0.elapsed() < Duration::from_secs(3));
}

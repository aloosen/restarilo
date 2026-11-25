use super::*;
use std::fs;
use std::io::Write;

fn write_file(path: &Path, bytes: &[u8]) {
    let mut f = fs::File::create(path).unwrap();
    f.write_all(bytes).unwrap();
}

fn read_file(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap()
}

fn temp_pair(label: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "restarilo-dump-publish-{}-{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let out = dir.join("dest.bin");
    let temp = dir.join(format!(
        ".restarilo-dump-{}-dest.bin",
        std::process::id()
    ));
    (dir, out, temp)
}

#[test]
fn no_overwrite_leaves_dest_and_cleans_temp() {
    let (dir, out, temp) = temp_pair("no-ow");
    write_file(&out, b"original");
    write_file(&temp, b"staged");
    let err = publish_dump(&temp, &out, false).unwrap_err();
    assert!(
        err.starts_with("TARGET_EXISTS:"),
        "expected TARGET_EXISTS, got {err}"
    );
    assert_eq!(read_file(&out), b"original");
    assert!(!temp.exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn overwrite_inject_fail_preserves_dest_rename_over() {
    let (dir, out, temp) = temp_pair("ow-rename");
    write_file(&out, b"original");
    write_file(&temp, b"staged");
    set_force_sidecar(false);
    set_fail_pre_final_place(true);
    let err = publish_dump(&temp, &out, true).unwrap_err();
    assert!(err.contains("injected"), "{err}");
    assert_eq!(read_file(&out), b"original");
    assert!(!temp.exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn overwrite_inject_fail_preserves_dest_sidecar() {
    let (dir, out, temp) = temp_pair("ow-sidecar");
    write_file(&out, b"original");
    write_file(&temp, b"staged");
    set_force_sidecar(true);
    set_fail_pre_final_place(true);
    let err = publish_dump(&temp, &out, true).unwrap_err();
    assert!(err.contains("injected"), "{err}");
    assert_eq!(read_file(&out), b"original");
    assert!(!temp.exists());
    set_force_sidecar(false);
    let leftover: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(".restarilo-dump-replace-"))
        .collect();
    assert!(
        leftover.is_empty(),
        "sidecar leftovers after failed replace: {leftover:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn overwrite_success_replaces_bytes() {
    let (dir, out, temp) = temp_pair("ow-ok");
    write_file(&out, b"original");
    write_file(&temp, b"staged-new");
    set_force_sidecar(false);
    set_fail_pre_final_place(false);
    publish_dump(&temp, &out, true).unwrap();
    assert_eq!(read_file(&out), b"staged-new");
    assert!(!temp.exists());
    let _ = fs::remove_dir_all(&dir);
}

use crate::config::Config;
use crate::restic_killable::{
    commit_finish, ensure_slot, release_action, run_restic_killable,
};
use std::path::{Path, PathBuf};

fn parent_snapshot_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return "/".into();
    }
    match trimmed.rfind('/') {
        None | Some(0) => "/".into(),
        Some(i) => trimmed[..i].to_string(),
    }
}

fn escape_filter_literal(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            '[' => out.push_str("[[]"),
            '*' | '?' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn target_exists_err(path: &Path) -> String {
    format!("TARGET_EXISTS:{}", path.display())
}

fn ensure_absent_or_overwrite(path: &Path, overwrite: bool) -> Result<(), String> {
    if path.exists() && !overwrite {
        return Err(target_exists_err(path));
    }
    Ok(())
}

fn with_action_slot<T>(
    action_id: Option<String>,
    body: impl FnOnce(&str) -> Result<T, String>,
) -> Result<T, String> {
    let id = ensure_slot(action_id)?;
    let result = body(&id);
    release_action(&id);
    result
}

pub fn restore_path(
    cfg: &Config,
    repo_name: &str,
    snapshot_id: &str,
    path: &str,
    name: &str,
    target_dir: &str,
    overwrite: bool,
    action_id: Option<String>,
) -> Result<String, String> {
    with_action_slot(action_id, |id| {
        let chosen = PathBuf::from(target_dir);
        let dest = chosen.join(name);
        ensure_absent_or_overwrite(&dest, overwrite)?;
        let parent = parent_snapshot_path(path);
        let snapshot_ref = format!("{snapshot_id}:{parent}");
        let include = format!("/{}", escape_filter_literal(name));
        let dest_s = chosen.to_string_lossy();
        run_restic_killable(
            cfg,
            repo_name,
            &[
                "restore",
                &snapshot_ref,
                "--target",
                &dest_s,
                "--include",
                &include,
            ],
            id,
        )?;
        Ok(dest.to_string_lossy().into_owned())
    })
}

pub fn dump_path(
    cfg: &Config,
    repo_name: &str,
    snapshot_id: &str,
    path: &str,
    name: &str,
    is_dir: bool,
    target_dir: &str,
    overwrite: bool,
    action_id: Option<String>,
) -> Result<String, String> {
    with_action_slot(action_id, |id| {
        let out = if is_dir {
            PathBuf::from(target_dir).join(format!("{name}.zip"))
        } else {
            PathBuf::from(target_dir).join(name)
        };
        ensure_absent_or_overwrite(&out, overwrite)?;
        let file_name = out
            .file_name()
            .ok_or_else(|| format!("invalid dump target {}", out.display()))?
            .to_string_lossy();
        let temp = out.with_file_name(format!(
            ".restarilo-dump-{}-{file_name}",
            std::process::id()
        ));
        if temp.exists() {
            std::fs::remove_file(&temp)
                .map_err(|e| format!("failed to clear temp {}: {e}", temp.display()))?;
        }
        let temp_s = temp.to_string_lossy();
        let mut args: Vec<&str> = vec!["dump"];
        if is_dir {
            args.push("-a");
            args.push("zip");
        }
        args.extend(["--target", &temp_s, snapshot_id, path]);
        let dump_result = run_restic_killable(cfg, repo_name, &args, id);
        if let Err(e) = dump_result {
            let _ = std::fs::remove_file(&temp);
            return Err(e);
        }
        if let Err(e) = commit_finish(id) {
            let _ = std::fs::remove_file(&temp);
            return Err(e);
        }
        if out.exists() {
            std::fs::remove_file(&out)
                .map_err(|e| format!("failed to replace {}: {e}", out.display()))?;
        }
        std::fs::rename(&temp, &out).map_err(|e| {
            let _ = std::fs::remove_file(&temp);
            format!("failed to move dump into place {}: {e}", out.display())
        })?;
        Ok(out.to_string_lossy().into_owned())
    })
}

pub fn open_path(
    cfg: &Config,
    repo_name: &str,
    snapshot_id: &str,
    path: &str,
    name: &str,
    is_dir: bool,
    action_id: Option<String>,
) -> Result<String, String> {
    with_action_slot(action_id, |id| {
        if is_dir {
            return Err(format!("OPEN_DIRECTORY:{path}"));
        }
        let temp = std::env::temp_dir().join(format!(
            "restarilo-open-{}-{name}",
            std::process::id()
        ));
        if temp.exists() {
            std::fs::remove_file(&temp)
                .map_err(|e| format!("failed to clear temp {}: {e}", temp.display()))?;
        }
        let temp_s = temp.to_string_lossy();
        run_restic_killable(
            cfg,
            repo_name,
            &["dump", "--target", &temp_s, snapshot_id, path],
            id,
        )?;
        commit_finish(id)?;
        open::that(&temp).map_err(|e| format!("failed to open {}: {e}", temp.display()))?;
        Ok(temp.to_string_lossy().into_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::escape_filter_literal;

    #[test]
    fn escape_brackets_as_literal_class() {
        assert_eq!(escape_filter_literal("a[1].txt"), "a[[]1].txt");
    }

    #[test]
    fn escape_glob_metacharacters() {
        assert_eq!(escape_filter_literal("a*b?c"), r"a\*b\?c");
        assert_eq!(escape_filter_literal("plain.txt"), "plain.txt");
    }
}

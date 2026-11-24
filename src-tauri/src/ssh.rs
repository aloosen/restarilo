use crate::config::{exe_dir, resolve_path, RepoConfig};
use std::path::{Path, PathBuf};

/// Build restic `-o sftp.args=…` value so host/user/port stay in the repo URL.
pub fn sftp_args_for_repo(repo: &RepoConfig) -> Result<Option<String>, String> {
    let Some(raw) = repo.ssh_key.as_ref().filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let key = resolve_ssh_key(raw)?;
    Ok(Some(format!(
        "-i {} -o IdentitiesOnly=yes",
        shell_quote(&key)
    )))
}

pub fn resolve_ssh_key(raw: &str) -> Result<PathBuf, String> {
    let path = resolve_path(raw, &exe_dir()?)?;
    if !path.is_file() {
        return Err(format!("ssh_key not found: {}", path.display()));
    }
    Ok(path)
}

/// Quote a path for inclusion in `sftp.args` (spaces + Windows backslashes).
fn shell_quote(path: &Path) -> String {
    let s = path.to_string_lossy();
    if !s.contains([' ', '\t', '\'', '"', '\\']) {
        return s.into_owned();
    }
    let escaped = s.replace('\'', "'\\''");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::shell_quote;
    use std::path::Path;

    #[test]
    fn quotes_spaces() {
        let q = shell_quote(Path::new("/keys/my key"));
        assert_eq!(q, "'/keys/my key'");
    }

    #[test]
    fn quotes_windows_backslashes_with_spaces() {
        let q = shell_quote(Path::new(r"C:\My Keys\id_ed25519"));
        assert!(q.starts_with('\''));
        assert!(q.contains(r"C:\My Keys\id_ed25519") || q.contains("C:\\My Keys\\id_ed25519"));
    }
}

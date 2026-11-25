use crate::config::{exe_dir, Config, PasswordMode, RepoConfig};
use crate::restic_bin;
use crate::ssh;
use std::process::{Command, Stdio};

#[path = "restic_snapshots.rs"]
mod restic_snapshots;
pub use restic_snapshots::{list_snapshots, SnapshotRow};
pub(crate) use restic_snapshots::parse_snapshot_rows_json;

pub fn find_repo<'a>(cfg: &'a Config, name: &str) -> Result<&'a RepoConfig, String> {
    cfg.repos
        .iter()
        .find(|r| r.name == name)
        .ok_or_else(|| format!("unknown repo '{name}'"))
}

pub(crate) fn repository_env_value(path: &str) -> String {
    repository_env_value_for(path, cfg!(windows))
}

fn repository_env_value_for(path: &str, on_windows: bool) -> String {
    if on_windows && crate::smb_unc::is_unc_path(path) {
        crate::smb_unc::normalize_unc_path(path)
    } else {
        path.to_string()
    }
}

pub(crate) fn prepare_restic_command(cfg: &Config, repo_name: &str) -> Result<Command, String> {
    let repo = find_repo(cfg, repo_name)?;
    let bin = restic_bin::resolve_restic(Some(cfg))?;
    let exe = exe_dir()?;
    let mode = repo.password_mode(&exe)?;
    let mut cmd = Command::new(&bin);
    cmd.env_remove("RESTIC_PASSWORD");
    cmd.env_remove("RESTIC_PASSWORD_FILE");
    cmd.env_remove("RESTIC_PASSWORD_COMMAND");
    cmd.env_remove("RESTIC_REPOSITORY_FILE");
    cmd.env("RESTIC_REPOSITORY", repository_env_value(&repo.path));
    match &mode {
        PasswordMode::File(path) => {
            cmd.env("RESTIC_PASSWORD_FILE", path);
        }
        PasswordMode::Command(script) => {
            cmd.env("RESTIC_PASSWORD_COMMAND", script);
        }
        PasswordMode::InsecureNoPassword => {
            cmd.arg("--insecure-no-password");
        }
    }
    if let Some(sftp_args) = ssh::sftp_args_for_repo(repo)? {
        cmd.arg("-o");
        cmd.arg(format!("sftp.args={sftp_args}"));
    }
    Ok(cmd)
}

pub fn run_restic(cfg: &Config, repo_name: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = prepare_restic_command(cfg, repo_name)?;
    cmd.args(args);
    cmd.stdin(Stdio::null());
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run restic: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        let msg = if err.trim().is_empty() {
            out.trim().to_string()
        } else {
            err.trim().to_string()
        };
        return Err(if msg.is_empty() {
            format!("restic exited with {}", output.status)
        } else {
            msg
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ResticSection, RepoConfig};
    use serde::Deserialize;
    use std::path::Path;

    #[test]
    fn unc_env_normalizes_forward_slash_on_windows() {
        assert_eq!(
            repository_env_value_for("//nas/backups/repo", true),
            r"\\nas\backups\repo"
        );
        assert_eq!(
            repository_env_value_for(r"\\nas\backups\repo", true),
            r"\\nas\backups\repo"
        );
        assert_eq!(
            repository_env_value_for("//nas/backups/repo", false),
            "//nas/backups/repo"
        );
        assert_eq!(repository_env_value_for("/var/backup", true), "/var/backup");
        assert_eq!(
            repository_env_value_for("sftp:u@h:/r", true),
            "sftp:u@h:/r"
        );
    }

    #[test]
    fn parse_snapshot_without_tags_field() {
        let raw = r#"[{"id":"x","short_id":"abc123","time":"2024-01-01T00:00:00Z","hostname":"host","paths":["/data"],"tree":"abcd"}]"#;
        let parsed = restic_snapshots::parse_snapshot_rows_json(raw).unwrap();
        assert_eq!(parsed[0].tags.len(), 0);
        assert_eq!(parsed[0].short_id, "abc123");
        assert_eq!(parsed[0].tree, "abcd");
    }

    #[test]
    fn skip_snapshots_without_tree() {
        let raw = r#"[{"id":"x","short_id":"abc","time":"t","hostname":"h","paths":[],"tree":""}]"#;
        let parsed = restic_snapshots::parse_snapshot_rows_json(raw).unwrap();
        assert!(parsed.is_empty());
    }

    #[derive(Deserialize)]
    struct ExpectedIds {
        local: String,
        sftp: String,
    }

    #[test]
    fn live_local_and_sftp_snapshots() {
        let fix = Path::new("/tmp/restarilo-z1");
        let local = fix.join("local-repo");
        let sftp_path = fix.join("sftp-repo");
        let key = fix.join("my keys/id_ed25519");
        let expected_path = fix.join("expected.json");
        assert!(
            local.exists() && sftp_path.exists() && key.exists() && expected_path.exists(),
            "missing MVP fixtures under {} (see README MVP verification)",
            fix.display()
        );
        let expected: ExpectedIds = serde_json::from_str(
            &std::fs::read_to_string(&expected_path).expect("read expected.json"),
        )
        .expect("parse expected.json");
        let user = std::env::var("USER").expect("USER");
        let cfg = Config {
            restic: ResticSection {
                binary: Some("/usr/bin/restic".into()),
            },
            repos: vec![
                RepoConfig {
                    name: "local".into(),
                    path: local.to_string_lossy().into_owned(),
                    password_file: None,
                    password_command: None,
                    insecure_no_password: true,
                    ssh_key: None,
                },
                RepoConfig {
                    name: "sftp".into(),
                    path: format!("sftp:{user}@127.0.0.1:{}", sftp_path.to_string_lossy()),
                    password_file: None,
                    password_command: None,
                    insecure_no_password: true,
                    ssh_key: Some(key.to_string_lossy().into_owned()),
                },
            ],
            cache_dir: None,
        };
        let local_rows = list_snapshots(&cfg, "local").expect("local snapshots");
        assert!(
            local_rows
                .iter()
                .any(|r| r.id == expected.local || r.id.starts_with(&expected.local)),
            "expected local snapshot {}, got {local_rows:?}",
            expected.local
        );
        assert!(local_rows.iter().any(|r| r.tags.is_empty()));
        let sftp_rows = list_snapshots(&cfg, "sftp").expect("sftp snapshots");
        assert!(
            sftp_rows
                .iter()
                .any(|r| r.id == expected.sftp || r.id.starts_with(&expected.sftp)),
            "expected sftp snapshot {}, got {sftp_rows:?}",
            expected.sftp
        );
    }
}

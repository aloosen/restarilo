use crate::config::{exe_dir, Config, PasswordMode, RepoConfig};
use crate::restic_bin;
use crate::ssh;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRow {
    pub id: String,
    pub time: String,
    pub hostname: String,
    pub paths: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
struct ResticSnapshot {
    #[serde(default)]
    short_id: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    time: String,
    #[serde(default)]
    hostname: String,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
}

pub fn find_repo<'a>(cfg: &'a Config, name: &str) -> Result<&'a RepoConfig, String> {
    cfg.repos
        .iter()
        .find(|r| r.name == name)
        .ok_or_else(|| format!("unknown repo '{name}'"))
}

pub fn list_snapshots(cfg: &Config, repo_name: &str) -> Result<Vec<SnapshotRow>, String> {
    let raw = run_restic(cfg, repo_name, &["snapshots", "--json"])?;
    let parsed: Vec<ResticSnapshot> = serde_json::from_str(raw.trim())
        .map_err(|e| format!("failed to parse snapshots JSON: {e}"))?;
    Ok(parsed
        .into_iter()
        .map(|s| {
            let id = if !s.short_id.is_empty() {
                s.short_id
            } else {
                s.id
            };
            SnapshotRow {
                id,
                time: s.time,
                hostname: s.hostname,
                paths: s.paths,
                tags: s.tags,
            }
        })
        .collect())
}

pub fn run_restic(cfg: &Config, repo_name: &str, args: &[&str]) -> Result<String, String> {
    let repo = find_repo(cfg, repo_name)?;
    let bin = restic_bin::resolve_restic(Some(cfg))?;
    let exe = exe_dir()?;
    let mode = repo.password_mode(&exe)?;

    let mut cmd = Command::new(&bin);
    cmd.env_remove("RESTIC_PASSWORD");
    cmd.env_remove("RESTIC_PASSWORD_FILE");
    cmd.env_remove("RESTIC_PASSWORD_COMMAND");
    cmd.env("RESTIC_REPOSITORY", &repo.path);

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

    #[test]
    fn parse_snapshot_without_tags_field() {
        let raw = r#"[{"short_id":"abc123","time":"2024-01-01T00:00:00Z","hostname":"host","paths":["/data"]}]"#;
        let parsed: Vec<ResticSnapshot> = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed[0].tags.len(), 0);
        assert_eq!(parsed[0].short_id, "abc123");
    }

    #[test]
    fn live_local_and_sftp_snapshots() {
        let local = "/tmp/restarilo-z1/local-repo";
        let sftp_path = "/tmp/restarilo-z1/sftp-repo";
        let key = "/tmp/restarilo-z1/my keys/id_ed25519";
        if !std::path::Path::new(local).exists() || !std::path::Path::new(sftp_path).exists() {
            eprintln!("skip live fixtures (run Z1 setup first)");
            return;
        }
        let user = std::env::var("USER").unwrap_or_else(|_| "andreas".into());
        let cfg = Config {
            restic: ResticSection {
                binary: Some("/usr/bin/restic".into()),
            },
            repos: vec![
                RepoConfig {
                    name: "local".into(),
                    path: local.into(),
                    password_file: None,
                    password_command: None,
                    insecure_no_password: true,
                    ssh_key: None,
                },
                RepoConfig {
                    name: "sftp".into(),
                    path: format!("sftp:{user}@127.0.0.1:{sftp_path}"),
                    password_file: None,
                    password_command: None,
                    insecure_no_password: true,
                    ssh_key: Some(key.into()),
                },
            ],
        };
        let local_rows = list_snapshots(&cfg, "local").expect("local snapshots");
        assert!(
            !local_rows.is_empty(),
            "expected at least one local snapshot, got {local_rows:?}"
        );
        assert!(local_rows.iter().any(|r| r.tags.is_empty()));
        let sftp_rows = list_snapshots(&cfg, "sftp").expect("sftp snapshots");
        assert!(
            !sftp_rows.is_empty(),
            "expected at least one sftp snapshot, got {sftp_rows:?}"
        );
        assert!(!sftp_rows[0].id.is_empty() && !sftp_rows[0].time.is_empty());
    }
}

use crate::config::{exe_dir, Config, PasswordMode, RepoConfig};
use crate::restic_bin;
use crate::ssh;
use std::process::{Command, Stdio};

pub fn find_repo<'a>(cfg: &'a Config, name: &str) -> Result<&'a RepoConfig, String> {
    cfg.repos
        .iter()
        .find(|r| r.name == name)
        .ok_or_else(|| format!("unknown repo '{name}'"))
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

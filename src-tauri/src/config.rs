use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub restic: ResticSection,
    #[serde(default)]
    pub repos: Vec<RepoConfig>,
    /// Optional on-disk tree listing cache; relative to executable dir.
    #[serde(default)]
    pub cache_dir: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResticSection {
    pub binary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepoConfig {
    pub name: String,
    pub path: String,
    pub password_file: Option<String>,
    pub password_command: Option<String>,
    #[serde(default)]
    pub insecure_no_password: bool,
    pub ssh_key: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PasswordMode {
    File(PathBuf),
    Command(String),
    InsecureNoPassword,
}

impl RepoConfig {
    pub fn password_mode(&self, exe_dir: &Path) -> Result<PasswordMode, String> {
        let file = self.password_file.as_ref().filter(|s| !s.is_empty());
        let cmd = self.password_command.as_ref().filter(|s| !s.is_empty());
        let insecure = self.insecure_no_password;
        match (file, cmd, insecure) {
            (Some(p), None, false) => Ok(PasswordMode::File(resolve_path(p, exe_dir)?)),
            (None, Some(c), false) => Ok(PasswordMode::Command(c.clone())),
            (None, None, true) => Ok(PasswordMode::InsecureNoPassword),
            _ => Err(format!(
                "repo '{}': set exactly one of password_file, password_command, or insecure_no_password",
                self.name
            )),
        }
    }
}

pub fn load_config() -> Result<Config, String> {
    let path = find_config_path()?;
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let mut cfg: Config = toml::from_str(&text)
        .map_err(|e| format!("invalid config {}: {e}", path.display()))?;
    validate_config(&mut cfg)?;
    Ok(cfg)
}

fn validate_config(cfg: &mut Config) -> Result<(), String> {
    let exe_dir = exe_dir()?;
    if let Some(bin) = cfg.restic.binary.take() {
        let expanded = expand_tilde(&bin);
        if !Path::new(&expanded).is_absolute() {
            return Err(format!(
                "[restic].binary must be absolute after ~ expansion, got '{expanded}'"
            ));
        }
        cfg.restic.binary = Some(expanded);
    }
    let mut names = std::collections::HashSet::new();
    for repo in &cfg.repos {
        if !names.insert(repo.name.clone()) {
            return Err(format!("duplicate repo name '{}'", repo.name));
        }
        repo.password_mode(&exe_dir)?;
    }
    Ok(())
}

fn find_config_path() -> Result<PathBuf, String> {
    let exe = exe_dir()?;
    let next_to_exe = exe.join("config.toml");
    if next_to_exe.is_file() {
        return Ok(next_to_exe);
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            let p = PathBuf::from(xdg).join("restarilo").join("config.toml");
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    #[cfg(windows)]
    {
        if let Some(appdata) = dirs::config_dir() {
            let p = appdata.join("restarilo").join("config.toml");
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(home) = dirs::home_dir() {
            let p = home.join(".config").join("restarilo").join("config.toml");
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    Err("config.toml not found next to executable or in user config dir".into())
}

pub fn exe_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    exe.parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "executable has no parent directory".into())
}

pub fn resolve_path(raw: &str, exe_dir: &Path) -> Result<PathBuf, String> {
    let expanded = expand_tilde(raw);
    let path = PathBuf::from(&expanded);
    if path.is_absolute() {
        return Ok(path);
    }
    Ok(exe_dir.join(path))
}

/// `None` when absent, empty, or whitespace-only.
pub fn resolve_cache_dir(cfg: &Config) -> Result<Option<PathBuf>, String> {
    let Some(raw) = cfg.cache_dir.as_ref() else {
        return Ok(None);
    };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(resolve_path(raw.trim(), &exe_dir()?)?))
}

pub fn expand_tilde(raw: &str) -> String {
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().into_owned();
        }
    }
    if raw == "~" {
        if let Some(home) = dirs::home_dir() {
            return home.to_string_lossy().into_owned();
        }
    }
    raw.to_string()
}

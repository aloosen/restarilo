use crate::config::Config;
use crate::find_pattern::{is_direct_name_hit, rewrite_find_pattern};
use crate::restic_killable::{ensure_slot, release_action, run_restic_killable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindMatch {
    pub snapshot_id: String,
    pub path: String,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Deserialize)]
struct FindGroup {
    matches: Option<Vec<FindEntry>>,
    snapshot: String,
}

#[derive(Debug, Deserialize)]
struct FindEntry {
    path: String,
    #[serde(rename = "type")]
    entry_type: Option<String>,
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

fn basename(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return "/".into();
    }
    match trimmed.rfind('/') {
        None => trimmed.to_string(),
        Some(i) => trimmed[i + 1..].to_string(),
    }
}

fn flatten_groups(query: &str, groups: Vec<FindGroup>) -> Vec<FindMatch> {
    let mut out = Vec::new();
    for g in groups {
        for e in g.matches.unwrap_or_default() {
            if !is_direct_name_hit(query, &e.path) {
                continue;
            }
            let is_dir = e.entry_type.as_deref() == Some("dir");
            out.push(FindMatch {
                snapshot_id: g.snapshot.clone(),
                name: basename(&e.path),
                path: e.path,
                is_dir,
            });
        }
    }
    out
}

pub fn find_paths(
    cfg: &Config,
    repo_name: &str,
    query: &str,
    snapshot_id: Option<&str>,
    action_id: Option<String>,
) -> Result<Vec<FindMatch>, String> {
    with_action_slot(action_id, |id| {
        let pattern =
            rewrite_find_pattern(query).ok_or_else(|| "empty search query".to_string())?;
        let mut args: Vec<&str> = vec!["find", "--json", "-i"];
        if let Some(snap) = snapshot_id {
            args.push("--snapshot");
            args.push(snap);
        }
        args.push("--");
        args.push(pattern.as_str());
        let stdout = run_restic_killable(cfg, repo_name, &args, id)?;
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let groups: Vec<FindGroup> = serde_json::from_str(trimmed)
            .map_err(|e| format!("invalid find JSON: {e}"))?;
        Ok(flatten_groups(query, groups))
    })
}

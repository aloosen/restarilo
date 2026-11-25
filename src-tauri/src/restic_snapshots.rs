use crate::config::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRow {
    pub id: String,
    pub short_id: String,
    pub time: String,
    pub hostname: String,
    pub paths: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub tree: String,
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
    #[serde(default)]
    tree: String,
}

pub(crate) fn parse_snapshot_rows_json(raw: &str) -> Result<Vec<SnapshotRow>, String> {
    let parsed: Vec<ResticSnapshot> = serde_json::from_str(raw.trim())
        .map_err(|e| format!("failed to parse snapshots JSON: {e}"))?;
    Ok(parsed
        .into_iter()
        .filter(|s| !s.tree.trim().is_empty())
        .map(|s| {
            let short_id = if !s.short_id.is_empty() {
                s.short_id.clone()
            } else if s.id.len() >= 8 {
                s.id[..8].to_string()
            } else {
                s.id.clone()
            };
            SnapshotRow {
                id: s.id,
                short_id,
                time: s.time,
                hostname: s.hostname,
                paths: s.paths,
                tags: s.tags,
                tree: s.tree,
            }
        })
        .collect())
}

pub fn list_snapshots(cfg: &Config, repo_name: &str) -> Result<Vec<SnapshotRow>, String> {
    let raw = super::run_restic(cfg, repo_name, &["snapshots", "--json"])?;
    parse_snapshot_rows_json(&raw)
}

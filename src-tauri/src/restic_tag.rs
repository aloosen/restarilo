use crate::config::Config;
use crate::restic_killable::{ensure_slot, release_action, run_restic_killable};
use serde::Deserialize;
use serde::Serialize;

const FULL_SNAPSHOT_ID_LEN: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagChange {
    pub old_snapshot_id: String,
    pub new_snapshot_id: String,
}

#[derive(Debug, Deserialize)]
struct TagJsonLine {
    message_type: String,
    #[serde(default)]
    old_snapshot_id: String,
    #[serde(default)]
    new_snapshot_id: String,
    #[serde(default)]
    changed_snapshots: i64,
}

pub fn normalize_tags(tags: &[String]) -> Result<Vec<String>, String> {
    let mut out = Vec::with_capacity(tags.len());
    for raw in tags {
        let tag = raw.trim();
        if tag.is_empty() {
            return Err("tag must not be empty".into());
        }
        if tag.contains(',') {
            return Err(format!("tag must not contain a comma: '{tag}'"));
        }
        if out.iter().any(|t: &String| t == tag) {
            return Err(format!("duplicate tag '{tag}'"));
        }
        out.push(tag.to_string());
    }
    Ok(out)
}

pub fn validate_full_snapshot_id(snapshot_id: &str) -> Result<(), String> {
    let id = snapshot_id.trim();
    if id.len() != FULL_SNAPSHOT_ID_LEN {
        return Err(format!(
            "snapshot id must be the full {FULL_SNAPSHOT_ID_LEN}-character id"
        ));
    }
    if !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("snapshot id must be hexadecimal".into());
    }
    Ok(())
}

pub fn parse_tag_change_json(stdout: &str, expected_old: &str) -> Result<TagChange, String> {
    let mut change: Option<TagChange> = None;
    let mut summary_changed: Option<i64> = None;
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let msg: TagJsonLine = serde_json::from_str(line)
            .map_err(|e| format!("failed to parse tag JSON: {e}"))?;
        match msg.message_type.as_str() {
            "changed" => {
                if change.is_some() {
                    return Err("tag updated more than one snapshot".into());
                }
                if msg.old_snapshot_id != expected_old {
                    return Err(format!(
                        "tag updated unexpected snapshot (want {expected_old}, got {})",
                        msg.old_snapshot_id
                    ));
                }
                if msg.new_snapshot_id.is_empty() {
                    return Err("tag JSON missing new_snapshot_id".into());
                }
                change = Some(TagChange {
                    old_snapshot_id: msg.old_snapshot_id,
                    new_snapshot_id: msg.new_snapshot_id,
                });
            }
            "summary" => summary_changed = Some(msg.changed_snapshots),
            _ => {}
        }
    }
    match (change, summary_changed) {
        (Some(c), Some(n)) if n == 1 => Ok(c),
        (Some(_), Some(n)) => Err(format!("tag summary reports {n} changed snapshots")),
        (None, Some(0)) => Err("no snapshots were modified".into()),
        (None, Some(n)) => Err(format!("tag reported {n} changes without id mapping")),
        (Some(_), None) => Err("tag JSON missing summary".into()),
        (None, None) => Err("tag JSON produced no change mapping".into()),
    }
}

/// Build `restic tag` args. Clearing all tags uses `--remove` for each previous
/// tag: restic 0.19.1 treats `--set ""` as a no-op (`changed_snapshots: 0`).
pub fn build_tag_args<'a>(
    snapshot_id: &'a str,
    tags: &'a [String],
    previous_tags: &'a [String],
) -> Result<Vec<&'a str>, String> {
    let mut args = vec!["tag", "--json"];
    if tags.is_empty() {
        if previous_tags.is_empty() {
            return Err("no tags to remove".into());
        }
        for tag in previous_tags {
            args.push("--remove");
            args.push(tag.as_str());
        }
    } else {
        for tag in tags {
            args.push("--set");
            args.push(tag.as_str());
        }
    }
    args.push(snapshot_id);
    Ok(args)
}

pub fn set_snapshot_tags(
    cfg: &Config,
    repo_name: &str,
    snapshot_id: &str,
    tags: Vec<String>,
    previous_tags: Vec<String>,
    action_id: Option<String>,
) -> Result<TagChange, String> {
    // Reserve the slot before any fallible work so every exit path can release it.
    let id = ensure_slot(action_id)?;
    let result = (|| {
        validate_full_snapshot_id(snapshot_id)?;
        let tags = normalize_tags(&tags)?;
        let previous = normalize_tags(&previous_tags)?;
        let args = build_tag_args(snapshot_id, &tags, &previous)?;
        let stdout = run_restic_killable(cfg, repo_name, &args, &id)?;
        parse_tag_change_json(&stdout, snapshot_id)
    })();
    release_action(&id);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_rejects_empty_and_duplicate() {
        assert!(normalize_tags(&["".into()]).is_err());
        assert!(normalize_tags(&["a".into(), " a ".into()]).is_err());
        assert_eq!(
            normalize_tags(&["a".into(), "b".into()]).unwrap(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn normalize_rejects_comma() {
        assert!(normalize_tags(&["a,b".into()]).is_err());
    }

    #[test]
    fn validate_full_id() {
        let ok = "a".repeat(64);
        assert!(validate_full_snapshot_id(&ok).is_ok());
        assert!(validate_full_snapshot_id("abc").is_err());
        assert!(validate_full_snapshot_id(&("g".repeat(64))).is_err());
    }

    #[test]
    fn parse_changed_mapping() {
        let old = "a".repeat(64);
        let new = "b".repeat(64);
        let raw = format!(
            "{{\"message_type\":\"changed\",\"old_snapshot_id\":\"{old}\",\"new_snapshot_id\":\"{new}\"}}\n\
             {{\"message_type\":\"summary\",\"changed_snapshots\":1}}\n"
        );
        let c = parse_tag_change_json(&raw, &old).unwrap();
        assert_eq!(c.old_snapshot_id, old);
        assert_eq!(c.new_snapshot_id, new);
    }

    #[test]
    fn parse_rejects_wrong_old_id() {
        let old = "a".repeat(64);
        let other = "c".repeat(64);
        let new = "b".repeat(64);
        let raw = format!(
            "{{\"message_type\":\"changed\",\"old_snapshot_id\":\"{other}\",\"new_snapshot_id\":\"{new}\"}}\n\
             {{\"message_type\":\"summary\",\"changed_snapshots\":1}}\n"
        );
        assert!(parse_tag_change_json(&raw, &old).is_err());
    }

    #[test]
    fn build_args_clear_uses_remove() {
        let id = "a".repeat(64);
        let previous = vec!["one".into(), "two".into()];
        assert_eq!(
            build_tag_args(&id, &[], &previous).unwrap(),
            vec![
                "tag",
                "--json",
                "--remove",
                "one",
                "--remove",
                "two",
                id.as_str()
            ]
        );
    }

    #[test]
    fn build_args_set_multi() {
        let id = "a".repeat(64);
        let tags = vec!["one".into(), "two".into()];
        assert_eq!(
            build_tag_args(&id, &tags, &[]).unwrap(),
            vec!["tag", "--json", "--set", "one", "--set", "two", id.as_str()]
        );
    }

    #[test]
    fn build_args_clear_without_previous_fails() {
        let id = "a".repeat(64);
        assert!(build_tag_args(&id, &[], &[]).is_err());
    }
}

use crate::restic_cmd::SnapshotRow;
use crate::tree_cache::{CachedListing, TreeCache};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;

const MAX_CHILD_PREFETCH: usize = 4;

struct State {
    queue: VecDeque<(u64, String, String)>,
    busy: bool,
}

pub struct PrefetchQueue {
    state: Mutex<State>,
    cache: Arc<TreeCache>,
}

impl PrefetchQueue {
    pub fn new(cache: Arc<TreeCache>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                queue: VecDeque::new(),
                busy: false,
            }),
            cache,
        })
    }

    pub fn clear(&self) {
        self.state.lock().unwrap().queue.clear();
    }

    pub fn enqueue_newest_root(self: &Arc<Self>, generation: u64, repo: &str, rows: &[SnapshotRow]) {
        if let Some(row) = newest_snapshot(rows) {
            self.enqueue(generation, repo, &row.tree);
        }
    }

    pub fn enqueue_child_subtrees(
        self: &Arc<Self>,
        generation: u64,
        repo: &str,
        listing: &CachedListing,
    ) {
        let mut n = 0;
        for child in listing.children.iter() {
            if n >= MAX_CHILD_PREFETCH {
                break;
            }
            let Some(sub) = child.subtree.as_deref().filter(|s| !s.is_empty()) else {
                continue;
            };
            self.enqueue(generation, repo, sub);
            n += 1;
        }
    }

    fn enqueue(self: &Arc<Self>, generation: u64, repo: &str, tree_id: &str) {
        let start = {
            let mut g = self.state.lock().unwrap();
            if !g.queue.iter().any(|(_, _, t)| t == tree_id) {
                g.queue
                    .push_back((generation, repo.to_string(), tree_id.to_string()));
            }
            if g.busy {
                return;
            }
            g.busy = true;
            true
        };
        if start {
            let me = self.clone();
            thread::spawn(move || me.drain());
        }
    }

    fn drain(self: Arc<Self>) {
        loop {
            let next = {
                let mut g = self.state.lock().unwrap();
                match g.queue.pop_front() {
                    Some(item) => item,
                    None => {
                        g.busy = false;
                        return;
                    }
                }
            };
            let (generation, repo, tree_id) = next;
            if self.cache.generation() == generation {
                let _ = self.cache.get_or_fetch(generation, &repo, &tree_id);
            }
        }
    }
}

pub fn newest_snapshot(rows: &[SnapshotRow]) -> Option<&SnapshotRow> {
    let mut best: Option<(i64, usize)> = None;
    for (i, row) in rows.iter().enumerate() {
        let Some(ts) = parse_rfc3339_secs(&row.time) else {
            continue;
        };
        match best {
            None => best = Some((ts, i)),
            Some((b, _)) if ts > b => best = Some((ts, i)),
            Some((b, bi)) if ts == b && i < bi => best = Some((ts, i)),
            _ => {}
        }
    }
    best.map(|(_, i)| &rows[i])
}

fn parse_rfc3339_secs(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() < 19 || s.as_bytes().get(10) != Some(&b'T') {
        return None;
    }
    let y: i64 = s[0..4].parse().ok()?;
    let mo: i64 = s[5..7].parse().ok()?;
    let d: i64 = s[8..10].parse().ok()?;
    let h: i64 = s[11..13].parse().ok()?;
    let mi: i64 = s[14..16].parse().ok()?;
    let se: i64 = s[17..19].parse().ok()?;
    let mut off = 0i64;
    let mut rest = &s[19..];
    if rest.starts_with('.') {
        rest = rest.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    }
    if let Some(sign) = rest.chars().next().filter(|c| *c == '+' || *c == '-') {
        let body = &rest[1..];
        let (oh, om): (i64, i64) = if body.len() >= 5 && body.as_bytes()[2] == b':' {
            (body[0..2].parse().ok()?, body[3..5].parse().ok()?)
        } else if body.len() >= 4 {
            (body[0..2].parse().ok()?, body[2..4].parse().ok()?)
        } else {
            return None;
        };
        off = oh * 60 + om;
        if sign == '-' {
            off = -off;
        }
    }
    Some(days_from_civil(y, mo, d)? * 86400 + h * 3600 + mi * 60 + se - off * 60)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> Option<i64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_picks_parsed_instant_not_string() {
        let mk = |time: &str, tree: &str| SnapshotRow {
            id: tree.into(),
            short_id: tree.into(),
            time: time.into(),
            hostname: String::new(),
            paths: vec![],
            tags: vec![],
            tree: tree.into(),
        };
        let rows = vec![
            mk("2024-01-01T12:00:00+00:00", "old"),
            mk("2024-01-01T10:00:00-05:00", "new"),
        ];
        assert_eq!(newest_snapshot(&rows).unwrap().tree, "new");
    }
}

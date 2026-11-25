use crate::restic_cmd::SnapshotRow;

pub fn newest_snapshot(rows: &[SnapshotRow]) -> Option<&SnapshotRow> {
    let mut best: Option<(i128, usize)> = None;
    for (i, row) in rows.iter().enumerate() {
        let Some(ts) = parse_rfc3339_nanos(&row.time) else {
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

fn parse_rfc3339_nanos(s: &str) -> Option<i128> {
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
    let mut rest = &s[19..];
    let mut frac_nanos: i128 = 0;
    if let Some(stripped) = rest.strip_prefix('.') {
        let digits: String = stripped.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            let n = digits.len();
            let mut padded = digits;
            while padded.len() < 9 {
                padded.push('0');
            }
            padded.truncate(9);
            frac_nanos = padded.parse().ok()?;
            rest = &stripped[n..];
        } else {
            rest = stripped;
        }
    }
    let mut off = 0i64;
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
    let secs = days_from_civil(y, mo, d)? * 86400 + h * 3600 + mi * 60 + se - off * 60;
    Some(i128::from(secs) * 1_000_000_000 + frac_nanos)
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

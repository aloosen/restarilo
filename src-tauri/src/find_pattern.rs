pub fn rewrite_find_pattern(query: &str) -> Option<String> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }
    if q.chars().any(|c| matches!(c, '*' | '?' | '[')) {
        return Some(q.to_string());
    }
    if !q.contains('/') {
        return Some(format!("*{q}*"));
    }
    let segments: Vec<&str> = q.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return None;
    }
    if segments.len() == 1 {
        return Some(format!("*{}*", segments[0]));
    }
    let last = segments[segments.len() - 1];
    let mut out = String::from("**/");
    for (i, seg) in segments[..segments.len() - 1].iter().enumerate() {
        if i > 0 {
            out.push_str("/**/");
        }
        out.push_str(seg);
    }
    out.push_str("/**/*");
    out.push_str(last);
    out.push('*');
    Some(out)
}

fn basename<'a>(path: &'a str) -> &'a str {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return "/";
    }
    match trimmed.rfind('/') {
        None => trimmed,
        Some(i) => &trimmed[i + 1..],
    }
}

fn name_intent<'a>(query: &'a str) -> &'a str {
    let q = query.trim();
    if !q.contains('/') {
        return q;
    }
    q.split('/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(q)
}

fn glob_match_ci(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().flat_map(|c| c.to_lowercase()).collect();
    let t: Vec<char> = text.chars().flat_map(|c| c.to_lowercase()).collect();
    fn rec(p: &[char], t: &[char]) -> bool {
        match (p.first(), t.first()) {
            (None, None) => true,
            (Some('*'), _) => {
                (0..=t.len()).any(|i| rec(&p[1..], &t[i..]))
            }
            (Some('?'), Some(_)) => rec(&p[1..], &t[1..]),
            (Some(a), Some(b)) if a == b => rec(&p[1..], &t[1..]),
            _ => false,
        }
    }
    rec(&p, &t)
}

/// Keep hits whose **basename** matches the query intent so searching a folder
/// name does not list every file under that folder (restic path globs do).
pub fn is_direct_name_hit(query: &str, path: &str) -> bool {
    let intent = name_intent(query);
    if intent.is_empty() || intent.chars().all(|c| c == '/') {
        return false;
    }
    let name = basename(path);
    if intent.chars().any(|c| matches!(c, '*' | '?' | '[')) {
        return glob_match_ci(intent, name);
    }
    name.to_lowercase().contains(&intent.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{is_direct_name_hit, rewrite_find_pattern};

    #[test]
    fn rewrite_plain_substring() {
        assert_eq!(rewrite_find_pattern("report").as_deref(), Some("*report*"));
    }

    #[test]
    fn rewrite_path_like_two_segments() {
        assert_eq!(
            rewrite_find_pattern("documents/report").as_deref(),
            Some("**/documents/**/*report*")
        );
    }

    #[test]
    fn rewrite_trailing_slash_one_segment() {
        assert_eq!(
            rewrite_find_pattern("documents/").as_deref(),
            Some("*documents*")
        );
    }

    #[test]
    fn rewrite_leading_slash_one_segment() {
        assert_eq!(
            rewrite_find_pattern("/documents").as_deref(),
            Some("*documents*")
        );
    }

    #[test]
    fn rewrite_wildcard_passthrough() {
        assert_eq!(
            rewrite_find_pattern("*documents*report*").as_deref(),
            Some("*documents*report*")
        );
        assert_eq!(
            rewrite_find_pattern("file?.txt").as_deref(),
            Some("file?.txt")
        );
    }

    #[test]
    fn rewrite_empty_and_slashes_only() {
        assert_eq!(rewrite_find_pattern(""), None);
        assert_eq!(rewrite_find_pattern("   "), None);
        assert_eq!(rewrite_find_pattern("/"), None);
        assert_eq!(rewrite_find_pattern("//"), None);
    }

    #[test]
    fn direct_hit_keeps_folder_drops_children() {
        assert!(is_direct_name_hit("Documents", "/backup/Documents"));
        assert!(is_direct_name_hit("documents", "/backup/Documents"));
        assert!(!is_direct_name_hit(
            "Documents",
            "/backup/Documents/readme.txt"
        ));
        assert!(is_direct_name_hit(
            "readme",
            "/backup/Documents/readme.txt"
        ));
    }

    #[test]
    fn direct_hit_path_like_uses_last_segment() {
        assert!(is_direct_name_hit(
            "documents/report",
            "/x/Documents/Report.TXT"
        ));
        assert!(!is_direct_name_hit(
            "documents/report",
            "/x/Documents/Report.TXT/nested"
        ));
    }
}

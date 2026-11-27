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

#[cfg(test)]
mod tests {
    use super::rewrite_find_pattern;

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
}

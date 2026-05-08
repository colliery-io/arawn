//! Shared helpers for Drive feed templates.

use crate::error::FeedError;

/// Sanitize one path component from a Drive file or folder name into
/// a filesystem-safe segment.
///
/// Drive names are user-controlled, so they can contain `/`, `..`,
/// NUL bytes, or other surprises. This function maps anything dodgy
/// to `_`. We intentionally do not URL-encode — the goal is a name
/// you can `cd` into and recognize, not one that round-trips perfectly.
pub fn sanitize_path_component(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return "_".into();
    }
    trimmed
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

/// Confirm `candidate` lives strictly under `root`. Defense-in-depth
/// against a hostile filename ever escaping the feed_dir via `..`
/// — even though [`sanitize_path_component`] should already strip
/// those, mirror semantics let us delete files, so we double-check
/// here.
pub fn is_under(root: &std::path::Path, candidate: &std::path::Path) -> bool {
    let root = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    // candidate may not exist yet — walk parents until one does, then
    // resolve.
    let mut anchor = candidate.to_path_buf();
    loop {
        if anchor.exists() {
            break;
        }
        match anchor.parent() {
            Some(p) if p != anchor => anchor = p.to_path_buf(),
            _ => return false,
        }
    }
    let anchor = match anchor.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    anchor.starts_with(&root)
}

/// Map an `md5_checksum` (binary) or `modified_time` (Google natives)
/// into the cursor's per-file change-detection token. We don't care
/// what shape it is, only that the same file with the same content
/// produces the same string.
pub fn change_token(md5: Option<&str>, modified_time: Option<&str>) -> String {
    match (md5, modified_time) {
        (Some(m), _) if !m.is_empty() => format!("md5:{m}"),
        (_, Some(t)) if !t.is_empty() => format!("mtime:{t}"),
        _ => "unknown".into(),
    }
}

/// Read a `modifiedTime` ISO string into an `i64` ms-since-epoch for
/// day partitioning by send time.
pub fn modified_to_yyyy_mm_dd(modified_time: Option<&str>) -> Result<String, FeedError> {
    let s = modified_time.ok_or_else(|| {
        FeedError::Schema("file missing modifiedTime".into())
    })?;
    let dt = chrono::DateTime::parse_from_rfc3339(s)
        .map_err(|e| FeedError::Schema(format!("bad modifiedTime '{s}': {e}")))?;
    Ok(dt
        .with_timezone(&chrono::Utc)
        .format("%Y-%m-%d")
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_separators_and_traversal() {
        assert_eq!(sanitize_path_component("normal name"), "normal name");
        assert_eq!(sanitize_path_component(".."), "_");
        assert_eq!(sanitize_path_component("."), "_");
        assert_eq!(sanitize_path_component(""), "_");
        assert_eq!(sanitize_path_component("a/b"), "a_b");
        assert_eq!(sanitize_path_component("a\\b"), "a_b");
        assert_eq!(sanitize_path_component("a\0b"), "a_b");
    }

    #[test]
    fn change_token_prefers_md5() {
        assert_eq!(change_token(Some("abc"), Some("2026-01-01")), "md5:abc");
        assert_eq!(change_token(None, Some("2026-01-01")), "mtime:2026-01-01");
        assert_eq!(change_token(None, None), "unknown");
        assert_eq!(change_token(Some(""), Some("2026-01-01")), "mtime:2026-01-01");
    }

    #[test]
    fn modified_to_day_basic() {
        assert_eq!(
            modified_to_yyyy_mm_dd(Some("2026-05-08T12:34:56Z")).unwrap(),
            "2026-05-08"
        );
        assert!(modified_to_yyyy_mm_dd(None).is_err());
        assert!(modified_to_yyyy_mm_dd(Some("not a date")).is_err());
    }
}

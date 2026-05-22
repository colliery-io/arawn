
use arawn_storage::Store;

pub fn validate_github_scope_scheme(feed_id: &str) -> Result<(), String> {
    if let Some(rest) = feed_id.strip_prefix("github:repo:") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(format!(
                "invalid github:repo binding `{feed_id}` — expected \
                 `github:repo:<owner>/<name>` (e.g. `github:repo:openai/codex`)"
            ));
        }
        return Ok(());
    }
    if let Some(rest) = feed_id.strip_prefix("github:org:") {
        if rest.is_empty() || rest.contains('/') {
            return Err(format!(
                "invalid github:org binding `{feed_id}` — expected \
                 `github:org:<owner>` (e.g. `github:org:openai`)"
            ));
        }
        return Ok(());
    }
    Ok(())
}

/// Returns true if `feed_id` is a github scope-binding (either
/// `github:repo:*` or `github:org:*`), false otherwise. Used by the
/// bind-backfill hook to short-circuit feed-store lookup for these
/// synthetic ids.
pub fn is_github_scope_binding(feed_id: &str) -> bool {
    feed_id.starts_with("github:repo:") || feed_id.starts_with("github:org:")
}

/// Parsed github scope binding. `None` for non-github bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubScope {
    Repo { owner: String, name: String },
    Org { owner: String },
}

pub fn parse_github_scope(feed_id: &str) -> Option<GithubScope> {
    if let Some(rest) = feed_id.strip_prefix("github:repo:") {
        let mut parts = rest.splitn(2, '/');
        let owner = parts.next()?.to_string();
        let name = parts.next()?.to_string();
        if owner.is_empty() || name.is_empty() {
            return None;
        }
        return Some(GithubScope::Repo { owner, name });
    }
    if let Some(rest) = feed_id.strip_prefix("github:org:") {
        if rest.is_empty() || rest.contains('/') {
            return None;
        }
        return Some(GithubScope::Org {
            owner: rest.to_string(),
        });
    }
    None
}

/// Walk active workstreams, return `(workstream_name, binding)` for
/// every binding matching the predicate.
pub(super) fn find_workstreams_binding(
    store: &Store,
    matcher: impl Fn(&str) -> bool,
) -> Vec<(String, String)> {
    let workstreams = store.list_workstreams().unwrap_or_default();
    let mut out = Vec::new();
    for ws in workstreams {
        for binding in &ws.bindings {
            if matcher(binding) {
                out.push((ws.name.clone(), binding.clone()));
            }
        }
    }
    out
}

/// Insert (or refresh) a `github/repo-mirror` feed record by writing
/// to the `feeds` table via raw SQL. Idempotent — if a record with
/// the same id already exists, returns Ok without modifying it.
///
/// Direct SQL (instead of `arawn_feeds::FeedStore`) avoids an
/// arawn-engine → arawn-feeds dependency cycle: arawn-feeds already
/// depends on arawn-engine via arawn-integrations → arawn-service.
pub(super) fn upsert_repo_mirror_feed(
    store: &Store,
    feed_id: &str,
    owner: &str,
    repo: &str,
) -> Result<(), arawn_storage::StorageError> {
    use arawn_storage::StorageError;
    use rusqlite::OptionalExtension;
    let db = store.database();
    let conn = db.conn();
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM feeds WHERE id = ?1",
            rusqlite::params![feed_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| StorageError::InvalidOperation(format!("feeds lookup: {e}")))?
        .unwrap_or(false);
    if exists {
        return Ok(());
    }
    let params_json = serde_json::to_string(&serde_json::json!({
        "owner": owner,
        "name": repo,
    }))
    .map_err(|e| StorageError::InvalidOperation(format!("serialize params: {e}")))?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO feeds (id, template, params, cadence, enabled, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            feed_id,
            "github/repo-mirror",
            params_json,
            "*/30 * * * *",
            1,
            &now,
            &now,
        ],
    )
    .map_err(|e| StorageError::InvalidOperation(format!("insert feed: {e}")))?;
    Ok(())
}

/// Drop a feed record by raw SQL. Idempotent.
pub(super) fn delete_feed(store: &Store, feed_id: &str) -> Result<(), arawn_storage::StorageError> {
    use arawn_storage::StorageError;
    let db = store.database();
    db.conn()
        .execute("DELETE FROM feeds WHERE id = ?1", rusqlite::params![feed_id])
        .map_err(|e| StorageError::InvalidOperation(format!("delete feed: {e}")))?;
    Ok(())
}

pub(super) fn extract_json_block(raw: &str) -> Option<&str> {
    let bytes = raw.as_bytes();
    let mut depth = 0i32;
    let mut start: Option<usize> = None;
    let mut open: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate() {
        match (open, b) {
            (None, b'{') | (None, b'[') => {
                start = Some(i);
                open = Some(b);
                depth = 1;
            }
            (Some(b'{'), b'{') | (Some(b'['), b'[') => depth += 1,
            (Some(b'{'), b'}') | (Some(b'['), b']') => {
                depth -= 1;
                if depth == 0 {
                    let end = i + 1;
                    return Some(&raw[start.unwrap()..end]);
                }
            }
            _ => {}
        }
    }
    None
}

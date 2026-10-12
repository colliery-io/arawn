//! Doc comments projection — `drive_comments` (COLLIERY-I-0533).
//!
//! Mirror layout (from `arawn-feeds::templates::drive::comments`):
//! ```text
//! <feed_dir>/<file id>/<comment id>.json
//! ```
//! One row per open comment thread that is for the user, keyed by
//! `<file id>/<comment id>`. The feed rewrites a snapshot when the thread
//! changes and removes it when the thread is resolved or no longer for
//! the user, so the dispatcher upserts every row and drops the rows whose
//! snapshot is gone.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::warn;

use crate::error::ProjectionError;
use crate::types::{Projection, ProjectionRow};

pub const FEED_TYPE: &str = "drive_comments";

#[derive(Debug, Clone)]
pub struct DriveCommentProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String,        // <file id>/<comment id>
    pub source_ts: DateTime<Utc>, // modifiedTime of the thread
    pub file_id: String,
    pub comment_id: String,
    pub doc_title: String,
    pub link: Option<String>,
    /// `mentioned | assigned | owner_unanswered`.
    pub reason: String,
    pub author: Option<String>,
    pub content: String,
    pub quoted: Option<String>,
    pub reply_count: u64,
    pub last_reply_author: Option<String>,
    pub last_reply: Option<String>,
}

impl Projection for DriveCommentProjection {
    fn feed_type(&self) -> &'static str {
        FEED_TYPE
    }

    fn row(&self) -> ProjectionRow {
        let mut body_text = format!(
            "{}: {}",
            self.author.as_deref().unwrap_or("?"),
            self.content
        );
        if let Some(q) = self.quoted.as_deref().filter(|q| !q.is_empty()) {
            body_text.push_str("\n\nOn: ");
            body_text.push_str(q);
        }
        if let Some(r) = self.last_reply.as_deref() {
            body_text.push_str(&format!(
                "\n\n{}: {r}",
                self.last_reply_author.as_deref().unwrap_or("?")
            ));
        }
        let metadata = serde_json::json!({
            "file_id": self.file_id,
            "comment_id": self.comment_id,
            "doc_title": self.doc_title,
            "link": self.link,
            "reason": self.reason,
            "author": self.author,
            "content": self.content,
            "quoted": self.quoted,
            "reply_count": self.reply_count,
            "last_reply_author": self.last_reply_author,
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title: self.doc_title.clone(),
            body_text,
            feed_type: FEED_TYPE.to_string(),
            metadata,
        }
    }
}

fn hash_id(feed_id: &str, source: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    feed_id.hash(&mut h);
    "::".hash(&mut h);
    source.hash(&mut h);
    format!("dc-{:016x}", h.finish())
}

fn str_at<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

/// Parse one comment snapshot. `None` (with a warning) when the
/// required `file_id` / `comment_id` are missing.
pub fn from_snapshot(feed_id: &str, v: &Value) -> Option<DriveCommentProjection> {
    let (Some(file_id), Some(comment_id)) = (str_at(v, "file_id"), str_at(v, "comment_id")) else {
        warn!("drive comment: snapshot without file_id/comment_id");
        return None;
    };
    let source_id = format!("{file_id}/{comment_id}");
    let source_ts = str_at(v, "modified_time")
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let opt = |k: &str| str_at(v, k).map(String::from);
    Some(DriveCommentProjection {
        id: hash_id(feed_id, &source_id),
        feed_id: feed_id.to_string(),
        source_id,
        source_ts,
        file_id: file_id.to_string(),
        comment_id: comment_id.to_string(),
        doc_title: str_at(v, "doc_title").unwrap_or_default().to_string(),
        link: opt("link"),
        reason: str_at(v, "reason").unwrap_or_default().to_string(),
        author: opt("author"),
        content: str_at(v, "content").unwrap_or_default().to_string(),
        quoted: opt("quoted"),
        reply_count: v.get("reply_count").and_then(Value::as_u64).unwrap_or(0),
        last_reply_author: opt("last_reply_author"),
        last_reply: opt("last_reply"),
    })
}

/// Walk `<feed_dir>/<file id>/<comment id>.json`.
pub fn walk_feed_dir(
    feed_id: &str,
    feed_dir: &Path,
) -> Result<Vec<DriveCommentProjection>, ProjectionError> {
    let mut out = Vec::new();
    let docs = match std::fs::read_dir(feed_dir) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };
    for doc in docs {
        let doc = doc?;
        if !doc.file_type()?.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(doc.path())? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
            match serde_json::from_slice::<Value>(&bytes) {
                Ok(v) => out.extend(from_snapshot(feed_id, &v)),
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "skip unparseable drive comment")
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ProjectionStore;
    use serde_json::json;

    fn snapshot(comment_id: &str) -> Value {
        json!({
            "file_id": "F1", "comment_id": comment_id, "doc_title": "Q3 plan",
            "mime_type": "application/vnd.google-apps.document", "owned_by_me": true,
            "link": format!("https://docs.google.com/document/d/F1/edit?disco={comment_id}"),
            "reason": "mentioned", "author": "Robin",
            "content": "Can you confirm the hiring budget?",
            "quoted": "Headcount: 4", "assignee": null,
            "created_time": "2026-10-05T09:00:00+00:00",
            "modified_time": "2026-10-05T10:00:00+00:00",
            "reply_count": 1, "last_reply_author": "Sam", "last_reply": "Same question here"
        })
    }

    fn write(dir: &Path, comment_id: &str) {
        let doc = dir.join("F1");
        std::fs::create_dir_all(&doc).unwrap();
        std::fs::write(
            doc.join(format!("{comment_id}.json")),
            snapshot(comment_id).to_string(),
        )
        .unwrap();
    }

    #[test]
    fn snapshot_walks_into_a_row_found_by_fts() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "AAA");
        std::fs::write(tmp.path().join("meta.json"), "{}").unwrap();

        let rows = walk_feed_dir("drive-comments", tmp.path()).unwrap();
        assert_eq!(rows.len(), 1);
        let p = &rows[0];
        assert_eq!(p.source_id, "F1/AAA");
        let row = p.row();
        assert_eq!(row.title, "Q3 plan");
        assert_eq!(row.metadata["reason"], "mentioned");
        assert_eq!(
            row.metadata["link"],
            "https://docs.google.com/document/d/F1/edit?disco=AAA"
        );
        assert!(row.body_text.contains("Robin: Can you confirm"));
        assert!(row.body_text.contains("Sam: Same question here"));

        let store = ProjectionStore::in_memory().unwrap();
        store.write_batch(&rows).unwrap();
        assert_eq!(
            store.fts_search(FEED_TYPE, "hiring budget", 5).unwrap(),
            std::slice::from_ref(&p.id)
        );
    }

    #[test]
    fn a_removed_snapshot_takes_its_row_with_it() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ProjectionStore::in_memory().unwrap();
        write(tmp.path(), "AAA");
        write(tmp.path(), "BBB");
        crate::project_feed_dir(&store, "drive/comments", "drive-comments", tmp.path()).unwrap();
        assert_eq!(store.count(FEED_TYPE).unwrap(), 2);
        // Not walked as drive_files.
        assert_eq!(store.count(crate::drive::FEED_TYPE).unwrap(), 0);

        std::fs::remove_file(tmp.path().join("F1/BBB.json")).unwrap();
        crate::project_feed_dir(&store, "drive/comments", "drive-comments", tmp.path()).unwrap();
        assert_eq!(store.count(FEED_TYPE).unwrap(), 1);
        assert_eq!(store.fts_search(FEED_TYPE, "hiring", 5).unwrap().len(), 1);
    }

    #[test]
    fn snapshot_without_identity_is_skipped() {
        assert!(from_snapshot("f", &json!({"content": "x"})).is_none());
    }
}

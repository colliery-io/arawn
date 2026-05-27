//! Filesystem projection — `filesystem_signals`.
//!
//! Mirror layout (from `arawn-feeds::clients::filesystem`):
//! ```text
//! <feed_dir>/
//!   ├── meta.json        # runtime-managed cursor (fingerprint map)
//!   └── signals.jsonl    # one JSON object per change event
//! ```
//!
//! Each `signals.jsonl` line is an immutable change record:
//! `{ source, path, rel_path, event, ts, size_bytes, mtime }`. We read
//! every line and project one row per event. The row's `source_id` is a
//! stable hash of the event's identifying fields, so re-projecting the
//! same (append-only) log is idempotent — already-written events fall
//! out via `missing_source_ids` and only newly-appended events insert.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::error::ProjectionError;
use crate::types::{Projection, ProjectionRow};

pub const FEED_TYPE: &str = "filesystem_signals";

#[derive(Debug, Clone)]
pub struct FilesystemSignalProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String,
    pub source_ts: DateTime<Utc>,
    /// Path relative to the watched root — the primary searchable text.
    pub rel_path: String,
    /// Just the file name, used as the row title.
    pub name: String,
    /// `created` / `modified` / `deleted`.
    pub event: String,
    /// Absolute path on disk.
    pub path: String,
    /// Absolute path of the watched root this signal came from.
    pub source: String,
    pub size_bytes: Option<u64>,
    pub mtime: Option<i64>,
    /// The file's text content at scan time (UTF-8, truncated to
    /// [`MAX_BODY_BYTES`]). Empty for `deleted` events and for
    /// binary / unreadable files. This is what makes a watched
    /// transcript's *content* searchable via `feed_search`.
    pub body_text: String,
}

/// Cap on the indexed body. Transcripts are text; anything larger than
/// this is truncated for the search index (the file on disk is
/// untouched).
const MAX_BODY_BYTES: usize = 256 * 1024;

impl Projection for FilesystemSignalProjection {
    fn feed_type(&self) -> &'static str {
        FEED_TYPE
    }

    fn row(&self) -> ProjectionRow {
        let metadata = serde_json::json!({
            "source": self.source,
            "path": self.path,
            "rel_path": self.rel_path,
            "event": self.event,
            "size_bytes": self.size_bytes,
            "mtime": self.mtime,
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title: self.name.clone(),
            // Index the relative path + event + the file body so a
            // search for a filename, a folder segment, "deleted", or
            // any phrase inside the transcript all hit.
            body_text: format!("{}\n{}\n{}", self.rel_path, self.event, self.body_text),
            feed_type: FEED_TYPE.to_string(),
            metadata,
        }
    }
}

/// Read a file as UTF-8 text, truncated to [`MAX_BODY_BYTES`]. Returns
/// an empty string for missing / binary / unreadable files so a
/// metadata-only row is still written.
fn read_text_body(path: &Path) -> String {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return String::new(),
    };
    let cap = (meta.len() as usize).min(MAX_BODY_BYTES);
    use std::io::Read;
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return String::new(),
    };
    let mut buf = vec![0u8; cap];
    let n = match f.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return String::new(),
    };
    buf.truncate(n);
    String::from_utf8(buf).unwrap_or_default()
}

/// Stable projection id from `feed_id` + the event's `source_id`.
pub fn projection_id(feed_id: &str, source_id: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    feed_id.hash(&mut h);
    "::".hash(&mut h);
    source_id.hash(&mut h);
    format!("fs-{:016x}", h.finish())
}

/// Stable per-event source id: a hash over the immutable identifying
/// fields of the signal. Two distinct events (e.g. a `created` then a
/// later `modified` of the same path) hash differently, so each gets
/// its own row; re-reading the same log line hashes identically, so it
/// dedups.
fn event_source_id(
    path: &str,
    event: &str,
    ts: &str,
    mtime: Option<i64>,
    size: Option<u64>,
) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    event.hash(&mut h);
    ts.hash(&mut h);
    mtime.hash(&mut h);
    size.hash(&mut h);
    format!("{:016x}", h.finish())
}

pub fn walk_feed_dir(
    feed_id: &str,
    feed_dir: &Path,
) -> Result<Vec<FilesystemSignalProjection>, ProjectionError> {
    let log_path = feed_dir.join("signals.jsonl");
    let contents = match std::fs::read_to_string(&log_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };

    let mut out = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            // One corrupt line shouldn't poison the whole projection.
            Err(_) => continue,
        };

        let path = v.get("path").and_then(|x| x.as_str()).unwrap_or_default();
        let rel_path = v
            .get("rel_path")
            .and_then(|x| x.as_str())
            .unwrap_or(path)
            .to_string();
        let event = v
            .get("event")
            .and_then(|x| x.as_str())
            .unwrap_or("unknown")
            .to_string();
        let ts = v.get("ts").and_then(|x| x.as_str()).unwrap_or_default();
        let size_bytes = v.get("size_bytes").and_then(|x| x.as_u64());
        let mtime = v.get("mtime").and_then(|x| x.as_i64());
        let source = v
            .get("source")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();

        let source_ts = DateTime::parse_from_rfc3339(ts)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let name = Path::new(&rel_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&rel_path)
            .to_string();

        let source_id = event_source_id(path, &event, ts, mtime, size_bytes);

        // Read content for live events; a deleted file has none.
        let body_text = if event == "deleted" {
            String::new()
        } else {
            read_text_body(Path::new(path))
        };

        out.push(FilesystemSignalProjection {
            id: projection_id(feed_id, &source_id),
            feed_id: feed_id.to_string(),
            source_id,
            source_ts,
            rel_path,
            name,
            event,
            path: path.to_string(),
            source,
            size_bytes,
            mtime,
            body_text,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_signals(dir: &Path, lines: &[Value]) {
        let body: String = lines
            .iter()
            .map(|l| format!("{l}\n"))
            .collect::<Vec<_>>()
            .join("");
        std::fs::write(dir.join("signals.jsonl"), body).unwrap();
    }

    fn signal(path: &str, rel: &str, event: &str, ts: &str) -> Value {
        serde_json::json!({
            "source": "/Users/me/notes",
            "path": path,
            "rel_path": rel,
            "event": event,
            "ts": ts,
            "size_bytes": if event == "deleted" { Value::Null } else { serde_json::json!(12) },
            "mtime": if event == "deleted" { Value::Null } else { serde_json::json!(1700000000) },
        })
    }

    #[test]
    fn missing_log_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(walk_feed_dir("f", tmp.path()).unwrap().is_empty());
    }

    #[test]
    fn projects_one_row_per_event() {
        let tmp = tempfile::tempdir().unwrap();
        write_signals(
            tmp.path(),
            &[
                signal(
                    "/Users/me/notes/a.txt",
                    "a.txt",
                    "created",
                    "2026-05-25T10:00:00+00:00",
                ),
                signal(
                    "/Users/me/notes/b.txt",
                    "b.txt",
                    "created",
                    "2026-05-25T10:00:01+00:00",
                ),
            ],
        );
        let out = walk_feed_dir("fs-feed", tmp.path()).unwrap();
        assert_eq!(out.len(), 2);
        let a = out.iter().find(|p| p.name == "a.txt").unwrap();
        assert_eq!(a.event, "created");
        assert_eq!(a.rel_path, "a.txt");
        assert_eq!(a.size_bytes, Some(12));
    }

    #[test]
    fn distinct_events_for_same_path_get_distinct_source_ids() {
        let tmp = tempfile::tempdir().unwrap();
        write_signals(
            tmp.path(),
            &[
                signal("/r/a.txt", "a.txt", "created", "2026-05-25T10:00:00+00:00"),
                signal("/r/a.txt", "a.txt", "modified", "2026-05-25T10:05:00+00:00"),
            ],
        );
        let out = walk_feed_dir("f", tmp.path()).unwrap();
        assert_eq!(out.len(), 2);
        assert_ne!(out[0].source_id, out[1].source_id);
    }

    #[test]
    fn same_line_hashes_stably() {
        let tmp = tempfile::tempdir().unwrap();
        write_signals(
            tmp.path(),
            &[signal(
                "/r/a.txt",
                "a.txt",
                "created",
                "2026-05-25T10:00:00+00:00",
            )],
        );
        let first = walk_feed_dir("f", tmp.path()).unwrap();
        let second = walk_feed_dir("f", tmp.path()).unwrap();
        assert_eq!(first[0].source_id, second[0].source_id);
        assert_eq!(first[0].id, second[0].id);
    }

    #[test]
    fn deleted_event_has_null_size_and_mtime() {
        let tmp = tempfile::tempdir().unwrap();
        write_signals(
            tmp.path(),
            &[signal(
                "/r/gone.txt",
                "gone.txt",
                "deleted",
                "2026-05-25T10:00:00+00:00",
            )],
        );
        let out = walk_feed_dir("f", tmp.path()).unwrap();
        assert_eq!(out[0].event, "deleted");
        assert_eq!(out[0].size_bytes, None);
        assert_eq!(out[0].mtime, None);
    }

    #[test]
    fn reads_file_content_into_body_text() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("standup.md");
        std::fs::write(&file, "Project Falcon: schema migration discussed").unwrap();
        write_signals(
            tmp.path(),
            &[serde_json::json!({
                "source": tmp.path().to_string_lossy(),
                "path": file.to_string_lossy(),
                "rel_path": "standup.md",
                "event": "created",
                "ts": "2026-05-25T10:00:00+00:00",
                "size_bytes": 42,
                "mtime": 1748160000,
            })],
        );
        let out = walk_feed_dir("f", tmp.path()).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].body_text.contains("Project Falcon"));
        // The row view folds rel_path + event + content together.
        assert!(out[0].row().body_text.contains("schema migration"));
    }

    #[test]
    fn deleted_event_carries_no_body() {
        let tmp = tempfile::tempdir().unwrap();
        write_signals(
            tmp.path(),
            &[signal(
                "/r/gone.txt",
                "gone.txt",
                "deleted",
                "2026-05-25T10:00:00+00:00",
            )],
        );
        let out = walk_feed_dir("f", tmp.path()).unwrap();
        assert_eq!(out[0].body_text, "");
    }

    #[test]
    fn corrupt_line_is_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("signals.jsonl"),
            "not json\n{\"path\":\"/r/a.txt\",\"rel_path\":\"a.txt\",\"event\":\"created\",\"ts\":\"2026-05-25T10:00:00+00:00\"}\n",
        )
        .unwrap();
        let out = walk_feed_dir("f", tmp.path()).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "a.txt");
    }
}

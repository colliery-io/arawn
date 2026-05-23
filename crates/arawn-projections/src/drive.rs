//! Drive projection — `drive_files`.
//!
//! Mirror layout (from `arawn-feeds::templates::drive::folder_sync`):
//! ```text
//! <feed_dir>/
//!   ├── meta.json             # cursor: { files: { <file_id>: { token, path } } }
//!   └── <subfolder>/<file>    # native bytes (or exported markdown for Docs)
//! ```
//!
//! We read meta.json to enumerate every projected file, then read each
//! file's bytes off disk. Binary or unsupported types still get rows
//! (path + size in metadata) but no body_text and no embedding —
//! body_hash is the file size + path so a re-run is still a no-op.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::warn;

use crate::error::ProjectionError;
use crate::types::{Projection, ProjectionRow};

pub const FEED_TYPE: &str = "drive_files";

/// Heuristic: only embed files whose body looks like text. Drive
/// Docs/Sheets/Slides export to markdown / CSV / markdown respectively
/// — those are all UTF-8 text.
const MAX_BODY_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct DriveFileProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String,
    pub source_ts: DateTime<Utc>,
    pub path: String,
    pub name: String,
    pub mime_type: Option<String>,
    pub size_bytes: u64,
    pub body_text: String,
}

impl Projection for DriveFileProjection {
    fn feed_type(&self) -> &'static str {
        FEED_TYPE
    }

    fn row(&self) -> ProjectionRow {
        let metadata = serde_json::json!({
            "path": self.path,
            "name": self.name,
            "mime_type": self.mime_type,
            "size_bytes": self.size_bytes,
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title: self.name.clone(),
            body_text: self.body_text.clone(),
            feed_type: FEED_TYPE.to_string(),
            metadata,
        }
    }
}

pub fn projection_id(feed_id: &str, file_id: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    feed_id.hash(&mut h);
    "::".hash(&mut h);
    file_id.hash(&mut h);
    format!("dr-{:016x}", h.finish())
}

pub fn walk_feed_dir(
    feed_id: &str,
    feed_dir: &Path,
) -> Result<Vec<DriveFileProjection>, ProjectionError> {
    let meta_path = feed_dir.join("meta.json");
    let bytes = match std::fs::read(&meta_path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let meta: Value = serde_json::from_slice(&bytes)?;
    // `meta.cursor.files` per FeedMeta + Cursor shape; older test
    // fixtures sometimes put it at the top level. Tolerate both.
    let files_obj = meta
        .pointer("/cursor/files")
        .or_else(|| meta.get("files"))
        .and_then(|v| v.as_object());
    let Some(files) = files_obj else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for (file_id, entry) in files {
        let rel_path = match entry.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => continue,
        };
        let abs: PathBuf = feed_dir.join(rel_path);
        let (body_text, size_bytes) = read_text_body(&abs);
        let name = abs
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(rel_path)
            .to_string();
        let mime_type = entry
            .get("mime_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let source_ts = entry
            .get("modified_at")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        out.push(DriveFileProjection {
            id: projection_id(feed_id, file_id),
            feed_id: feed_id.to_string(),
            source_id: file_id.clone(),
            source_ts,
            path: rel_path.to_string(),
            name,
            mime_type,
            size_bytes,
            body_text,
        });
    }
    Ok(out)
}

/// Read a file as utf-8 text, truncated to `MAX_BODY_BYTES`. Returns
/// `("", size)` for missing / binary files so callers can still write
/// a metadata-only projection.
fn read_text_body(path: &Path) -> (String, u64) {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "drive projection: cannot stat file");
            return (String::new(), 0);
        }
    };
    let size = meta.len();
    let cap = (size as usize).min(MAX_BODY_BYTES);
    let bytes = match read_capped(path, cap) {
        Ok(b) => b,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "drive projection: cannot read file");
            return (String::new(), size);
        }
    };
    match String::from_utf8(bytes) {
        Ok(s) => (s, size),
        Err(_) => (String::new(), size),
    }
}

fn read_capped(path: &Path, cap: usize) -> Result<Vec<u8>, std::io::Error> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut buf = Vec::with_capacity(cap.min(8192));
    let mut chunk = [0u8; 8192];
    let mut total = 0;
    while total < cap {
        let want = (cap - total).min(chunk.len());
        let n = f.read(&mut chunk[..want])?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        total += n;
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write_meta(dir: &Path, meta: Value) {
        std::fs::write(dir.join("meta.json"), meta.to_string()).unwrap();
    }

    #[test]
    fn walks_files_from_meta() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("notes.md"), "# Hello\n\nBody text").unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub/data.csv"), "a,b\n1,2\n").unwrap();

        write_meta(
            tmp.path(),
            json!({
                "cursor": {
                    "files": {
                        "file-1": {
                            "path": "notes.md",
                            "token": "t1",
                            "mime_type": "text/markdown",
                        },
                        "file-2": {
                            "path": "sub/data.csv",
                            "token": "t2",
                            "mime_type": "text/csv",
                        }
                    }
                }
            }),
        );

        let out = walk_feed_dir("dr-feed", tmp.path()).unwrap();
        assert_eq!(out.len(), 2);
        let notes = out.iter().find(|p| p.name == "notes.md").unwrap();
        assert!(notes.body_text.contains("Body text"));
        assert_eq!(notes.path, "notes.md");
        let csv = out.iter().find(|p| p.name == "data.csv").unwrap();
        assert_eq!(csv.path, "sub/data.csv");
    }

    #[test]
    fn missing_meta_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let out = walk_feed_dir("f", tmp.path()).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn tolerates_top_level_files_key() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "alpha").unwrap();
        write_meta(
            tmp.path(),
            json!({ "files": { "f1": { "path": "a.txt", "token": "t" } } }),
        );
        let out = walk_feed_dir("f", tmp.path()).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].body_text, "alpha");
    }

    #[test]
    fn missing_local_file_still_produces_metadata_row() {
        let tmp = tempfile::tempdir().unwrap();
        write_meta(
            tmp.path(),
            json!({ "cursor": { "files": {
                "ghost": { "path": "gone.bin", "token": "x" }
            }}}),
        );
        let out = walk_feed_dir("f", tmp.path()).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].body_text, "");
        assert_eq!(out[0].size_bytes, 0);
    }
}

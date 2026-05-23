//! Gmail message projection.
//!
//! Walks a `gmail/inbox-archive` (or `gmail/sender-filter`, `gmail/label-archive`)
//! feed dir on disk: each `<YYYY-MM-DD>/<message_id>.json` file is a Gmail
//! `Message` payload at `format=full`. Each is normalized into a
//! `GmailMessageProjection` row.

use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use tracing::warn;

use crate::error::ProjectionError;
use crate::types::{Projection, ProjectionRow};

pub const FEED_TYPE: &str = "gmail_messages";

#[derive(Debug, Clone)]
pub struct GmailMessageProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String,
    pub source_ts: DateTime<Utc>,
    pub sender: Option<String>,
    pub recipients: Vec<String>,
    pub subject: String,
    pub body_text: String,
    pub thread_id: Option<String>,
    pub labels: Vec<String>,
}

impl Projection for GmailMessageProjection {
    fn feed_type(&self) -> &'static str {
        FEED_TYPE
    }

    fn row(&self) -> ProjectionRow {
        let title = if self.subject.is_empty() {
            format!(
                "(no subject) from {}",
                self.sender.as_deref().unwrap_or("unknown")
            )
        } else {
            self.subject.clone()
        };
        let metadata = serde_json::json!({
            "sender": self.sender,
            "recipients": self.recipients,
            "subject": self.subject,
            "thread_id": self.thread_id,
            "labels": self.labels,
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title,
            body_text: self.body_text.clone(),
            feed_type: FEED_TYPE.to_string(),
            metadata,
        }
    }
}

/// Stable projection id from `(feed_id, message_id)`. Uses a hex hash
/// so the id is short and url-safe.
pub fn projection_id(feed_id: &str, message_id: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    feed_id.hash(&mut h);
    "::".hash(&mut h);
    message_id.hash(&mut h);
    format!("gm-{:016x}", h.finish())
}

/// Parse a single Gmail Message JSON value into a projection.
///
/// Returns `Ok(None)` for malformed messages (missing id /
/// unparseable internalDate) — same skip-warn-continue policy the
/// feed templates use.
pub fn from_gmail_message(
    feed_id: &str,
    msg: &Value,
) -> Result<Option<GmailMessageProjection>, ProjectionError> {
    let source_id = match msg.get("id").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            warn!("gmail projection: missing 'id'");
            return Ok(None);
        }
    };

    let internal_date_ms = match msg
        .get("internalDate")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .or_else(|| msg.get("internalDate").and_then(|v| v.as_i64()))
    {
        Some(v) => v,
        None => {
            warn!(id = %source_id, "gmail projection: missing 'internalDate'");
            return Ok(None);
        }
    };
    let source_ts = match Utc.timestamp_millis_opt(internal_date_ms) {
        chrono::LocalResult::Single(dt) => dt,
        _ => {
            warn!(id = %source_id, "gmail projection: bad 'internalDate'");
            return Ok(None);
        }
    };

    let payload = msg.get("payload");
    let headers = payload
        .and_then(|p| p.get("headers"))
        .and_then(|h| h.as_array());

    let header = |name: &str| -> Option<String> {
        headers?.iter().find_map(|h| {
            let n = h.get("name")?.as_str()?;
            if n.eq_ignore_ascii_case(name) {
                h.get("value")?.as_str().map(|s| s.to_string())
            } else {
                None
            }
        })
    };

    let subject = header("Subject").unwrap_or_default();
    let sender = header("From");
    let to = header("To").unwrap_or_default();
    let cc = header("Cc").unwrap_or_default();
    let recipients: Vec<String> = to
        .split(',')
        .chain(cc.split(','))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let thread_id = msg
        .get("threadId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let labels: Vec<String> = msg
        .get("labelIds")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let body_text = extract_body_text(payload)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            msg.get("snippet")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        });

    Ok(Some(GmailMessageProjection {
        id: projection_id(feed_id, &source_id),
        feed_id: feed_id.to_string(),
        source_id,
        source_ts,
        sender,
        recipients,
        subject,
        body_text,
        thread_id,
        labels,
    }))
}

/// Walk the on-disk feed dir, parsing every `<YYYY-MM-DD>/<id>.json`
/// into a `GmailMessageProjection`. Malformed messages are skipped
/// with a warn. The output is unordered.
pub fn walk_feed_dir(
    feed_id: &str,
    feed_dir: &Path,
) -> Result<Vec<GmailMessageProjection>, ProjectionError> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(feed_dir) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };
    for day_entry in entries {
        let day_entry = day_entry?;
        let day_path = day_entry.path();
        if !day_entry.file_type()?.is_dir() {
            continue;
        }
        // Skip non-date dirs (e.g. nested helpers); date dirs are 10 chars wide.
        if day_entry
            .file_name()
            .to_str()
            .is_none_or(|s| s.len() != 10 || s.chars().nth(4).is_none_or(|c| c != '-'))
        {
            continue;
        }
        for msg_entry in std::fs::read_dir(&day_path)? {
            let msg_entry = msg_entry?;
            let msg_path: PathBuf = msg_entry.path();
            if msg_path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let bytes = std::fs::read(&msg_path)?;
            let msg: Value = match serde_json::from_slice(&bytes) {
                Ok(v) => v,
                Err(e) => {
                    warn!(path = %msg_path.display(), error = %e, "skip unparseable gmail JSON");
                    continue;
                }
            };
            if let Some(p) = from_gmail_message(feed_id, &msg)? {
                out.push(p);
            }
        }
    }
    Ok(out)
}

/// Decode a gmail body part. Walks `parts` recursively, preferring
/// `text/plain`, falling back to `text/html` (raw, no markdown
/// conversion in this layer — that's a feed-side concern).
fn extract_body_text(payload: Option<&Value>) -> Option<String> {
    let p = payload?;
    if let Some(t) = extract_part(p, "text/plain") {
        return Some(t);
    }
    if let Some(t) = extract_part(p, "text/html") {
        return Some(t);
    }
    None
}

fn extract_part(part: &Value, mime: &str) -> Option<String> {
    let mt = part.get("mimeType").and_then(|v| v.as_str()).unwrap_or("");
    if mt == mime {
        let data = part
            .get("body")
            .and_then(|b| b.get("data"))
            .and_then(|d| d.as_str())?;
        return decode_base64url(data).ok();
    }
    if let Some(parts) = part.get("parts").and_then(|v| v.as_array()) {
        for sub in parts {
            if let Some(t) = extract_part(sub, mime) {
                return Some(t);
            }
        }
    }
    None
}

fn decode_base64url(s: &str) -> Result<String, ProjectionError> {
    // base64url: '-' → '+', '_' → '/'. Pad to multiple of 4.
    let mut padded: String = s.replace('-', "+").replace('_', "/");
    while !padded.len().is_multiple_of(4) {
        padded.push('=');
    }
    let bytes = base64_decode(&padded)
        .map_err(|e| ProjectionError::Schema(format!("base64 decode: {e}")))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Minimal base64 decoder (we don't have base64 as a workspace dep
/// here yet). Standard alphabet, accepts padding.
fn base64_decode(s: &str) -> Result<Vec<u8>, &'static str> {
    fn val(c: u8) -> Result<u8, &'static str> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err("invalid base64 char"),
        }
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let a = bytes[i];
        let b = bytes[i + 1];
        let c = bytes[i + 2];
        let d = bytes[i + 3];
        let va = val(a)?;
        let vb = val(b)?;
        let n0 = (va << 2) | (vb >> 4);
        out.push(n0);
        if c != b'=' {
            let vc = val(c)?;
            let n1 = (vb << 4) | (vc >> 2);
            out.push(n1);
            if d != b'=' {
                let vd = val(d)?;
                let n2 = (vc << 6) | vd;
                out.push(n2);
            }
        }
        i += 4;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_minimal_message() {
        let msg = json!({
            "id": "abc123",
            "threadId": "t1",
            "internalDate": "1700000000000",
            "labelIds": ["INBOX", "IMPORTANT"],
            "snippet": "hello there",
            "payload": {
                "headers": [
                    {"name": "From", "value": "alice@example.com"},
                    {"name": "To", "value": "bob@example.com, carol@example.com"},
                    {"name": "Subject", "value": "Greetings"},
                ],
                "mimeType": "text/plain",
                "body": { "data": "SGVsbG8sIHdvcmxkIQ" }  // "Hello, world!"
            }
        });
        let p = from_gmail_message("feed-1", &msg).unwrap().unwrap();
        assert_eq!(p.source_id, "abc123");
        assert_eq!(p.feed_id, "feed-1");
        assert_eq!(p.thread_id.as_deref(), Some("t1"));
        assert_eq!(p.subject, "Greetings");
        assert_eq!(p.sender.as_deref(), Some("alice@example.com"));
        assert_eq!(p.recipients.len(), 2);
        assert_eq!(p.body_text, "Hello, world!");
        assert_eq!(p.labels, vec!["INBOX".to_string(), "IMPORTANT".to_string()]);
    }

    #[test]
    fn skips_missing_id() {
        let msg = json!({ "internalDate": "1" });
        assert!(from_gmail_message("feed-1", &msg).unwrap().is_none());
    }

    #[test]
    fn skips_bad_internaldate() {
        let msg = json!({ "id": "x" });
        assert!(from_gmail_message("feed-1", &msg).unwrap().is_none());
    }

    #[test]
    fn projection_id_is_stable() {
        let a = projection_id("feed", "msg");
        let b = projection_id("feed", "msg");
        assert_eq!(a, b);
        let c = projection_id("feed2", "msg");
        assert_ne!(a, c);
    }

    #[test]
    fn snippet_fallback_when_no_body() {
        let msg = json!({
            "id": "x",
            "internalDate": "1",
            "snippet": "fallback text",
            "payload": { "headers": [] }
        });
        let p = from_gmail_message("f", &msg).unwrap().unwrap();
        assert_eq!(p.body_text, "fallback text");
    }

    #[test]
    fn walk_feed_dir_picks_up_files() {
        let tmp = tempfile::tempdir().unwrap();
        let day_dir = tmp.path().join("2026-05-11");
        std::fs::create_dir_all(&day_dir).unwrap();
        let msg = json!({
            "id": "m1",
            "internalDate": "1700000000000",
            "snippet": "body",
            "payload": { "headers": [{"name":"Subject","value":"Hi"}] }
        });
        std::fs::write(day_dir.join("m1.json"), msg.to_string()).unwrap();
        // Throw in a non-json file + non-date dir to make sure they're ignored.
        std::fs::write(tmp.path().join("ignored.txt"), "x").unwrap();
        std::fs::create_dir(tmp.path().join("not-a-date")).unwrap();

        let out = walk_feed_dir("feed-1", tmp.path()).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].source_id, "m1");
    }
}

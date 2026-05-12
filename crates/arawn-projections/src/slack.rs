//! Slack projections — top-level messages and thread replies.
//!
//! Mirror layout (from `arawn-feeds::templates::slack`):
//! ```text
//! <feed_dir>/
//!   ├── <YYYY-MM-DD>.jsonl     # one top-level message per line
//!   └── threads/
//!       └── <parent_ts>.jsonl  # parent + replies, one per line
//! ```

use std::io::BufRead;
use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use tracing::warn;

use crate::error::ProjectionError;
use crate::types::{Projection, ProjectionRow};

pub const TOPLEVEL_FEED_TYPE: &str = "slack_messages";
pub const THREAD_FEED_TYPE: &str = "slack_thread_messages";

#[derive(Debug, Clone)]
pub struct SlackMessageProjection {
    pub id: String,
    pub feed_id: String,
    pub source_id: String,
    pub source_ts: DateTime<Utc>,
    pub channel_id: Option<String>,
    pub sender_id: Option<String>,
    pub text: String,
    pub thread_ts: Option<String>,
    pub reactions: Vec<Value>,
    pub is_thread_reply: bool,
}

impl Projection for SlackMessageProjection {
    fn feed_type(&self) -> &'static str {
        if self.is_thread_reply {
            THREAD_FEED_TYPE
        } else {
            TOPLEVEL_FEED_TYPE
        }
    }

    fn row(&self) -> ProjectionRow {
        let title = synth_title(self);
        let metadata = serde_json::json!({
            "channel_id": self.channel_id,
            "sender_id": self.sender_id,
            "thread_ts": self.thread_ts,
            "reactions": self.reactions,
        });
        ProjectionRow {
            id: self.id.clone(),
            feed_id: self.feed_id.clone(),
            source_id: self.source_id.clone(),
            source_ts: self.source_ts,
            title,
            body_text: self.text.clone(),
            feed_type: self.feed_type().to_string(),
            metadata,
        }
    }
}

fn synth_title(p: &SlackMessageProjection) -> String {
    let mut snippet: String = p.text.chars().take(80).collect();
    if p.text.len() > 80 {
        snippet.push('…');
    }
    let who = p.sender_id.as_deref().unwrap_or("?");
    if p.is_thread_reply {
        format!("reply from {who}: {snippet}")
    } else {
        format!("from {who}: {snippet}")
    }
}

pub fn projection_id(feed_id: &str, slack_ts: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    feed_id.hash(&mut h);
    "::".hash(&mut h);
    slack_ts.hash(&mut h);
    format!("sl-{:016x}", h.finish())
}

/// Slack `ts` is `"<unix_secs>.<microseconds>"`. Returns `None` for
/// malformed input.
pub fn parse_slack_ts(ts: &str) -> Option<DateTime<Utc>> {
    let (secs_s, micros_s) = ts.split_once('.')?;
    let secs: i64 = secs_s.parse().ok()?;
    let micros: i64 = micros_s.parse().ok()?;
    Utc.timestamp_opt(secs, (micros * 1000) as u32).single()
}

pub fn from_slack_message(
    feed_id: &str,
    msg: &Value,
    is_thread_reply: bool,
) -> Option<SlackMessageProjection> {
    let ts = msg.get("ts").and_then(|v| v.as_str())?;
    let source_ts = match parse_slack_ts(ts) {
        Some(t) => t,
        None => {
            warn!(ts = %ts, "slack projection: unparseable ts");
            return None;
        }
    };
    let channel_id = msg
        .get("channel")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let sender_id = msg
        .get("user")
        .and_then(|v| v.as_str())
        .or_else(|| msg.get("bot_id").and_then(|v| v.as_str()))
        .map(|s| s.to_string());
    let text = msg
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let thread_ts = msg
        .get("thread_ts")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let reactions = msg
        .get("reactions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Some(SlackMessageProjection {
        id: projection_id(feed_id, ts),
        feed_id: feed_id.to_string(),
        source_id: ts.to_string(),
        source_ts,
        channel_id,
        sender_id,
        text,
        thread_ts,
        reactions,
        is_thread_reply,
    })
}

fn parse_jsonl(path: &Path) -> Result<Vec<Value>, ProjectionError> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str(&line) {
            Ok(v) => out.push(v),
            Err(e) => warn!(path = %path.display(), error = %e, "skip unparseable slack jsonl line"),
        }
    }
    Ok(out)
}

pub fn walk_feed_dir(
    feed_id: &str,
    feed_dir: &Path,
) -> Result<Vec<SlackMessageProjection>, ProjectionError> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(feed_dir) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };
    for entry in entries {
        let entry = entry?;
        let path: PathBuf = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "meta.json" {
            continue;
        }
        if entry.file_type()?.is_dir() {
            if name == "threads" {
                for sub in std::fs::read_dir(&path)? {
                    let sub = sub?;
                    let sub_path = sub.path();
                    if sub_path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                        continue;
                    }
                    let parent_ts = sub_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or_default();
                    let msgs = parse_jsonl(&sub_path)?;
                    for (i, msg) in msgs.iter().enumerate() {
                        // The first line of a thread file is the parent
                        // (already captured by top-level day file).
                        // Skip it here to avoid double-counting.
                        let ts = msg.get("ts").and_then(|v| v.as_str()).unwrap_or("");
                        let is_parent = ts == parent_ts;
                        if i == 0 && is_parent {
                            continue;
                        }
                        if let Some(p) = from_slack_message(feed_id, msg, true) {
                            out.push(p);
                        }
                    }
                }
            }
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }
        let msgs = parse_jsonl(&path)?;
        for msg in &msgs {
            if let Some(p) = from_slack_message(feed_id, msg, false) {
                out.push(p);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_ts() {
        let dt = parse_slack_ts("1715000000.001234").unwrap();
        assert_eq!(dt.timestamp(), 1715000000);
    }

    #[test]
    fn from_message_basic() {
        let msg = json!({
            "ts": "1715000000.000001",
            "user": "U1",
            "text": "hello",
            "channel": "C1",
            "reactions": [{"name": "thumbsup"}]
        });
        let p = from_slack_message("feed-1", &msg, false).unwrap();
        assert_eq!(p.source_id, "1715000000.000001");
        assert_eq!(p.sender_id.as_deref(), Some("U1"));
        assert_eq!(p.text, "hello");
        assert_eq!(p.reactions.len(), 1);
        assert!(!p.is_thread_reply);
        assert_eq!(p.feed_type(), TOPLEVEL_FEED_TYPE);
    }

    #[test]
    fn thread_reply_routes_to_thread_table() {
        let msg = json!({
            "ts": "1715000010.000001",
            "user": "U2",
            "text": "reply",
            "thread_ts": "1715000000.000001",
        });
        let p = from_slack_message("f", &msg, true).unwrap();
        assert!(p.is_thread_reply);
        assert_eq!(p.feed_type(), THREAD_FEED_TYPE);
    }

    #[test]
    fn walks_top_level_and_threads() {
        let tmp = tempfile::tempdir().unwrap();
        // Day file
        std::fs::write(
            tmp.path().join("2026-05-11.jsonl"),
            "{\"ts\":\"1715000000.000001\",\"user\":\"U1\",\"text\":\"parent\"}\n\
             {\"ts\":\"1715000005.000001\",\"user\":\"U2\",\"text\":\"another top\"}\n",
        )
        .unwrap();
        // Threads dir
        let threads = tmp.path().join("threads");
        std::fs::create_dir(&threads).unwrap();
        std::fs::write(
            threads.join("1715000000.000001.jsonl"),
            "{\"ts\":\"1715000000.000001\",\"user\":\"U1\",\"text\":\"parent\"}\n\
             {\"ts\":\"1715000010.000001\",\"user\":\"U3\",\"text\":\"reply\",\"thread_ts\":\"1715000000.000001\"}\n",
        )
        .unwrap();

        let out = walk_feed_dir("f", tmp.path()).unwrap();
        // 2 top-level + 1 reply (parent in thread file skipped).
        assert_eq!(out.len(), 3);
        let top: Vec<_> = out.iter().filter(|p| !p.is_thread_reply).collect();
        let replies: Vec<_> = out.iter().filter(|p| p.is_thread_reply).collect();
        assert_eq!(top.len(), 2);
        assert_eq!(replies.len(), 1);
    }
}

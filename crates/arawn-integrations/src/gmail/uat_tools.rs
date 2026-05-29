//! ARAWN-I-0062 T-C: UAT-mode Gmail tools backed by the projection store.
//!
//! `gmail_inbox_read` and `gmail_search` mirror the production tool names,
//! descriptions, and schemas. Output shape matches the production
//! `MessageSummary` (private in `tools.rs`) so the agent sees no difference.

use std::path::PathBuf;

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};

use arawn_projections::ProjectionStore;

/// Wire-shape mirror of the production `MessageSummary` (private in
/// `crates/arawn-integrations/src/gmail/tools.rs`).
#[derive(Debug, Clone, Serialize)]
struct MessageSummary {
    id: String,
    thread_id: Option<String>,
    from: Option<String>,
    subject: Option<String>,
    date: Option<String>,
    snippet: Option<String>,
    body_truncated: bool,
}

fn row_to_summary(
    id: String,
    title: String,
    body: String,
    ts: String,
    metadata: Value,
) -> MessageSummary {
    let sender = metadata
        .get("sender")
        .and_then(|v| v.as_str())
        .map(String::from);
    let thread_id = metadata
        .get("thread_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    let snippet = if body.is_empty() {
        None
    } else {
        let trimmed: String = body.chars().take(200).collect();
        Some(trimmed)
    };
    MessageSummary {
        id,
        thread_id,
        from: sender,
        subject: Some(title),
        date: Some(ts),
        snippet,
        body_truncated: true,
    }
}

fn open_store(data_dir: &PathBuf) -> Result<ProjectionStore, ToolError> {
    let store = ProjectionStore::open(&data_dir.join("projections.db"))
        .map_err(|e| ToolError::ExecutionFailed(format!("open projections: {e}")))?;
    store
        .ensure_feed_type(arawn_projections::gmail::FEED_TYPE)
        .map_err(|e| ToolError::ExecutionFailed(format!("ensure schema: {e}")))?;
    Ok(store)
}

/// `gmail_inbox_read` UAT impl — returns the most recent `gmail_messages`
/// rows ordered by `source_ts` desc.
pub struct UatGmailInboxReadTool {
    data_dir: PathBuf,
}

impl UatGmailInboxReadTool {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn list_inbox(&self, limit: usize) -> Result<Vec<MessageSummary>, ToolError> {
        let store = open_store(&self.data_dir)?;
        let conn = store.conn().lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, body_text, source_ts, metadata \
                 FROM gmail_messages \
                 ORDER BY source_ts DESC LIMIT ?1",
            )
            .map_err(|e| ToolError::ExecutionFailed(format!("prepare: {e}")))?;
        let rows = stmt
            .query_map([limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| ToolError::ExecutionFailed(format!("query: {e}")))?;
        let mut out = Vec::new();
        for r in rows {
            let (id, title, body, ts, meta) =
                r.map_err(|e| ToolError::ExecutionFailed(format!("row: {e}")))?;
            let meta_val: Value = serde_json::from_str(&meta).unwrap_or(Value::Null);
            out.push(row_to_summary(id, title, body, ts, meta_val));
        }
        Ok(out)
    }
}

#[async_trait]
impl Tool for UatGmailInboxReadTool {
    fn name(&self) -> &str {
        "gmail_inbox_read"
    }
    fn description(&self) -> &str {
        // Match the production description so the agent's tool-selection
        // signal is identical between modes.
        "Read recent messages from the connected Gmail inbox. Returns a list of messages with \
         sender, subject, snippet, and date. Body is always truncated — call gmail_get_message \
         with the message id when you need the full text."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Gmail
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Max messages to return (default 10, max 50)",
                    "minimum": 1,
                    "maximum": 50
                },
                "label": {
                    "type": "string",
                    "description": "Gmail label id to filter by (e.g. 'INBOX', 'UNREAD'). Default: INBOX. Ignored in UAT."
                }
            }
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(50) as usize;
        let out = self.list_inbox(limit)?;
        Ok(ToolOutput::success(serde_json::to_string(&out).unwrap()))
    }
}

/// `gmail_search` UAT impl — uses the projection store's FTS5 index.
pub struct UatGmailSearchTool {
    data_dir: PathBuf,
}

impl UatGmailSearchTool {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<MessageSummary>, ToolError> {
        let store = open_store(&self.data_dir)?;
        let plain = strip_gmail_operators(query);
        let ids = store
            .fts_search(arawn_projections::gmail::FEED_TYPE, &plain, limit)
            .map_err(|e| ToolError::ExecutionFailed(format!("fts: {e}")))?;
        let mut out = Vec::new();
        for id in ids {
            if let Some(row) = store
                .get_row(arawn_projections::gmail::FEED_TYPE, &id)
                .map_err(|e| ToolError::ExecutionFailed(format!("get_row: {e}")))?
            {
                let ts = row.source_ts.to_rfc3339();
                out.push(row_to_summary(
                    row.id,
                    row.title,
                    row.body_text,
                    ts,
                    row.metadata,
                ));
            }
        }
        Ok(out)
    }
}

#[async_trait]
impl Tool for UatGmailSearchTool {
    fn name(&self) -> &str {
        "gmail_search"
    }
    fn description(&self) -> &str {
        "Search messages using Gmail search syntax (e.g. 'from:alice', 'has:attachment newer_than:7d'). \
         Returns the same shape as gmail_inbox_read with body_truncated=true."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Gmail
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Gmail search query — same syntax as the search bar in Gmail."
                },
                "limit": {
                    "type": "integer",
                    "description": "Max messages to return (default 10, max 50)",
                    "minimum": 1,
                    "maximum": 50
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'query'".into()))?;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(50) as usize;
        let out = self.search(query, limit)?;
        Ok(ToolOutput::success(serde_json::to_string(&out).unwrap()))
    }
}

/// Convert a Gmail-search-syntax-style query into a plain whitespace bag
/// suitable for FTS5. Drops `<op>:<value>` tokens like `from:alice` (keeps
/// `alice` though, so the search remains broad) and removes unmatched
/// punctuation. UAT-only — the production tool routes the full syntax to
/// Google's API.
fn strip_gmail_operators(query: &str) -> String {
    let mut bag: Vec<String> = Vec::new();
    for tok in query.split_whitespace() {
        if let Some((_op, rest)) = tok.split_once(':') {
            if !rest.is_empty() {
                bag.push(rest.to_string());
            }
        } else {
            bag.push(tok.to_string());
        }
    }
    bag.join(" ")
}

/// Convenience constructor returning the gmail UAT tool set as `Box<dyn Tool>`s.
pub fn uat_gmail_tools(data_dir: PathBuf) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(UatGmailInboxReadTool::new(data_dir.clone())) as Box<dyn Tool>,
        Box::new(UatGmailSearchTool::new(data_dir)) as Box<dyn Tool>,
    ]
}

// (Time imports kept for parity with future tools that need them.)
#[allow(dead_code)]
const _DT_PARITY: Option<DateTime<Utc>> = None;

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_projections::gmail::{FEED_TYPE, GmailMessageProjection};

    fn fixture_dir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let store = ProjectionStore::open(&tmp.path().join("projections.db")).unwrap();
        store.ensure_feed_type(FEED_TYPE).unwrap();
        let now = Utc::now();
        for (i, (subject, body, sender)) in [
            ("RFC-0042 sign-off", "Alice asks for alignment on Postgres", "alice@example.com"),
            ("Catch up next week", "Bob says Tue/Wed mornings or Thu after 2", "bob@example.com"),
            ("SAVE10 — meal kit", "Unsubscribe", "promo@meal.example"),
        ]
        .iter()
        .enumerate()
        {
            let proj = GmailMessageProjection {
                id: format!("gm-{i}"),
                feed_id: "f".into(),
                source_id: format!("msg-{i}"),
                source_ts: now - chrono::Duration::hours(i as i64),
                sender: Some((*sender).into()),
                recipients: vec!["me@example.com".into()],
                subject: (*subject).into(),
                body_text: (*body).into(),
                thread_id: Some(format!("thread-{i}")),
                labels: vec!["INBOX".into()],
            };
            store.write(&proj).unwrap();
        }
        tmp
    }

    #[test]
    fn inbox_list_orders_newest_first() {
        let tmp = fixture_dir();
        let tool = UatGmailInboxReadTool::new(tmp.path().to_path_buf());
        let payload = tool.list_inbox(10).unwrap();
        assert_eq!(payload.len(), 3);
        assert_eq!(payload[0].subject.as_deref(), Some("RFC-0042 sign-off"));
    }

    #[test]
    fn search_finds_by_subject_substring() {
        let tmp = fixture_dir();
        let tool = UatGmailSearchTool::new(tmp.path().to_path_buf());
        let payload = tool.search("from:alice RFC-0042", 10).unwrap();
        assert!(
            payload
                .iter()
                .any(|m| m.subject.as_deref() == Some("RFC-0042 sign-off")),
            "expected RFC-0042 result, got {payload:?}"
        );
    }

    #[test]
    fn strip_drops_operator_keeps_value() {
        let q = strip_gmail_operators("from:alice@example.com newer_than:7d Postgres");
        assert!(q.contains("alice@example.com"));
        assert!(q.contains("Postgres"));
        // The leading operator words are gone.
        assert!(!q.contains("from:"));
        assert!(!q.contains("newer_than:"));
    }
}

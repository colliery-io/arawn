//! Engine tools that wrap the Gmail integration.
//!
//! Each tool holds an `Arc<GmailIntegration>` and constructs a fresh
//! `GmailHub` per call (cheap — just an Arc + a hyper-util client builder).
//! Per-call construction means a refreshed token persisted by one call is
//! picked up by the next call automatically.

use std::sync::Arc;

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use google_gmail1::api::{Message, ModifyMessageRequest};
use serde::Serialize;
use serde_json::{Value, json};

use super::integration::GmailIntegration;

/// One-line summary of a Gmail message — what `inbox_read` and `search` return per row.
/// `body_truncated` is always `true` for these tools; the agent calls
/// `gmail_get_message` (separate tool) for the full plain-text body.
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

fn integ_err(e: crate::IntegrationError) -> ToolError {
    ToolError::ExecutionFailed(e.user_message())
}

fn google_err(stage: &str, e: google_gmail1::Error) -> ToolError {
    ToolError::ExecutionFailed(format!("Gmail {stage}: {e}"))
}

/// Pull metadata + snippet for a list of message ids. Used by both
/// `inbox_read` and `search` since the response shape is identical.
async fn fetch_summaries(
    hub: &super::client::GmailHub,
    ids: &[String],
) -> Result<Vec<MessageSummary>, ToolError> {
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let (_resp, message) = hub
            .users()
            .messages_get("me", id)
            .format("metadata")
            .add_metadata_headers("From")
            .add_metadata_headers("Subject")
            .add_metadata_headers("Date")
            .doit()
            .await
            .map_err(|e| google_err("messages.get", e))?;
        out.push(summary_from_message(&message));
    }
    Ok(out)
}

fn summary_from_message(m: &Message) -> MessageSummary {
    let mut from = None;
    let mut subject = None;
    let mut date = None;
    if let Some(ref payload) = m.payload
        && let Some(ref headers) = payload.headers
    {
        for h in headers {
            match (h.name.as_deref(), h.value.clone()) {
                (Some("From"), Some(v)) => from = Some(v),
                (Some("Subject"), Some(v)) => subject = Some(v),
                (Some("Date"), Some(v)) => date = Some(v),
                _ => {}
            }
        }
    }
    MessageSummary {
        id: m.id.clone().unwrap_or_default(),
        thread_id: m.thread_id.clone(),
        from,
        subject,
        date,
        snippet: m.snippet.clone(),
        body_truncated: true,
    }
}

// ─── /gmail_inbox_read ────────────────────────────────────────────────────

pub struct GmailInboxReadTool {
    integration: Arc<GmailIntegration>,
}

impl GmailInboxReadTool {
    pub fn new(integration: Arc<GmailIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for GmailInboxReadTool {
    fn name(&self) -> &str {
        "gmail_inbox_read"
    }
    fn description(&self) -> &str {
        "Read recent messages from the connected Gmail inbox. Returns a list of messages with \
         sender, subject, snippet, and date. Body is always truncated — call gmail_get_message \
         with the message id when you need the full text."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
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
                    "description": "Gmail label id to filter by (e.g. 'INBOX', 'UNREAD'). Default: INBOX."
                }
            }
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10).min(50) as u32;
        let label = params
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("INBOX")
            .to_string();

        let hub = self.integration.hub().map_err(integ_err)?;
        let (_resp, list) = hub
            .users()
            .messages_list("me")
            .max_results(limit)
            .add_label_ids(&label)
            .doit()
            .await
            .map_err(|e| google_err("messages.list", e))?;
        let ids: Vec<String> = list
            .messages
            .unwrap_or_default()
            .into_iter()
            .filter_map(|m| m.id)
            .collect();
        let summaries = fetch_summaries(&hub, &ids).await?;
        Ok(ToolOutput::success(serde_json::to_string(&summaries).unwrap()))
    }
}

// ─── /gmail_search ────────────────────────────────────────────────────────

pub struct GmailSearchTool {
    integration: Arc<GmailIntegration>,
}

impl GmailSearchTool {
    pub fn new(integration: Arc<GmailIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for GmailSearchTool {
    fn name(&self) -> &str {
        "gmail_search"
    }
    fn description(&self) -> &str {
        "Search messages using Gmail search syntax (e.g. 'from:alice', 'has:attachment newer_than:7d'). \
         Returns the same shape as gmail_inbox_read with body_truncated=true."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
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
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'query' parameter".into()))?
            .to_string();
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10).min(50) as u32;

        let hub = self.integration.hub().map_err(integ_err)?;
        let (_resp, list) = hub
            .users()
            .messages_list("me")
            .q(&query)
            .max_results(limit)
            .doit()
            .await
            .map_err(|e| google_err("messages.list", e))?;
        let ids: Vec<String> = list
            .messages
            .unwrap_or_default()
            .into_iter()
            .filter_map(|m| m.id)
            .collect();
        let summaries = fetch_summaries(&hub, &ids).await?;
        Ok(ToolOutput::success(serde_json::to_string(&summaries).unwrap()))
    }
}

// ─── /gmail_get_message — full body fetch with multipart decode ───────────

pub struct GmailGetMessageTool {
    integration: Arc<GmailIntegration>,
}

impl GmailGetMessageTool {
    pub fn new(integration: Arc<GmailIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for GmailGetMessageTool {
    fn name(&self) -> &str {
        "gmail_get_message"
    }
    fn description(&self) -> &str {
        "Fetch the full plain-text body of a Gmail message by id. Use after gmail_inbox_read or \
         gmail_search returns a snippet you want to expand. Returns headers + decoded body."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message_id": { "type": "string", "description": "Gmail message id to fetch." }
            },
            "required": ["message_id"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let id = params
            .get("message_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'message_id' parameter".into()))?
            .to_string();

        let hub = self.integration.hub().map_err(integ_err)?;
        let (_resp, message) = hub
            .users()
            .messages_get("me", &id)
            .format("full")
            .doit()
            .await
            .map_err(|e| google_err("messages.get", e))?;

        let summary = summary_from_message(&message);
        let body = extract_plain_text_body(&message).unwrap_or_default();

        let payload = json!({
            "id": summary.id,
            "thread_id": summary.thread_id,
            "from": summary.from,
            "subject": summary.subject,
            "date": summary.date,
            "snippet": summary.snippet,
            "body": body,
            "body_truncated": false,
        });
        Ok(ToolOutput::success(payload.to_string()))
    }
}

/// Walk a `Message`'s payload tree looking for the first `text/plain` part.
/// Returns the decoded UTF-8 body. Pure function — public for testing.
pub(super) fn extract_plain_text_body(m: &Message) -> Option<String> {
    let payload = m.payload.as_ref()?;
    walk_for_plain_text(payload)
}

fn walk_for_plain_text(part: &google_gmail1::api::MessagePart) -> Option<String> {
    if part.mime_type.as_deref() == Some("text/plain")
        && let Some(ref body) = part.body
        && let Some(ref data) = body.data
    {
        return String::from_utf8(data.clone()).ok();
    }
    if let Some(ref children) = part.parts {
        for child in children {
            if let Some(text) = walk_for_plain_text(child) {
                return Some(text);
            }
        }
    }
    None
}

// ─── /gmail_send ──────────────────────────────────────────────────────────

pub struct GmailSendTool {
    integration: Arc<GmailIntegration>,
}

impl GmailSendTool {
    pub fn new(integration: Arc<GmailIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for GmailSendTool {
    fn name(&self) -> &str {
        "gmail_send"
    }
    fn description(&self) -> &str {
        "Send an email via the connected Gmail account. v1 sends plain text only. \
         Returns the new message id on success."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        // Mode-default in `default` mode is Ask, which is the right gate for
        // "agent wants to send mail on your behalf."
        PermissionCategory::Other
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "to": { "type": "string", "description": "Recipient email address" },
                "subject": { "type": "string", "description": "Subject line" },
                "body": { "type": "string", "description": "Plain text body" },
                "in_reply_to": {
                    "type": "string",
                    "description": "Optional Message-ID header value to thread the reply (e.g. <abc@example.com>)"
                }
            },
            "required": ["to", "subject", "body"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let to = params
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'to'".into()))?;
        let subject = params
            .get("subject")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'subject'".into()))?;
        let body = params
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'body'".into()))?;
        let in_reply_to = params.get("in_reply_to").and_then(|v| v.as_str());

        // google-gmail1 only exposes `upload()` on UserMessageSendCall, not a
        // plain `doit()` — the Gmail API treats outgoing messages as media
        // uploads. We pass the raw RFC2822 bytes as a Cursor (any
        // `Read + Seek` works) with mime_type `message/rfc822`.
        let raw = build_rfc2822(to, subject, body, in_reply_to);
        let stream = std::io::Cursor::new(raw.into_bytes());
        let mime_type: mime::Mime = "message/rfc822".parse().expect("static mime type parses");

        let hub = self.integration.hub().map_err(integ_err)?;
        let (_resp, sent) = hub
            .users()
            .messages_send(Message::default(), "me")
            .upload(stream, mime_type)
            .await
            .map_err(|e| google_err("messages.send", e))?;

        let payload = json!({
            "id": sent.id.unwrap_or_default(),
            "thread_id": sent.thread_id,
        });
        Ok(ToolOutput::success(payload.to_string()))
    }
}

/// Tiny RFC 2822 builder. v1 = plain text only; HTML is a follow-up.
pub(super) fn build_rfc2822(
    to: &str,
    subject: &str,
    body: &str,
    in_reply_to: Option<&str>,
) -> String {
    let mut msg = String::new();
    msg.push_str(&format!("To: {to}\r\n"));
    msg.push_str(&format!("Subject: {subject}\r\n"));
    msg.push_str("Content-Type: text/plain; charset=\"UTF-8\"\r\n");
    msg.push_str("MIME-Version: 1.0\r\n");
    if let Some(reply_id) = in_reply_to {
        msg.push_str(&format!("In-Reply-To: {reply_id}\r\n"));
        msg.push_str(&format!("References: {reply_id}\r\n"));
    }
    msg.push_str("\r\n");
    msg.push_str(body);
    msg
}

// ─── /gmail_mark_read ─────────────────────────────────────────────────────

pub struct GmailMarkReadTool {
    integration: Arc<GmailIntegration>,
}

impl GmailMarkReadTool {
    pub fn new(integration: Arc<GmailIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for GmailMarkReadTool {
    fn name(&self) -> &str {
        "gmail_mark_read"
    }
    fn description(&self) -> &str {
        "Strip the UNREAD label from a Gmail message, marking it as read."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        // Modifies state but reversible; FileWrite is the closest match
        // (mode default in `accept_edits` allows it).
        PermissionCategory::FileWrite
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message_id": { "type": "string", "description": "Gmail message id" }
            },
            "required": ["message_id"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let id = params
            .get("message_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'message_id'".into()))?;

        let req = ModifyMessageRequest {
            remove_label_ids: Some(vec!["UNREAD".to_string()]),
            ..Default::default()
        };
        let hub = self.integration.hub().map_err(integ_err)?;
        hub.users()
            .messages_modify(req, "me", id)
            .doit()
            .await
            .map_err(|e| google_err("messages.modify", e))?;
        Ok(ToolOutput::success(json!({"id": id, "status": "marked_read"}).to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use google_gmail1::api::{MessagePart, MessagePartBody, MessagePartHeader};

    fn header(name: &str, value: &str) -> MessagePartHeader {
        MessagePartHeader {
            name: Some(name.to_string()),
            value: Some(value.to_string()),
        }
    }

    #[test]
    fn summary_from_message_extracts_known_headers() {
        let m = Message {
            id: Some("m1".into()),
            thread_id: Some("t1".into()),
            snippet: Some("just checking in...".into()),
            payload: Some(MessagePart {
                headers: Some(vec![
                    header("From", "alice@example.com"),
                    header("Subject", "lunch?"),
                    header("Date", "Mon, 03 May 2026 10:00:00 -0400"),
                    header("Other", "ignore me"),
                ]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let summary = summary_from_message(&m);
        assert_eq!(summary.id, "m1");
        assert_eq!(summary.thread_id.as_deref(), Some("t1"));
        assert_eq!(summary.from.as_deref(), Some("alice@example.com"));
        assert_eq!(summary.subject.as_deref(), Some("lunch?"));
        assert_eq!(summary.snippet.as_deref(), Some("just checking in..."));
        assert!(summary.body_truncated);
    }

    #[test]
    fn summary_handles_empty_payload() {
        let m = Message {
            id: Some("bare".into()),
            ..Default::default()
        };
        let summary = summary_from_message(&m);
        assert_eq!(summary.id, "bare");
        assert!(summary.from.is_none());
        assert!(summary.subject.is_none());
    }

    #[test]
    fn extract_plain_text_finds_top_level_text_plain() {
        let m = Message {
            payload: Some(MessagePart {
                mime_type: Some("text/plain".into()),
                body: Some(MessagePartBody {
                    data: Some(b"hello world".to_vec()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(extract_plain_text_body(&m).as_deref(), Some("hello world"));
    }

    #[test]
    fn extract_plain_text_descends_into_multipart_alternative() {
        let m = Message {
            payload: Some(MessagePart {
                mime_type: Some("multipart/alternative".into()),
                parts: Some(vec![
                    MessagePart {
                        mime_type: Some("text/plain".into()),
                        body: Some(MessagePartBody {
                            data: Some(b"plain version".to_vec()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    MessagePart {
                        mime_type: Some("text/html".into()),
                        body: Some(MessagePartBody {
                            data: Some(b"<b>html version</b>".to_vec()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(extract_plain_text_body(&m).as_deref(), Some("plain version"));
    }

    #[test]
    fn extract_plain_text_returns_none_when_html_only() {
        let m = Message {
            payload: Some(MessagePart {
                mime_type: Some("text/html".into()),
                body: Some(MessagePartBody {
                    data: Some(b"<i>only html</i>".to_vec()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(extract_plain_text_body(&m).is_none());
    }

    #[test]
    fn rfc2822_includes_required_headers_and_body() {
        let msg = build_rfc2822("you@example.com", "hi", "body line\nsecond line", None);
        assert!(msg.starts_with("To: you@example.com\r\n"));
        assert!(msg.contains("Subject: hi\r\n"));
        assert!(msg.contains("Content-Type: text/plain"));
        assert!(msg.ends_with("body line\nsecond line"));
    }

    #[test]
    fn rfc2822_threads_via_in_reply_to() {
        let msg = build_rfc2822("a@b", "re: x", "ack", Some("<orig@example>"));
        assert!(msg.contains("In-Reply-To: <orig@example>\r\n"));
        assert!(msg.contains("References: <orig@example>\r\n"));
    }
}

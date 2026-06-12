//! ARAWN-I-0062 T-D: UAT-mode Slack tools backed by the projection store.
//!
//! Production Slack has no search-across-channels tool — the agent does
//! `slack_list_channels` + `slack_history` per channel and filters in
//! prose. UAT mirrors that workflow: list distinct channel ids from
//! `slack_messages` rows, then return messages for a given channel.

use std::path::{Path, PathBuf};

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Value, json};

use arawn_projections::ProjectionStore;

#[derive(Debug, Clone, Serialize)]
struct MessageSummary {
    ts: String,
    user: Option<String>,
    text: Option<String>,
    thread_ts: Option<String>,
    reply_count: Option<usize>,
    reactions: Vec<ReactionSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct ReactionSummary {
    name: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ChannelSummary {
    id: String,
    name: String,
    kind: String,
    member_count: Option<usize>,
    topic: Option<String>,
    purpose: Option<String>,
}

fn open_store(data_dir: &Path) -> Result<ProjectionStore, ToolError> {
    let store = ProjectionStore::open(&data_dir.join("projections.db"))
        .map_err(|e| ToolError::ExecutionFailed(format!("open projections: {e}")))?;
    store
        .ensure_feed_type(arawn_projections::slack::TOPLEVEL_FEED_TYPE)
        .map_err(|e| ToolError::ExecutionFailed(format!("ensure schema: {e}")))?;
    Ok(store)
}

/// Channel-kind hint inferred from a Slack channel id prefix.
/// C = public, G = private, D = im, M = mpim.
fn kind_for(id: &str) -> &'static str {
    match id.chars().next() {
        Some('C') => "public",
        Some('G') => "private",
        Some('D') => "im",
        Some('M') => "mpim",
        _ => "public",
    }
}

/// `slack_list_channels` UAT impl — distinct channel ids appearing in the
/// `slack_messages` projection. Channel "names" are derived from the
/// channel-id suffix (so `C-platform` → `platform`).
pub struct UatSlackListChannelsTool {
    data_dir: PathBuf,
}

impl UatSlackListChannelsTool {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn list_channels(&self) -> Result<Vec<ChannelSummary>, ToolError> {
        let store = open_store(&self.data_dir)?;
        let conn = store.conn().lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT json_extract(metadata, '$.channel_id') AS channel_id \
                 FROM slack_messages \
                 WHERE channel_id IS NOT NULL \
                 ORDER BY channel_id",
            )
            .map_err(|e| ToolError::ExecutionFailed(format!("prepare list channels: {e}")))?;
        let ids = stmt
            .query_map([], |r| r.get::<_, Option<String>>(0))
            .map_err(|e| ToolError::ExecutionFailed(format!("query: {e}")))?;
        let mut out = Vec::new();
        for r in ids {
            let id = match r.map_err(|e| ToolError::ExecutionFailed(format!("row: {e}")))? {
                Some(id) => id,
                None => continue,
            };
            let name = id
                .strip_prefix('C')
                .or_else(|| id.strip_prefix('G'))
                .or_else(|| id.strip_prefix('D'))
                .or_else(|| id.strip_prefix('M'))
                .map(|rest| rest.trim_start_matches('-').to_string())
                .unwrap_or_else(|| id.clone());
            out.push(ChannelSummary {
                id: id.clone(),
                name,
                kind: kind_for(&id).to_string(),
                member_count: None,
                topic: None,
                purpose: None,
            });
        }
        Ok(out)
    }
}

#[async_trait]
impl Tool for UatSlackListChannelsTool {
    fn name(&self) -> &str {
        "slack_list_channels"
    }
    fn description(&self) -> &str {
        "List channels (public, private, DMs, group DMs) the bot can see in the connected Slack \
         workspace. Use this to discover channel ids before reading history. Returns id, name, \
         kind (public/private/im/mpim), member_count, topic, purpose."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Slack
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        _params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let out = self.list_channels()?;
        Ok(ToolOutput::success(serde_json::to_string(&out).unwrap()))
    }
}

/// `slack_history` UAT impl — returns messages for one channel, newest first.
pub struct UatSlackHistoryTool {
    data_dir: PathBuf,
}

impl UatSlackHistoryTool {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn channel_history(
        &self,
        channel: &str,
        limit: usize,
    ) -> Result<Vec<MessageSummary>, ToolError> {
        let store = open_store(&self.data_dir)?;
        let conn = store.conn().lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, body_text, source_ts, metadata \
                 FROM slack_messages \
                 WHERE json_extract(metadata, '$.channel_id') = ?1 \
                 ORDER BY source_ts DESC LIMIT ?2",
            )
            .map_err(|e| ToolError::ExecutionFailed(format!("prepare history: {e}")))?;
        let rows = stmt
            .query_map((channel, limit as i64), |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| ToolError::ExecutionFailed(format!("query: {e}")))?;
        let mut out = Vec::new();
        for r in rows {
            let (_id, body, ts, meta) =
                r.map_err(|e| ToolError::ExecutionFailed(format!("row: {e}")))?;
            let meta_val: Value = serde_json::from_str(&meta).unwrap_or(Value::Null);
            let user = meta_val
                .get("sender_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            let thread_ts = meta_val
                .get("thread_ts")
                .and_then(|v| v.as_str())
                .map(String::from);
            let reactions: Vec<ReactionSummary> = meta_val
                .get("reactions")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|r| {
                            let name = r.get("name").and_then(|v| v.as_str())?.to_string();
                            let count =
                                r.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                            Some(ReactionSummary { name, count })
                        })
                        .collect()
                })
                .unwrap_or_default();
            out.push(MessageSummary {
                ts,
                user,
                text: Some(body),
                thread_ts,
                reply_count: None,
                reactions,
            });
        }
        Ok(out)
    }
}

#[async_trait]
impl Tool for UatSlackHistoryTool {
    fn name(&self) -> &str {
        "slack_history"
    }
    fn description(&self) -> &str {
        "Read recent messages from a Slack channel by id. Returns ts, user id, text, thread_ts, \
         reply_count, and a reactions summary. Use slack_list_channels first to discover ids."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Slack
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "Slack channel id (C/G/D/M-prefixed)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max messages (default 20, max 200)",
                    "minimum": 1,
                    "maximum": 200
                }
            },
            "required": ["channel"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let channel = params
            .get("channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'channel'".into()))?;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20)
            .min(200) as usize;
        let out = self.channel_history(channel, limit)?;
        Ok(ToolOutput::success(serde_json::to_string(&out).unwrap()))
    }
}

pub fn uat_slack_tools(data_dir: PathBuf) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(UatSlackListChannelsTool::new(data_dir.clone())) as Box<dyn Tool>,
        Box::new(UatSlackHistoryTool::new(data_dir)) as Box<dyn Tool>,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_projections::slack::{SlackMessageProjection, TOPLEVEL_FEED_TYPE};
    use chrono::Utc;

    fn fixture_dir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let store = ProjectionStore::open(&tmp.path().join("projections.db")).unwrap();
        store.ensure_feed_type(TOPLEVEL_FEED_TYPE).unwrap();
        let now = Utc::now();
        for (i, (ch, text, sender)) in [
            ("C-platform", "@pat ping on the ledger dashboard", "U-jamie"),
            ("C-platform", "weekend on-call is empty", "U-jamie"),
            ("C-random", "lunch at noon", "U-mei"),
        ]
        .iter()
        .enumerate()
        {
            let proj = SlackMessageProjection {
                id: format!("sl-{i}"),
                feed_id: "f".into(),
                source_id: format!("sl-{i}"),
                source_ts: now - chrono::Duration::hours(i as i64),
                channel_id: Some((*ch).into()),
                sender_id: Some((*sender).into()),
                text: (*text).into(),
                thread_ts: None,
                reactions: Vec::new(),
                is_thread_reply: false,
            };
            store.write(&proj).unwrap();
        }
        tmp
    }

    #[test]
    fn list_channels_returns_distinct_ids() {
        let tmp = fixture_dir();
        let tool = UatSlackListChannelsTool::new(tmp.path().to_path_buf());
        let channels = tool.list_channels().unwrap();
        let ids: Vec<&str> = channels.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"C-platform"));
        assert!(ids.contains(&"C-random"));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn history_returns_messages_for_one_channel_newest_first() {
        let tmp = fixture_dir();
        let tool = UatSlackHistoryTool::new(tmp.path().to_path_buf());
        let messages = tool.channel_history("C-platform", 20).unwrap();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].text.as_deref().unwrap().contains("@pat"));
    }
}

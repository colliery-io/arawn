//! Engine tools wrapping the Slack integration.
//!
//! Four tools land in v1: `slack_list_channels`, `slack_history`,
//! `slack_post`, `slack_react`. `slack_search` is deferred —
//! slack-morphism doesn't typed-expose `search.messages`, and the agent
//! can scan per-channel history to answer most "what was discussed"
//! questions in the meantime.

use std::sync::Arc;

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Value, json};
use slack_morphism::prelude::{
    SlackApiChatPostMessageRequest, SlackApiConversationsHistoryRequest,
    SlackApiConversationsListRequest, SlackApiReactionsAddRequest, SlackChannelId,
    SlackConversationType, SlackMessageContent, SlackReactionName, SlackTs,
};
// `value()` lives on the rvstruct::ValueStruct trait; pull it into scope so
// SlackChannelId::value() / SlackTs::value() / etc. are callable on the
// newtypes slack-morphism uses for ids.
use rvstruct::ValueStruct;

use super::integration::SlackIntegration;

fn integ_err(e: crate::IntegrationError) -> ToolError {
    ToolError::ExecutionFailed(e.user_message())
}

/// `slack-morphism::ClientError` → `ToolError`. Wraps the message; the engine
/// error chain (T-0191) ferries the body to the user.
fn slack_err(stage: &str, e: slack_morphism::errors::SlackClientError) -> ToolError {
    ToolError::ExecutionFailed(format!("Slack {stage}: {e}"))
}

/// Compact, agent-friendly channel summary. The full `SlackChannelInfo`
/// has dozens of fields most of which the agent doesn't need; we project
/// down to the obvious ones plus a `kind` enum so the model can route on it.
#[derive(Debug, Clone, Serialize)]
struct ChannelSummary {
    id: String,
    name: Option<String>,
    kind: String, // "public" | "private" | "im" | "mpim"
    member_count: Option<u64>,
    is_archived: Option<bool>,
    topic: Option<String>,
    purpose: Option<String>,
}

fn summarize_channel(c: &slack_morphism::prelude::SlackChannelInfo) -> ChannelSummary {
    let kind = if c.flags.is_im.unwrap_or(false) {
        "im"
    } else if c.flags.is_mpim.unwrap_or(false) {
        "mpim"
    } else if c.flags.is_private.unwrap_or(false) {
        "private"
    } else {
        "public"
    };
    ChannelSummary {
        id: c.id.value().clone(),
        name: c.name.clone(),
        kind: kind.to_string(),
        member_count: c.num_members,
        is_archived: c.flags.is_archived,
        topic: c.topic.as_ref().map(|t| t.value.clone()),
        purpose: c.purpose.as_ref().map(|p| p.value.clone()),
    }
}

/// Compact message record — what the agent sees from `slack_history`.
#[derive(Debug, Clone, Serialize)]
struct MessageSummary {
    ts: String,
    /// User id (e.g. `U12345`) — agent resolves to a name via list_channels
    /// or a future users_lookup tool.
    user: Option<String>,
    text: Option<String>,
    thread_ts: Option<String>,
    /// Number of replies in this thread, when this message is a thread root.
    reply_count: Option<usize>,
    /// Reactions, summarized as `[{name, count}]`.
    reactions: Vec<ReactionSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct ReactionSummary {
    name: String,
    count: usize,
}

fn summarize_message(m: &slack_morphism::prelude::SlackHistoryMessage) -> MessageSummary {
    let reactions: Vec<ReactionSummary> = m
        .content
        .reactions
        .as_ref()
        .map(|rs| {
            rs.iter()
                .map(|r| ReactionSummary {
                    name: r.name.value().clone(),
                    count: r.count,
                })
                .collect()
        })
        .unwrap_or_default();

    MessageSummary {
        ts: m.origin.ts.value().clone(),
        user: m.sender.user.as_ref().map(|u| u.value().clone()),
        text: m.content.text.clone(),
        thread_ts: m.origin.thread_ts.as_ref().map(|t| t.value().clone()),
        reply_count: m.parent.reply_count,
        reactions,
    }
}

// ─── /slack_list_channels ─────────────────────────────────────────────────

pub struct SlackListChannelsTool {
    integration: Arc<SlackIntegration>,
}

impl SlackListChannelsTool {
    pub fn new(integration: Arc<SlackIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for SlackListChannelsTool {
    fn name(&self) -> &str {
        "slack_list_channels"
    }
    fn description(&self) -> &str {
        "List channels (public, private, DMs, group DMs) the bot can see in the connected Slack \
         workspace. Use this to discover channel ids before reading history. Returns id, name, \
         kind (public/private/im/mpim), member_count, topic, purpose."
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
                "include_dms": {
                    "type": "boolean",
                    "description": "Include direct-message conversations (default false)"
                },
                "include_private": {
                    "type": "boolean",
                    "description": "Include private channels the bot is a member of (default true)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max channels to return per page (default 100, max 1000)",
                    "minimum": 1,
                    "maximum": 1000
                }
            }
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let include_dms = params.get("include_dms").and_then(|v| v.as_bool()).unwrap_or(false);
        let include_private = params.get("include_private").and_then(|v| v.as_bool()).unwrap_or(true);
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(100).min(1000) as u16;

        let mut types = vec![SlackConversationType::Public];
        if include_private {
            types.push(SlackConversationType::Private);
        }
        if include_dms {
            types.push(SlackConversationType::Im);
            types.push(SlackConversationType::Mpim);
        }

        let req = SlackApiConversationsListRequest::new()
            .with_types(types)
            .with_limit(limit)
            .with_exclude_archived(true);

        let ctx = self.integration.context().map_err(integ_err)?;
        let session = ctx.session();
        let resp = session
            .conversations_list(&req)
            .await
            .map_err(|e| slack_err("conversations.list", e))?;
        let channels: Vec<ChannelSummary> = resp.channels.iter().map(summarize_channel).collect();
        Ok(ToolOutput::success(serde_json::to_string(&channels).unwrap()))
    }
}

// ─── /slack_history ───────────────────────────────────────────────────────

pub struct SlackHistoryTool {
    integration: Arc<SlackIntegration>,
}

impl SlackHistoryTool {
    pub fn new(integration: Arc<SlackIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for SlackHistoryTool {
    fn name(&self) -> &str {
        "slack_history"
    }
    fn description(&self) -> &str {
        "Read recent messages from a Slack channel. Returns ts, user (Slack user id like U12345), \
         text, thread_ts, reply_count, reaction summaries. Channel must be a Slack channel id (C12345 \
         for public, G12345 for private, D12345 for IM); use slack_list_channels to discover ids."
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
                "channel": {
                    "type": "string",
                    "description": "Slack channel id (C/G/D/M-prefixed)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max messages (default 20, max 200)",
                    "minimum": 1,
                    "maximum": 200
                },
                "oldest": {
                    "type": "string",
                    "description": "Only messages after this Slack ts (e.g. '1714867200.000000')"
                },
                "latest": {
                    "type": "string",
                    "description": "Only messages before this Slack ts"
                }
            },
            "required": ["channel"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let channel = params
            .get("channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'channel'".into()))?
            .to_string();
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20).min(200) as u16;
        let oldest = params.get("oldest").and_then(|v| v.as_str()).map(|s| SlackTs::new(s.to_string()));
        let latest = params.get("latest").and_then(|v| v.as_str()).map(|s| SlackTs::new(s.to_string()));

        let mut req = SlackApiConversationsHistoryRequest::new()
            .with_channel(SlackChannelId::new(channel))
            .with_limit(limit);
        if let Some(o) = oldest {
            req = req.with_oldest(o);
        }
        if let Some(l) = latest {
            req = req.with_latest(l);
        }

        let ctx = self.integration.context().map_err(integ_err)?;
        let session = ctx.session();
        let resp = session
            .conversations_history(&req)
            .await
            .map_err(|e| slack_err("conversations.history", e))?;
        let messages: Vec<MessageSummary> = resp.messages.iter().map(summarize_message).collect();
        Ok(ToolOutput::success(serde_json::to_string(&messages).unwrap()))
    }
}

// ─── /slack_post ──────────────────────────────────────────────────────────

pub struct SlackPostTool {
    integration: Arc<SlackIntegration>,
}

impl SlackPostTool {
    pub fn new(integration: Arc<SlackIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for SlackPostTool {
    fn name(&self) -> &str {
        "slack_post"
    }
    fn description(&self) -> &str {
        "Post a plain-text message to a Slack channel or as a thread reply. `channel` accepts a \
         channel id (C12345) or name (#general). For a thread reply, set `thread_ts` to the root \
         message's ts. Returns the new message's ts."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Other
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "Channel id (C/G/D-prefixed) or name (#channel-name)"
                },
                "text": {
                    "type": "string",
                    "description": "Plain text body. mrkdwn formatting is supported."
                },
                "thread_ts": {
                    "type": "string",
                    "description": "Optional thread root message ts to reply to"
                }
            },
            "required": ["channel", "text"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let channel = params
            .get("channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'channel'".into()))?
            .to_string();
        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'text'".into()))?
            .to_string();
        let thread_ts = params
            .get("thread_ts")
            .and_then(|v| v.as_str())
            .map(|s| SlackTs::new(s.to_string()));

        let content = SlackMessageContent::new().with_text(text);
        let mut req = SlackApiChatPostMessageRequest::new(SlackChannelId::new(channel), content);
        if let Some(t) = thread_ts {
            req = req.with_thread_ts(t);
        }

        let ctx = self.integration.context().map_err(integ_err)?;
        let session = ctx.session();
        let resp = session
            .chat_post_message(&req)
            .await
            .map_err(|e| slack_err("chat.postMessage", e))?;

        let payload = json!({
            "channel": resp.channel.value(),
            "ts": resp.ts.value(),
        });
        Ok(ToolOutput::success(payload.to_string()))
    }
}

// ─── /slack_react ─────────────────────────────────────────────────────────

pub struct SlackReactTool {
    integration: Arc<SlackIntegration>,
}

impl SlackReactTool {
    pub fn new(integration: Arc<SlackIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for SlackReactTool {
    fn name(&self) -> &str {
        "slack_react"
    }
    fn description(&self) -> &str {
        "Add an emoji reaction to a Slack message. `name` is the emoji name without colons \
         (e.g. 'thumbsup', not ':thumbsup:'). `channel` is the channel id, `ts` is the message ts."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::FileWrite
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "channel": { "type": "string", "description": "Channel id" },
                "ts": { "type": "string", "description": "Message ts" },
                "name": { "type": "string", "description": "Emoji name without colons" }
            },
            "required": ["channel", "ts", "name"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let channel = params
            .get("channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'channel'".into()))?
            .to_string();
        let ts = params
            .get("ts")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'ts'".into()))?
            .to_string();
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'name'".into()))?
            .trim_matches(':')
            .to_string();

        let req = SlackApiReactionsAddRequest::new(
            SlackChannelId::new(channel),
            SlackReactionName::new(name.clone()),
            SlackTs::new(ts.clone()),
        );

        let ctx = self.integration.context().map_err(integ_err)?;
        let session = ctx.session();
        session
            .reactions_add(&req)
            .await
            .map_err(|e| slack_err("reactions.add", e))?;
        Ok(ToolOutput::success(json!({"ok": true, "name": name, "ts": ts}).to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slack_morphism::prelude::{
        SlackChannelDetails, SlackChannelFlags, SlackChannelInfo, SlackDateTime,
        SlackHistoryMessage, SlackMessageContent, SlackMessageOrigin, SlackMessageSender,
        SlackParentMessageParams, SlackReaction, SlackReactionName, SlackTs, SlackUserId,
    };

    fn channel(id: &str, kind: &str) -> SlackChannelInfo {
        let mut flags = SlackChannelFlags::new();
        match kind {
            "im" => flags = flags.with_is_im(true),
            "mpim" => flags = flags.with_is_mpim(true),
            "private" => flags = flags.with_is_private(true),
            _ => flags = flags.with_is_channel(true),
        }
        SlackChannelInfo::new(
            slack_morphism::prelude::SlackChannelId::new(id.to_string()),
            SlackDateTime::new(chrono::Utc::now()),
            flags,
            slack_morphism::prelude::SlackChannelCurrentState::new(),
        )
    }

    #[test]
    fn summarize_channel_classifies_kind_correctly() {
        let pub_ch = channel("C001", "public");
        assert_eq!(summarize_channel(&pub_ch).kind, "public");
        let priv_ch = channel("G001", "private");
        assert_eq!(summarize_channel(&priv_ch).kind, "private");
        let im = channel("D001", "im");
        assert_eq!(summarize_channel(&im).kind, "im");
        let mpim = channel("M001", "mpim");
        assert_eq!(summarize_channel(&mpim).kind, "mpim");
    }

    #[test]
    fn summarize_channel_carries_topic_and_purpose() {
        let mut ch = channel("C100", "public");
        ch.topic = Some(SlackChannelDetails::new("morning standup".into()));
        ch.purpose = Some(SlackChannelDetails::new("daily 9am sync".into()));
        ch.num_members = Some(7);
        let s = summarize_channel(&ch);
        assert_eq!(s.topic.as_deref(), Some("morning standup"));
        assert_eq!(s.purpose.as_deref(), Some("daily 9am sync"));
        assert_eq!(s.member_count, Some(7));
    }

    #[test]
    fn summarize_message_extracts_user_text_and_reactions() {
        let mut content = SlackMessageContent::new().with_text("hello world".into());
        content.reactions = Some(vec![SlackReaction {
            name: SlackReactionName::new("thumbsup".into()),
            count: 3,
            users: vec![],
        }]);
        let m = SlackHistoryMessage::new(
            SlackMessageOrigin::new(SlackTs::new("1714867200.000100".into())),
            content,
            SlackMessageSender {
                user: Some(SlackUserId::new("U001".into())),
                bot_id: None,
                username: None,
                display_as_bot: None,
                user_profile: None,
                bot_profile: None,
            },
            SlackParentMessageParams::new(),
        );
        let s = summarize_message(&m);
        assert_eq!(s.ts, "1714867200.000100");
        assert_eq!(s.user.as_deref(), Some("U001"));
        assert_eq!(s.text.as_deref(), Some("hello world"));
        assert_eq!(s.reactions.len(), 1);
        assert_eq!(s.reactions[0].name, "thumbsup");
        assert_eq!(s.reactions[0].count, 3);
    }
}

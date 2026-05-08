//! Slack — what feeds need from Slack, plus the production adapter
//! over `arawn-integrations` + `slack-morphism`.
//!
//! Templates depend on the [`SlackFeedClient`] trait. Tests fake it
//! externally; production wires [`RealSlackClient`] which reuses the
//! same `SlackIntegration` (and persisted token) the rest of the
//! Slack tools use.

use std::sync::Arc;

use arawn_integrations::slack::SlackIntegration;
use async_trait::async_trait;
use slack_morphism::prelude::{
    SlackApiConversationsHistoryRequest, SlackApiConversationsListRequest, SlackChannelId,
    SlackConversationType, SlackTs,
};

use crate::error::FeedError;

/// What feeds need from Slack. Designed for the
/// `slack/channel-archive` flow: list channel history since a
/// `latest_ts` cursor.
///
/// Kept small on purpose — only the methods feeds actually use. As
/// more Slack templates land, this trait grows but the surface is
/// always feed-driven, not "everything Slack can do."
#[async_trait]
pub trait SlackFeedClient: Send + Sync {
    /// Resolve a channel name (`#design`) or id (`CABCDEF`) to its
    /// channel id. Used at registration time to validate `params`.
    async fn resolve_channel(&self, name_or_id: &str) -> Result<String, FeedError>;

    /// Fetch messages from `channel_id` newer than `oldest_ts`. If
    /// `oldest_ts` is `None`, returns recent history (slack-side
    /// default). Returned messages are oldest-first; `next_cursor_ts`
    /// is the highest `ts` seen so the template persists it as the
    /// next cursor.
    async fn channel_history(
        &self,
        channel_id: &str,
        oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError>;
}

/// One page of Slack channel history. Templates don't paginate — the
/// client either returns everything since the cursor in one call, or
/// pages internally.
#[derive(Debug, Clone)]
pub struct SlackHistoryPage {
    /// Raw API messages, oldest-first. Each entry is the raw JSON
    /// payload Slack returned — templates write this verbatim to disk
    /// to preserve full fidelity.
    pub messages: Vec<serde_json::Value>,
    /// Slack's `ts` of the newest message in this page, or the prior
    /// cursor if no new messages. The template persists this as the
    /// next cursor.
    pub next_cursor_ts: Option<String>,
}

// ─── Production adapter ──────────────────────────────────────────────

pub struct RealSlackClient {
    integration: Arc<SlackIntegration>,
}

impl RealSlackClient {
    pub fn new(integration: Arc<SlackIntegration>) -> Self {
        Self { integration }
    }
}

fn integ_err(e: arawn_integrations::IntegrationError) -> FeedError {
    use arawn_integrations::IntegrationError;
    match e {
        IntegrationError::NotConnected(msg) => FeedError::Auth(msg),
        other => FeedError::Provider(other.user_message()),
    }
}

fn slack_morphism_err<E: std::fmt::Display>(op: &str, e: E) -> FeedError {
    let msg = e.to_string();
    if msg.contains("rate") || msg.contains("ratelimit") || msg.contains("Retry-After") {
        FeedError::RateLimited { retry_after: None }
    } else if msg.contains("invalid_auth")
        || msg.contains("token_revoked")
        || msg.contains("not_authed")
    {
        FeedError::Auth(format!("{op}: {msg}"))
    } else {
        FeedError::Provider(format!("{op}: {msg}"))
    }
}

#[async_trait]
impl SlackFeedClient for RealSlackClient {
    async fn resolve_channel(&self, name_or_id: &str) -> Result<String, FeedError> {
        if looks_like_channel_id(name_or_id) {
            return Ok(name_or_id.to_string());
        }
        let stripped = name_or_id.trim_start_matches('#');

        // Prefer the user context if available so we see private
        // channels the user is in but the bot isn't invited to.
        let ctx = self
            .integration
            .user_context()
            .or_else(|_| self.integration.bot_context())
            .map_err(integ_err)?;
        let session = ctx.session();

        let req = SlackApiConversationsListRequest::new()
            .with_types(vec![
                SlackConversationType::Public,
                SlackConversationType::Private,
            ])
            .with_limit(1000)
            .with_exclude_archived(true);
        let resp = session
            .conversations_list(&req)
            .await
            .map_err(|e| slack_morphism_err("conversations.list", e))?;

        for ch in &resp.channels {
            if ch.name.as_deref() == Some(stripped) {
                return Ok(ch.id.to_string());
            }
        }
        Err(FeedError::InvalidParams(format!(
            "no Slack channel matching '{name_or_id}' (looked at {} channels)",
            resp.channels.len()
        )))
    }

    async fn channel_history(
        &self,
        channel_id: &str,
        oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        let ctx = self
            .integration
            .user_context()
            .or_else(|_| self.integration.bot_context())
            .map_err(integ_err)?;
        let session = ctx.session();

        let mut req = SlackApiConversationsHistoryRequest::new()
            .with_channel(SlackChannelId::new(channel_id.to_string()))
            .with_limit(200);
        if let Some(o) = oldest_ts {
            req = req.with_oldest(SlackTs::new(o.to_string()));
        }
        let resp = session
            .conversations_history(&req)
            .await
            .map_err(|e| slack_morphism_err("conversations.history", e))?;

        // slack-morphism returns messages newest-first. Reverse so the
        // JSONL appendlog reads chronologically.
        let mut messages: Vec<serde_json::Value> = resp
            .messages
            .iter()
            .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
            .collect();
        messages.reverse();

        let next_cursor_ts = messages
            .iter()
            .filter_map(|m| m.get("ts").and_then(|v| v.as_str()))
            .max()
            .map(str::to_string)
            .or_else(|| oldest_ts.map(str::to_string));

        Ok(SlackHistoryPage {
            messages,
            next_cursor_ts,
        })
    }
}

fn looks_like_channel_id(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    matches!(bytes[0], b'C' | b'G' | b'D' | b'M')
        && bytes[1..].iter().all(|b| b.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_id_recognized_by_prefix() {
        assert!(looks_like_channel_id("CABCDEF"));
        assert!(looks_like_channel_id("G123456"));
        assert!(looks_like_channel_id("D000111"));
        assert!(looks_like_channel_id("M99XYZ"));
    }

    #[test]
    fn names_not_recognized_as_ids() {
        assert!(!looks_like_channel_id("design"));
        assert!(!looks_like_channel_id("#design"));
        assert!(!looks_like_channel_id(""));
        assert!(!looks_like_channel_id("Z123"));
    }
}

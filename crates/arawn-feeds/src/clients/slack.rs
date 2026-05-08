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
    SlackApiConversationsHistoryRequest, SlackApiConversationsListRequest,
    SlackApiConversationsOpenRequest, SlackApiConversationsRepliesRequest,
    SlackApiUsersListRequest, SlackChannelId, SlackConversationType, SlackTs, SlackUserId,
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

    /// Fetch replies in a thread newer than `oldest_ts`. The first call
    /// (`oldest_ts: None`) returns the parent + every reply; subsequent
    /// calls with the prior `next_cursor_ts` return only deltas.
    /// Returned messages are oldest-first.
    ///
    /// Slack's API includes the parent message in the first page, so
    /// the template MUST dedupe against the channel-level cursor it
    /// already has if the parent is the same message — otherwise the
    /// parent ends up in two places. (For our `slack/channel-archive`
    /// template the parent is intentionally written once to the day
    /// file and once to the thread file as the conversation context.)
    async fn thread_replies(
        &self,
        channel_id: &str,
        parent_ts: &str,
        oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError>;

    /// Resolve a Slack user reference (id `UABC123` or username) to
    /// the channel id of a 1-on-1 DM with that user. Idempotent —
    /// `conversations.open` returns the existing DM channel if one
    /// already exists; otherwise creates one.
    async fn open_dm(&self, user_id_or_name: &str) -> Result<String, FeedError>;

    /// Identify the user the OAuth token belongs to. Used by
    /// `slack/my-mentions` to build the `<@user_id>` search query.
    /// Templates cache the result in their cursor — cheap call, but
    /// no reason to repeat it every fire.
    async fn auth_test(&self) -> Result<SlackAuthInfo, FeedError>;

    /// Run a Slack search.messages query and return the result page
    /// shaped like `SlackHistoryPage` (oldest-first, with a
    /// `next_cursor_ts` of the highest ts seen).
    ///
    /// `query` follows Slack's search query syntax — common pattern
    /// for mentions is `"<@U01ABC>"` (the literal Slack mention
    /// token). Add `oldest_ts` to filter to messages newer than the
    /// cursor; the adapter converts to Slack's `after:YYYY-MM-DD`
    /// query operator (Slack's search doesn't support exact-ts
    /// filtering, so we round down to the day and dedupe in the
    /// caller).
    async fn search_messages(
        &self,
        query: &str,
        oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError>;
}

/// Subset of Slack `auth.test` response that feeds care about.
#[derive(Debug, Clone, Default)]
pub struct SlackAuthInfo {
    pub user_id: String,
    pub team_id: String,
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

    async fn thread_replies(
        &self,
        channel_id: &str,
        parent_ts: &str,
        oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        let ctx = self
            .integration
            .user_context()
            .or_else(|_| self.integration.bot_context())
            .map_err(integ_err)?;
        let session = ctx.session();

        let mut req = SlackApiConversationsRepliesRequest::new(
            SlackChannelId::new(channel_id.to_string()),
            SlackTs::new(parent_ts.to_string()),
        )
        .with_limit(200);
        if let Some(o) = oldest_ts {
            req = req.with_oldest(SlackTs::new(o.to_string()));
        }
        let resp = session
            .conversations_replies(&req)
            .await
            .map_err(|e| slack_morphism_err("conversations.replies", e))?;

        let messages: Vec<serde_json::Value> = resp
            .messages
            .iter()
            .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
            .collect();

        // conversations.replies returns parent + replies oldest-first
        // already; no reverse needed (unlike history).
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

    async fn open_dm(&self, user_id_or_name: &str) -> Result<String, FeedError> {
        let user_id = if looks_like_user_id(user_id_or_name) {
            user_id_or_name.to_string()
        } else {
            self.resolve_user_name_to_id(user_id_or_name).await?
        };

        let ctx = self
            .integration
            .user_context()
            .or_else(|_| self.integration.bot_context())
            .map_err(integ_err)?;
        let session = ctx.session();

        let req = SlackApiConversationsOpenRequest::new()
            .with_users(vec![SlackUserId::new(user_id)])
            .with_return_im(true);
        let resp = session
            .conversations_open(&req)
            .await
            .map_err(|e| slack_morphism_err("conversations.open", e))?;
        Ok(resp.channel.id.to_string())
    }

    async fn auth_test(&self) -> Result<SlackAuthInfo, FeedError> {
        // Prefer the user context — `search.messages` requires user
        // token anyway, so we identify *that* user.
        let ctx = self
            .integration
            .user_context()
            .or_else(|_| self.integration.bot_context())
            .map_err(integ_err)?;
        let session = ctx.session();
        let resp = session
            .auth_test()
            .await
            .map_err(|e| slack_morphism_err("auth.test", e))?;
        Ok(SlackAuthInfo {
            user_id: resp.user_id.to_string(),
            team_id: resp.team_id.to_string(),
        })
    }

    async fn search_messages(
        &self,
        query: &str,
        oldest_ts: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        // slack-morphism doesn't expose search.messages; drop down to
        // raw HTTP against the same user token. Search requires the
        // `search:read` scope which is user-token only — fail fast
        // with Auth if there's no user context.
        use rvstruct::ValueStruct;
        let user_ctx = self.integration.user_context().map_err(integ_err)?;
        let user_token = user_ctx.token.token_value.value().to_string();

        // Slack's search supports `after:YYYY-MM-DD` for time-window
        // filtering. ts → date is lossy (we may re-fetch up to one
        // day's worth on each tick), so we dedupe in the caller using
        // the precise ts cursor.
        let mut full_query = query.to_string();
        if let Some(ts) = oldest_ts
            && let Some(date) = ts_to_yyyy_mm_dd(ts) {
                full_query.push_str(&format!(" after:{date}"));
            }

        let client = reqwest::Client::new();
        let resp = client
            .get("https://slack.com/api/search.messages")
            .bearer_auth(&user_token)
            .query(&[
                ("query", full_query.as_str()),
                ("sort", "timestamp"),
                ("sort_dir", "asc"),
                ("count", "100"),
                ("highlight", "false"),
            ])
            .send()
            .await
            .map_err(|e| slack_morphism_err("search.messages send", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| slack_morphism_err("search.messages decode", e))?;

        let ok = body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let err_str = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            // Map known errors to typed FeedError variants.
            return Err(match err_str {
                "missing_scope" | "no_permission" => FeedError::Auth(format!(
                    "search.messages: missing scope (need `search:read`): {err_str}"
                )),
                "not_authed" | "invalid_auth" | "token_revoked" => {
                    FeedError::Auth(format!("search.messages: {err_str}"))
                }
                "ratelimited" => FeedError::RateLimited { retry_after: None },
                _ => FeedError::Provider(format!("search.messages: {err_str}")),
            });
        }

        let matches: Vec<serde_json::Value> = body
            .pointer("/messages/matches")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let next_cursor_ts = matches
            .iter()
            .filter_map(|m| m.get("ts").and_then(|v| v.as_str()))
            .max()
            .map(str::to_string)
            .or_else(|| oldest_ts.map(str::to_string));

        Ok(SlackHistoryPage {
            messages: matches,
            next_cursor_ts,
        })
    }
}

/// Lossy conversion from Slack's float-string `ts` to a `YYYY-MM-DD`
/// string. Returns `None` on parse failure (caller treats as "no
/// after: filter" rather than aborting).
fn ts_to_yyyy_mm_dd(ts: &str) -> Option<String> {
    use chrono::TimeZone;
    let secs: i64 = ts.split('.').next()?.parse().ok()?;
    let dt = chrono::Utc.timestamp_opt(secs, 0).single()?;
    Some(dt.format("%Y-%m-%d").to_string())
}

impl RealSlackClient {
    async fn resolve_user_name_to_id(&self, name: &str) -> Result<String, FeedError> {
        let stripped = name.trim_start_matches('@');
        let ctx = self
            .integration
            .user_context()
            .or_else(|_| self.integration.bot_context())
            .map_err(integ_err)?;
        let session = ctx.session();
        let req = SlackApiUsersListRequest::new().with_limit(1000);
        let resp = session
            .users_list(&req)
            .await
            .map_err(|e| slack_morphism_err("users.list", e))?;
        for member in &resp.members {
            // Match on profile.display_name first (preferred), fall
            // back to .name (legacy username).
            let display = member
                .profile
                .as_ref()
                .and_then(|p| p.display_name.as_deref());
            let name_field = member.name.as_deref();
            if display == Some(stripped) || name_field == Some(stripped) {
                return Ok(member.id.to_string());
            }
        }
        Err(FeedError::InvalidParams(format!(
            "no Slack user matching '{name}'"
        )))
    }
}

fn looks_like_user_id(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 2
        && bytes[0] == b'U'
        && bytes[1..].iter().all(|b| b.is_ascii_alphanumeric())
}

fn looks_like_channel_id(s: &str) -> bool {
    classify_channel_id(s).is_some()
}

/// Slack conversation kind, classified by id prefix.
///
/// Slack treats every conversational surface — public channel, private
/// channel, 1-on-1 DM, group DM — as a "channel" with a one-letter
/// prefix on the id. The dispatch path through `conversations.history`
/// / `conversations.replies` is identical for all four; the prefix
/// only changes which `*:history` OAuth scope is required and which
/// access path makes sense for the user (name vs user-id vs M-id).
///
/// Used by:
/// - `/watch` (T-0219) to render the right picker for the input.
/// - `slack/channel-archive` to validate a raw id before running.
/// - `slack/dm-archive` to short-circuit when the user passes a `D`-id
///   directly instead of a username.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    /// `C…` — public channel. Discoverable by name (`#design`).
    /// Needs `channels:history` to read.
    Public,
    /// `G…` — private channel. Discoverable by name if user is a
    /// member. Needs `groups:history` to read.
    Private,
    /// `D…` — 1-on-1 DM. Identified by the user on the other side.
    /// Needs `im:history` to read.
    DirectMessage,
    /// `M…` — group DM (3+ members). No stable name; addressable only
    /// by id. Needs `mpim:history` to read.
    GroupDm,
}

impl ChannelKind {
    /// Required Slack OAuth scope to call `conversations.history` on
    /// this kind. Useful for surfacing precise scope-missing errors
    /// before we hit the API.
    pub fn history_scope(self) -> &'static str {
        match self {
            ChannelKind::Public => "channels:history",
            ChannelKind::Private => "groups:history",
            ChannelKind::DirectMessage => "im:history",
            ChannelKind::GroupDm => "mpim:history",
        }
    }

    /// Recommended template to archive this kind.
    pub fn recommended_template(self) -> &'static str {
        match self {
            ChannelKind::Public | ChannelKind::Private | ChannelKind::GroupDm => {
                "slack/channel-archive"
            }
            ChannelKind::DirectMessage => "slack/dm-archive",
        }
    }
}

/// Classify a Slack id by its prefix. Returns `None` for anything
/// that doesn't look like a channel id (e.g. names, user ids).
pub fn classify_channel_id(s: &str) -> Option<ChannelKind> {
    let bytes = s.as_bytes();
    if bytes.len() < 2 || !bytes[1..].iter().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    Some(match bytes[0] {
        b'C' => ChannelKind::Public,
        b'G' => ChannelKind::Private,
        b'D' => ChannelKind::DirectMessage,
        b'M' => ChannelKind::GroupDm,
        _ => return None,
    })
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

    #[test]
    fn classify_returns_kind_for_each_prefix() {
        assert_eq!(classify_channel_id("CABC"), Some(ChannelKind::Public));
        assert_eq!(classify_channel_id("GABC"), Some(ChannelKind::Private));
        assert_eq!(classify_channel_id("DABC"), Some(ChannelKind::DirectMessage));
        assert_eq!(classify_channel_id("MABC"), Some(ChannelKind::GroupDm));
        assert_eq!(classify_channel_id("ZABC"), None);
        assert_eq!(classify_channel_id("design"), None);
        assert_eq!(classify_channel_id(""), None);
    }

    #[test]
    fn channel_kind_exposes_required_scope() {
        assert_eq!(ChannelKind::Public.history_scope(), "channels:history");
        assert_eq!(ChannelKind::Private.history_scope(), "groups:history");
        assert_eq!(ChannelKind::DirectMessage.history_scope(), "im:history");
        assert_eq!(ChannelKind::GroupDm.history_scope(), "mpim:history");
    }

    #[test]
    fn channel_kind_recommends_correct_template() {
        assert_eq!(
            ChannelKind::Public.recommended_template(),
            "slack/channel-archive"
        );
        assert_eq!(
            ChannelKind::DirectMessage.recommended_template(),
            "slack/dm-archive"
        );
        // Both private channels and group DMs go through channel-archive
        assert_eq!(
            ChannelKind::Private.recommended_template(),
            "slack/channel-archive"
        );
        assert_eq!(
            ChannelKind::GroupDm.recommended_template(),
            "slack/channel-archive"
        );
    }

    #[test]
    fn user_id_recognized_by_prefix() {
        assert!(looks_like_user_id("UABC123"));
        assert!(looks_like_user_id("U01XYZ"));
        assert!(!looks_like_user_id("alice"));
        assert!(!looks_like_user_id("@alice"));
        assert!(!looks_like_user_id(""));
        assert!(!looks_like_user_id("CABC")); // channel, not user
    }
}

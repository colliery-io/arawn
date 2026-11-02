//! `slack/dm-archive` — archive a 1-on-1 DM conversation with the
//! same dual-layer (day file + per-thread file) storage as
//! `slack/channel-archive`.
//!
//! Slack DMs are channels with a `D` prefix. After resolving
//! `params.user` to a DM channel id (via `conversations.open`), the
//! archive flow is identical to channel-archive — we delegate to
//! `super::common::archive_channel_with_threads`.
//!
//! Disk layout:
//!
//! ```text
//! slack/dm-archive/<feed_id>/
//!   ├── meta.json                       # cursor: { latest_ts, threads: {...} }
//!   ├── 2026-05-08.jsonl                # top-level DM messages by ts
//!   └── threads/
//!       └── <parent_ts>.jsonl           # threaded replies, even in DMs
//! ```

use std::path::Path;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::common::{archive_channel_with_threads, synth_since_cursor};
use super::workspace::{check_workspace, client, workspace_param, workspace_rows, workspace_spec};
use crate::error::FeedError;
use crate::template::{DiscoveryRow, FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, TemplateParams};

pub struct DmArchiveTemplate;

const NAME: &str = "slack/dm-archive";

#[async_trait]
impl FeedTemplate for DmArchiveTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        workspace_param(NAME, params)?;
        let user = params.get_str("user").ok_or_else(|| {
            FeedError::InvalidParams(
                "slack/dm-archive requires `user` (Slack user id like 'UABC123' or username)"
                    .into(),
            )
        })?;
        if user.trim().is_empty() {
            return Err(FeedError::InvalidParams(
                "slack/dm-archive `user` cannot be empty".into(),
            ));
        }
        Ok(())
    }

    fn validate_with(&self, ctx: &TemplateCtx, params: &TemplateParams) -> Result<(), FeedError> {
        check_workspace(NAME, ctx, params)?;
        self.validate(params)
    }

    fn param_schema(&self) -> Vec<crate::param_schema::ParamSpec> {
        use crate::param_schema::{ParamKind, ParamSpec};
        vec![
            workspace_spec(),
            ParamSpec::required(
                "user",
                "User",
                ParamKind::Text,
                "Slack user ID (e.g. U0123ABCD) or username (e.g. alice / @alice).",
            ),
            ParamSpec::since(),
        ]
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            // DMs are usually lower-volume than channels — hourly is
            // the sensible default. Users can drop to 15 min if needed
            // (still subject to the 15-min floor).
            cadence: "0 * * * *".into(),
            initial_cursor: json!({ "latest_ts": Value::Null, "threads": {} }),
        }
    }

    async fn run(
        &self,
        ctx: &TemplateCtx,
        params: &TemplateParams,
        feed_dir: &Path,
        cursor: &Value,
    ) -> Result<RunOutcome, FeedError> {
        let slack = client(NAME, ctx, params)?;

        let user = params
            .get_str("user")
            .ok_or_else(|| FeedError::InvalidParams("missing `user` param".into()))?;
        let dm_channel_id = slack.open_dm(user).await?;

        let effective_cursor = synth_since_cursor(cursor, params)?;
        archive_channel_with_threads(slack.as_ref(), &dm_channel_id, feed_dir, &effective_cursor)
            .await
    }

    /// The connected workspaces; the user is typed.
    async fn discover(&self, ctx: &TemplateCtx) -> Result<Option<Vec<DiscoveryRow>>, FeedError> {
        Ok(Some(workspace_rows(ctx)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_rejects_missing_user() {
        let t = DmArchiveTemplate;
        let err = t
            .validate(&TemplateParams::new(json!({ "workspace": "acme" })))
            .unwrap_err();
        assert!(matches!(err, FeedError::InvalidParams(_)));
    }

    #[test]
    fn validate_rejects_empty_user() {
        let t = DmArchiveTemplate;
        let err = t
            .validate(&TemplateParams::new(
                json!({ "workspace": "acme", "user": "  " }),
            ))
            .unwrap_err();
        assert!(matches!(err, FeedError::InvalidParams(_)));
    }

    #[test]
    fn validate_accepts_user_id_or_name() {
        let t = DmArchiveTemplate;
        for user in ["UABC123", "alice", "@alice"] {
            t.validate(&TemplateParams::new(
                json!({ "workspace": "acme", "user": user }),
            ))
            .unwrap();
        }
    }
}

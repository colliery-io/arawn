//! `gmail/sender-filter` — watched-sender feed.
//!
//! Archives every new message from a given sender. Param-shaped so
//! the user can stand up one feed per high-signal correspondent
//! (e.g. an oncall alert address, an exec, a vendor).
//!
//! Required param:
//! - `sender_pattern: string` — Gmail's `from:` operator value. Can
//!   be a literal address (`alice@example.com`) or any Gmail-supported
//!   shape (`*@vendor.com`, a name, a saved contact).
//!
//! Optional param:
//! - `days_back: u32` (default 14)
//!
//! Storage shape and cursor: identical to inbox-archive — see
//! [`super::common`].

use std::path::Path;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::common::archive_query;
use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, TemplateParams};

pub struct SenderFilterTemplate;

const NAME: &str = "gmail/sender-filter";
const DEFAULT_DAYS_BACK: u32 = 14;

#[async_trait]
impl FeedTemplate for SenderFilterTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        let sender = params
            .0
            .get("sender_pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                FeedError::InvalidParams("missing required param: sender_pattern".into())
            })?;
        if sender.trim().is_empty() {
            return Err(FeedError::InvalidParams(
                "sender_pattern must not be empty".into(),
            ));
        }
        if let Some(v) = params.0.get("days_back") {
            let n = v.as_u64().ok_or_else(|| {
                FeedError::InvalidParams("days_back must be a non-negative integer".into())
            })?;
            if !(1..=90).contains(&n) {
                return Err(FeedError::InvalidParams(
                    "days_back must be between 1 and 90".into(),
                ));
            }
        }
        Ok(())
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "*/30 * * * *".into(),
            initial_cursor: json!({ "latest_internal_date": Value::Null }),
        }
    }

    async fn run(
        &self,
        ctx: &TemplateCtx,
        params: &TemplateParams,
        feed_dir: &Path,
        cursor: &Value,
    ) -> Result<RunOutcome, FeedError> {
        let gmail = ctx.clients().gmail().ok_or_else(|| {
            FeedError::Auth("gmail integration not connected".into())
        })?;
        let sender = params
            .0
            .get("sender_pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::InvalidParams("missing sender_pattern".into()))?;
        let days_back = params
            .0
            .get("days_back")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_DAYS_BACK as u64);
        // Quote in case the sender contains shell-y characters; Gmail
        // accepts quoted from: values.
        let query = format!("from:\"{sender}\" newer_than:{days_back}d");
        archive_query(gmail, feed_dir, &query, cursor).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_requires_sender_pattern() {
        assert!(SenderFilterTemplate
            .validate(&TemplateParams::default())
            .is_err());
        let p = TemplateParams(json!({ "sender_pattern": "" }));
        assert!(SenderFilterTemplate.validate(&p).is_err());
        let p = TemplateParams(json!({ "sender_pattern": "alice@example.com" }));
        SenderFilterTemplate.validate(&p).unwrap();
    }

    #[test]
    fn validate_rejects_bad_days_back() {
        let p = TemplateParams(json!({
            "sender_pattern": "x@y.z",
            "days_back": 0,
        }));
        assert!(SenderFilterTemplate.validate(&p).is_err());
    }
}

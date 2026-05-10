//! `gmail/inbox-archive` — personal inbox feed.
//!
//! Auto-creates on `/connect gmail` (singleton — only one inbox feed
//! per Gmail account). Captures every new inbox message and stores
//! the full `format=full` payload, partitioned by `internalDate`.
//!
//! Storage layout: see [`super::common`].
//!
//! Optional params:
//! - `days_back: u32` — how far back to look on each run (default 7).
//!   Keeps first-run pulls bounded; once the cursor has caught up,
//!   `days_back` mostly bounds the recovery window after a long
//!   pause.

use std::path::Path;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::common::{archive_query, compose_time_bound};
use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, TemplateParams};

pub struct InboxArchiveTemplate;

const NAME: &str = "gmail/inbox-archive";
const DEFAULT_DAYS_BACK: u32 = 7;

#[async_trait]
impl FeedTemplate for InboxArchiveTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
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
            cadence: "*/15 * * * *".into(),
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
        let days_back = params
            .0
            .get("days_back")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_DAYS_BACK as u64);
        let (time_clause, max_results) = compose_time_bound(
            cursor,
            params.0.get("since").and_then(|v| v.as_str()),
            days_back,
        );
        let query = format!("in:inbox {time_clause}");
        archive_query(gmail, feed_dir, &query, cursor, max_results).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_default_params() {
        InboxArchiveTemplate
            .validate(&TemplateParams::default())
            .unwrap();
    }

    #[test]
    fn validate_rejects_bad_days_back() {
        let p = TemplateParams(json!({ "days_back": 0 }));
        assert!(InboxArchiveTemplate.validate(&p).is_err());
        let p = TemplateParams(json!({ "days_back": 1000 }));
        assert!(InboxArchiveTemplate.validate(&p).is_err());
        let p = TemplateParams(json!({ "days_back": "seven" }));
        assert!(InboxArchiveTemplate.validate(&p).is_err());
    }

    #[test]
    fn defaults_use_15min_cadence() {
        let d = InboxArchiveTemplate.defaults(&TemplateParams::default());
        assert_eq!(d.cadence, "*/15 * * * *");
    }
}

//! `gmail/label-archive` — watched-label feed.
//!
//! Archives every new message under a given Gmail label. Lets the
//! user lean on Gmail's own filter rules to scope an archive: tag
//! anything interesting with a label, point a feed at the label.
//!
//! Required param:
//! - `label: string` — Gmail label name. Both built-in labels
//!   (`IMPORTANT`, `STARRED`, `INBOX`, etc.) and user labels work.
//!   Nested labels use slashes (`Projects/Arawn`).
//!
//! Optional param:
//! - `days_back: u32` (default 30)
//!
//! Storage shape and cursor: identical to inbox-archive — see
//! [`super::common`].
//!
//! Note: we don't validate label existence at registration. Gmail's
//! search will simply return zero results for an unknown label, and
//! labels can be created/renamed/deleted out-of-band; cheaper to let
//! the feed run as a no-op than to bind validity at registration time.

use std::path::Path;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::common::{archive_query, compose_time_bound};
use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, TemplateParams};

pub struct LabelArchiveTemplate;

const NAME: &str = "gmail/label-archive";
const DEFAULT_DAYS_BACK: u32 = 30;

#[async_trait]
impl FeedTemplate for LabelArchiveTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        let label = params
            .0
            .get("label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::InvalidParams("missing required param: label".into()))?;
        if label.trim().is_empty() {
            return Err(FeedError::InvalidParams("label must not be empty".into()));
        }
        if let Some(v) = params.0.get("days_back") {
            let n = v.as_u64().ok_or_else(|| {
                FeedError::InvalidParams("days_back must be a non-negative integer".into())
            })?;
            if !(1..=180).contains(&n) {
                return Err(FeedError::InvalidParams(
                    "days_back must be between 1 and 180".into(),
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
        let label = params
            .0
            .get("label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::InvalidParams("missing label".into()))?;
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
        let query = format!("label:\"{label}\" {time_clause}");
        archive_query(gmail, feed_dir, &query, cursor, max_results).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_requires_label() {
        assert!(LabelArchiveTemplate
            .validate(&TemplateParams::default())
            .is_err());
        let p = TemplateParams(json!({ "label": "" }));
        assert!(LabelArchiveTemplate.validate(&p).is_err());
        let p = TemplateParams(json!({ "label": "Projects/Arawn" }));
        LabelArchiveTemplate.validate(&p).unwrap();
    }
}

//! `confluence/space-archive` — every page in a Confluence space,
//! metadata + raw storage-format body, one directory per page.
//!
//! Required param:
//! - `space_key: string` — Confluence space key (e.g. `"ENG"`).
//!
//! Disk layout:
//!
//! ```text
//! confluence/space-archive/<feed_id>/
//!   ├── meta.json                       # cursor: { last_modified_iso }
//!   └── <page_id>/
//!       ├── page.json                   # ConfluencePageMeta + version
//!       └── body.storage.xml            # raw body, overwrite-on-update
//! ```
//!
//! Storage model:
//!
//! - One directory per page id; both files inside are
//!   overwrite-on-update. Confluence pages are mutable (edits, renames,
//!   moves) and the agent wants the current version; if revision
//!   history is needed later we can add an append-only
//!   `versions/<n>.xml` log.
//! - Bodies are written verbatim as `body.storage.xml`. No ADF
//!   conversion at archive time — agents prefer source-of-truth
//!   markup. The Markdown→storage converter from T-0213 is for write
//!   paths only.
//!
//! Cursor: `{ last_modified_iso }`. CQL's `lastmodified > "..."` is
//! minute-grained, so we re-fetch any page whose modified time matches
//! the cursor exactly; the body fetch is idempotent (overwrite), so
//! that's harmless.
//!
//! Out of scope here:
//! - Comments. A separate `confluence/page-comments` template can
//!   layer on append-only comment logs if anyone asks.
//! - Attachments. Same — a separate template.

use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::clients::ConfluencePageMeta;
use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct SpaceArchiveTemplate;

const NAME: &str = "confluence/space-archive";

#[async_trait]
impl FeedTemplate for SpaceArchiveTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        let key = params
            .0
            .get("space_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                FeedError::InvalidParams("missing required param: space_key".into())
            })?;
        if key.trim().is_empty() {
            return Err(FeedError::InvalidParams(
                "space_key must not be empty".into(),
            ));
        }
        Ok(())
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "*/30 * * * *".into(),
            initial_cursor: json!({ "last_modified_iso": Value::Null }),
        }
    }

    async fn run(
        &self,
        ctx: &TemplateCtx,
        params: &TemplateParams,
        feed_dir: &Path,
        cursor: &Value,
    ) -> Result<RunOutcome, FeedError> {
        let started = Instant::now();
        let atlassian = ctx.clients().atlassian().ok_or_else(|| {
            FeedError::Auth("atlassian integration not connected".into())
        })?;

        let space_key = params
            .0
            .get("space_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::InvalidParams("missing space_key".into()))?
            .to_string();

        let prior_iso: Option<DateTime<Utc>> = cursor
            .get("last_modified_iso")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let pages = atlassian
            .space_pages_modified_since(&space_key, prior_iso)
            .await?;

        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut new_latest: Option<String> = cursor
            .get("last_modified_iso")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        for page in &pages {
            // Fetch body. One bad page shouldn't poison the whole
            // run; log + skip on Schema/Provider errors. Auth and
            // rate-limit propagate so the runtime can back off.
            let body = match atlassian.page_body_storage(&page.id).await {
                Ok(b) => b,
                Err(FeedError::Schema(msg)) | Err(FeedError::Provider(msg)) => {
                    tracing::warn!(
                        target: "arawn::feeds",
                        feed = NAME,
                        page_id = %page.id,
                        error = %msg,
                        "skipping page body fetch"
                    );
                    continue;
                }
                Err(other) => return Err(other),
            };

            let page_dir = feed_dir.join(&page.id);
            std::fs::create_dir_all(&page_dir).map_err(|e| {
                FeedError::Storage(format!("create {}: {e}", page_dir.display()))
            })?;
            let meta_bytes = write_meta(&page_dir, page)?;
            let body_bytes = write_body(&page_dir, body.storage_xml.as_deref())?;
            total_items += 1;
            total_bytes += meta_bytes + body_bytes;

            if let Some(modified) = page.modified_time.as_deref()
                && new_latest.as_deref().map(|n| modified > n).unwrap_or(true) {
                    new_latest = Some(modified.to_string());
                }
        }

        let new_cursor = json!({
            "last_modified_iso": new_latest.map(Value::String).unwrap_or(Value::Null),
        });
        let status = if total_items == 0 {
            "no-new-items".to_string()
        } else {
            "ok".to_string()
        };

        Ok(RunOutcome {
            cursor: new_cursor,
            summary: RunSummary {
                items_written: total_items,
                bytes_written: total_bytes,
                duration: started.elapsed(),
            },
            status,
        })
    }
}

fn write_meta(page_dir: &Path, page: &ConfluencePageMeta) -> Result<u64, FeedError> {
    let path = page_dir.join("page.json");
    let body = serde_json::to_vec_pretty(page)
        .map_err(|e| FeedError::Storage(format!("serialize page meta: {e}")))?;
    let len = body.len() as u64;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &body)
        .map_err(|e| FeedError::Storage(format!("write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| FeedError::Storage(format!("rename {}: {e}", path.display())))?;
    Ok(len)
}

fn write_body(page_dir: &Path, storage_xml: Option<&str>) -> Result<u64, FeedError> {
    let path = page_dir.join("body.storage.xml");
    let body = storage_xml.unwrap_or("").as_bytes();
    let len = body.len() as u64;
    let tmp = path.with_extension("xml.tmp");
    std::fs::write(&tmp, body)
        .map_err(|e| FeedError::Storage(format!("write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| FeedError::Storage(format!("rename {}: {e}", path.display())))?;
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_requires_space_key() {
        assert!(SpaceArchiveTemplate
            .validate(&TemplateParams::default())
            .is_err());
        let p = TemplateParams(json!({ "space_key": "" }));
        assert!(SpaceArchiveTemplate.validate(&p).is_err());
        let p = TemplateParams(json!({ "space_key": "ENG" }));
        SpaceArchiveTemplate.validate(&p).unwrap();
    }

    #[test]
    fn defaults_use_30min_cadence() {
        let d = SpaceArchiveTemplate.defaults(&TemplateParams::default());
        assert_eq!(d.cadence, "*/30 * * * *");
    }
}

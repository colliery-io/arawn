//! Template-name → adapter dispatch.
//!
//! Maps a feed template name (e.g. `gmail/inbox-archive`) to the
//! projection adapter that knows how to walk that feed's on-disk
//! mirror. Per-feed-type modules register themselves here; the feeds
//! runtime calls `project_feed_dir` after a successful template run
//! and after backfill.

use std::path::Path;

use tracing::{debug, warn};

use crate::error::ProjectionError;
use crate::atlassian;
use crate::calendar;
use crate::drive;
use crate::gmail;
use crate::slack;
use crate::store::{ProjectionStore, WriteOutcome};

/// Project every item under the on-disk mirror for `feed_id`, walking
/// `feed_dir` and writing rows for items not yet projected.
///
/// `template_name` is the feed's template (e.g. `gmail/inbox-archive`).
/// Unknown templates are a no-op with a warn — keeps the hook safe to
/// call against feeds we haven't built adapters for yet.
pub fn project_feed_dir(
    store: &ProjectionStore,
    template_name: &str,
    feed_id: &str,
    feed_dir: &Path,
) -> Result<WriteOutcome, ProjectionError> {
    let provider = template_name.split('/').next().unwrap_or(template_name);
    let outcome = match provider {
        "gmail" => {
            let parsed = gmail::walk_feed_dir(feed_id, feed_dir)?;
            dedup_and_write_single_type(store, gmail::FEED_TYPE, feed_id, parsed, |p| {
                p.source_id.clone()
            })?
        }
        "jira" => {
            let result = atlassian::walk_jira_feed_dir(feed_id, feed_dir)?;
            let mut combined = WriteOutcome::default();
            for (feed_type, projections, source_id_extractor) in [
                (
                    atlassian::JIRA_ISSUES,
                    SubBatch::Issues(result.issues),
                    SubKind::IssueKey,
                ),
                (
                    atlassian::JIRA_COMMENTS,
                    SubBatch::Comments(result.comments),
                    SubKind::CommentId,
                ),
                (
                    atlassian::JIRA_HISTORY,
                    SubBatch::History(result.history),
                    SubKind::HistoryId,
                ),
            ] {
                let outcome = atlassian_write_subbatch(
                    store,
                    feed_type,
                    feed_id,
                    projections,
                    source_id_extractor,
                )?;
                combined.inserted += outcome.inserted;
                combined.updated += outcome.updated;
                combined.unchanged += outcome.unchanged;
            }
            combined
        }
        "confluence" => {
            let parsed = atlassian::walk_confluence_feed_dir(feed_id, feed_dir)?;
            dedup_and_write_single_type(
                store,
                atlassian::CONFLUENCE_PAGES,
                feed_id,
                parsed,
                |p| p.source_id.clone(),
            )?
        }
        "calendar" => {
            let parsed = calendar::walk_feed_dir(feed_id, feed_dir)?;
            dedup_and_write_single_type(store, calendar::FEED_TYPE, feed_id, parsed, |p| {
                p.source_id.clone()
            })?
        }
        "drive" => {
            let parsed = drive::walk_feed_dir(feed_id, feed_dir)?;
            dedup_and_write_single_type(store, drive::FEED_TYPE, feed_id, parsed, |p| {
                p.source_id.clone()
            })?
        }
        "slack" => {
            let parsed = slack::walk_feed_dir(feed_id, feed_dir)?;
            let (top, reply): (Vec<_>, Vec<_>) =
                parsed.into_iter().partition(|p| !p.is_thread_reply);
            let mut combined = WriteOutcome::default();
            let o = dedup_and_write_single_type(
                store,
                slack::TOPLEVEL_FEED_TYPE,
                feed_id,
                top,
                |p| p.source_id.clone(),
            )?;
            combined.inserted += o.inserted;
            combined.updated += o.updated;
            combined.unchanged += o.unchanged;
            let o = dedup_and_write_single_type(
                store,
                slack::THREAD_FEED_TYPE,
                feed_id,
                reply,
                |p| p.source_id.clone(),
            )?;
            combined.inserted += o.inserted;
            combined.updated += o.updated;
            combined.unchanged += o.unchanged;
            combined
        }
        other => {
            warn!(
                template = %template_name,
                provider = %other,
                "no projection adapter registered; skipping"
            );
            return Ok(WriteOutcome::default());
        }
    };
    if outcome.inserted > 0 || outcome.updated > 0 {
        debug!(
            template = %template_name,
            feed_id = %feed_id,
            inserted = outcome.inserted,
            updated = outcome.updated,
            "projected feed"
        );
    }
    Ok(outcome)
}

enum SubBatch {
    Issues(Vec<atlassian::JiraIssueProjection>),
    Comments(Vec<atlassian::JiraCommentProjection>),
    History(Vec<atlassian::JiraHistoryProjection>),
}

enum SubKind {
    IssueKey,
    CommentId,
    HistoryId,
}

fn atlassian_write_subbatch(
    store: &ProjectionStore,
    feed_type: &str,
    feed_id: &str,
    sub: SubBatch,
    _kind: SubKind,
) -> Result<WriteOutcome, ProjectionError> {
    match sub {
        SubBatch::Issues(v) => {
            dedup_and_write_single_type(store, feed_type, feed_id, v, |p| p.source_id.clone())
        }
        SubBatch::Comments(v) => {
            dedup_and_write_single_type(store, feed_type, feed_id, v, |p| p.source_id.clone())
        }
        SubBatch::History(v) => {
            dedup_and_write_single_type(store, feed_type, feed_id, v, |p| p.source_id.clone())
        }
    }
}

fn dedup_and_write_single_type<P, F>(
    store: &ProjectionStore,
    feed_type: &str,
    feed_id: &str,
    parsed: Vec<P>,
    source_id_of: F,
) -> Result<WriteOutcome, ProjectionError>
where
    P: crate::types::Projection,
    F: Fn(&P) -> String,
{
    if parsed.is_empty() {
        return Ok(WriteOutcome::default());
    }
    let source_ids: Vec<String> = parsed.iter().map(&source_id_of).collect();
    let missing = store.missing_source_ids(feed_type, feed_id, &source_ids)?;
    let to_write: Vec<_> = parsed
        .into_iter()
        .filter(|p| missing.contains(&source_id_of(p)))
        .collect();
    if to_write.is_empty() {
        return Ok(WriteOutcome::default());
    }
    store.write_batch(&to_write)
}

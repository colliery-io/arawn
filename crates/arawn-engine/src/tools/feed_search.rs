//! `feed_search` — cross-feed semantic + structured search over
//! `arawn-projections`. Phase 2 of I-0040: no lens needed.
//!
//! Today this runs FTS5 only (embedding pipeline is a separate
//! follow-up — see `<feed_type>_embeddings` in arawn-projections).
//! When the embed pass lands the tool gets a hybrid path with RRF
//! fusion, no API change.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;
use serde_json::{Value, json};

use arawn_embed::Embedder;
use arawn_projections::ProjectionStore;

use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

const KNOWN_FEED_TYPES: &[&str] = &[
    "gmail_messages",
    "slack_messages",
    "slack_thread_messages",
    "drive_files",
    "jira_issues",
    "jira_comments",
    "jira_history",
    "confluence_pages",
    "calendar_events",
    // I-0045 / I-0050 GitHub projection tables — T-0345.
    "github_notifications",
    "github_issues_and_prs",
    "github_review_queue",
    "github_repo_commits",
    "github_repo_issues",
    "github_repo_prs",
    "github_issue_or_pr_comments",
    // I-0057 local filesystem feed — T-0419.
    "filesystem_signals",
];

/// RRF constant (Cormack et al. 2009). Same value the memory bench
/// uses; smaller values favor top-ranked items, larger values smooth
/// the contribution from each list.
const RRF_K: f32 = 60.0;

pub struct FeedSearchTool {
    store: Arc<ProjectionStore>,
    /// Optional embedder; when present the tool runs hybrid FTS +
    /// vector search and RRF-fuses the two rankings.
    embedder: Option<Arc<dyn Embedder>>,
}

impl FeedSearchTool {
    pub fn new(store: Arc<ProjectionStore>, embedder: Option<Arc<dyn Embedder>>) -> Self {
        Self { store, embedder }
    }
}

#[async_trait]
impl Tool for FeedSearchTool {
    fn name(&self) -> &str {
        "feed_search"
    }

    fn description(&self) -> &str {
        "Search the raw projection rows of your continual data feeds (gmail, slack, \
         drive, jira, confluence, calendar). Hybrid FTS5 + semantic similarity \
         (RRF-fused when an embedder is configured); **default limit is 10**, so \
         pass `limit` higher for sweeps where you can't afford to drop items.\n\n\
         **Reach for the live integration tool first when one applies** — \
         `slack_history` / `slack_list_channels` for any Slack-specific question \
         (\"who said X\", \"@-mentions\", \"today in #X\"), `gmail_inbox_read` / \
         `gmail_search` for inbox lookups, `calendar_upcoming` for schedule \
         questions, `drive_search` for files. Those tools return complete, \
         present-tense data uncapped by FTS top-10 and labeled with the source's \
         own identifiers. `feed_search` is the fallback for cross-source sweeps \
         when no single live tool fits.\n\n\
         For *synthesizing across sources* — morning briefing, \"what's on my \
         plate today\", \"what did we decide about X\" — call `signal_search` / \
         `signal_query` first; the extracted signal stream surfaces \
         relationships (calendar conflicts, related mentions) the raw FTS top-10 \
         will miss. The daily tablet (`daily_list_items`) is a curated brief, \
         not a substitute for either.\n\n\
         Use `feed_types` to scope (e.g. `[\"gmail_messages\"]` for inbox-only), \
         `since`/`until` (RFC3339) for time windows."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Memory
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Free-text search query. Goes to FTS5."
                },
                "feed_types": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Restrict to feed types. Defaults to all. Known: gmail_messages, slack_messages, slack_thread_messages, drive_files, jira_issues, jira_comments, jira_history, confluence_pages, calendar_events."
                },
                "since": {
                    "type": "string",
                    "description": "RFC3339 timestamp; filter source_ts >= since"
                },
                "until": {
                    "type": "string",
                    "description": "RFC3339 timestamp; filter source_ts <= until"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results across all feed types (default 10, max 50)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'query'".into()))?;
        let feed_types: Vec<String> = params
            .get("feed_types")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| KNOWN_FEED_TYPES.iter().map(|s| s.to_string()).collect());
        let since = params
            .get("since")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok());
        let until = params
            .get("until")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(50) as usize;

        // Compute the query embedding once (when available) so we can
        // pair FTS + vector ranks per feed type.
        let query_vec = match self.embedder.as_ref() {
            Some(emb) => match emb.embed(query).await {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::warn!(error = %e, "feed_search: query embedding failed; falling back to FTS-only");
                    None
                }
            },
            None => None,
        };

        let mut fused: HashMap<String, FusedHit> = HashMap::new();
        for ft in &feed_types {
            // FTS ranks
            let fts_ids = self
                .store
                .fts_search(ft, query, limit * 4)
                .map_err(|e| ToolError::ExecutionFailed(format!("fts ({ft}): {e}")))?;
            for (rank, id) in fts_ids.iter().enumerate() {
                fused
                    .entry(key(ft, id))
                    .or_insert_with(|| FusedHit::new(ft.clone(), id.clone()))
                    .score += rrf_score(rank);
            }

            // Vector ranks (when an embedder is wired and produced a vector)
            if let Some(qv) = query_vec.as_ref() {
                let vec_ids = self
                    .store
                    .vector_search(ft, qv, limit * 4)
                    .map_err(|e| ToolError::ExecutionFailed(format!("vec ({ft}): {e}")))?;
                for (rank, id) in vec_ids.iter().enumerate() {
                    fused
                        .entry(key(ft, id))
                        .or_insert_with(|| FusedHit::new(ft.clone(), id.clone()))
                        .score += rrf_score(rank);
                }
            }
        }

        // Hydrate + filter by time window in one pass; drops anything
        // outside [since, until] before sorting.
        let mut hits: Vec<Hit> = Vec::new();
        for (_, fh) in fused.into_iter() {
            let row = match self
                .store
                .get_row(&fh.feed_type, &fh.projection_id)
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("hydrate ({}): {e}", fh.feed_type))
                })? {
                Some(r) => r,
                None => continue,
            };
            if let Some(s) = since
                && row.source_ts < s.with_timezone(&chrono::Utc)
            {
                continue;
            }
            if let Some(u) = until
                && row.source_ts > u.with_timezone(&chrono::Utc)
            {
                continue;
            }
            hits.push(Hit {
                score: fh.score,
                row,
            });
        }
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(limit);

        // Run each hit's body through the prompt-injection guard.
        // Blocked hits are dropped from the result with a tracing
        // warning; sanitised hits surface their sanitised snippet so
        // the model sees the quarantine markers.
        let mut blocked = 0usize;
        let mut sanitised = 0usize;
        let results: Vec<Value> = hits
            .into_iter()
            .filter_map(|h| {
                let verdict = crate::prompt_injection::enforce(&h.row.body_text, "feed_search");
                let body_for_snippet: String = match &verdict {
                    crate::prompt_injection::Verdict::Allow => h.row.body_text.clone(),
                    crate::prompt_injection::Verdict::Sanitize { sanitized, .. } => {
                        sanitised += 1;
                        sanitized.clone()
                    }
                    crate::prompt_injection::Verdict::Block { reasons } => {
                        blocked += 1;
                        tracing::warn!(
                            feed_id = %h.row.feed_id,
                            id = %h.row.id,
                            ?reasons,
                            "feed_search dropping blocked hit"
                        );
                        return None;
                    }
                };
                Some(json!({
                    "feed_type": h.row.feed_type,
                    "id": h.row.id,
                    "feed_id": h.row.feed_id,
                    "source_id": h.row.source_id,
                    "source_ts": h.row.source_ts.to_rfc3339(),
                    "title": h.row.title,
                    "snippet": snippet(&body_for_snippet, 240),
                    "score": h.score,
                    "metadata": h.row.metadata,
                }))
            })
            .collect();

        let body = json!({
            "results": results,
            "count": results.len(),
            "blocked": blocked,
            "sanitised": sanitised,
        });
        Ok(ToolOutput::success(body.to_string()))
    }
}

struct Hit {
    score: f32,
    row: arawn_projections::ProjectionRow,
}

/// Per-(feed_type, projection_id) accumulator for RRF scores.
struct FusedHit {
    feed_type: String,
    projection_id: String,
    score: f32,
}

impl FusedHit {
    fn new(feed_type: String, projection_id: String) -> Self {
        Self {
            feed_type,
            projection_id,
            score: 0.0,
        }
    }
}

fn key(feed_type: &str, projection_id: &str) -> String {
    format!("{feed_type}::{projection_id}")
}

/// Reciprocal rank fusion contribution from a single ranked list.
fn rrf_score(rank: usize) -> f32 {
    1.0 / (RRF_K + rank as f32 + 1.0)
}

fn snippet(text: &str, cap: usize) -> String {
    if text.len() <= cap {
        return text.to_string();
    }
    let truncated: String = text.chars().take(cap).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T-0345: cross-feed search must include every GitHub projection
    /// table by default, otherwise `/feed_search "RFC postgres"` skips
    /// every GitHub hit on a lens that has GitHub feeds wired.
    #[test]
    fn known_feed_types_contains_all_github_tables() {
        let github_tables = [
            "github_notifications",
            "github_issues_and_prs",
            "github_review_queue",
            "github_repo_commits",
            "github_repo_issues",
            "github_repo_prs",
            "github_issue_or_pr_comments",
        ];
        for t in github_tables {
            assert!(
                KNOWN_FEED_TYPES.contains(&t),
                "KNOWN_FEED_TYPES missing GitHub table: {t}"
            );
        }
    }

    /// Guard rail: the projection table names in `arawn-projections`
    /// are the source of truth — assert ours match.
    #[test]
    fn known_feed_types_match_projection_constants() {
        let pairs = [
            (
                arawn_projections::github::NOTIFICATIONS_FEED_TYPE,
                "github_notifications",
            ),
            (
                arawn_projections::github::ISSUES_AND_PRS_FEED_TYPE,
                "github_issues_and_prs",
            ),
            (
                arawn_projections::github::REVIEW_QUEUE_FEED_TYPE,
                "github_review_queue",
            ),
            (
                arawn_projections::github::REPO_COMMITS_FEED_TYPE,
                "github_repo_commits",
            ),
            (
                arawn_projections::github::REPO_ISSUES_FEED_TYPE,
                "github_repo_issues",
            ),
            (
                arawn_projections::github::REPO_PRS_FEED_TYPE,
                "github_repo_prs",
            ),
            (
                arawn_projections::github::ISSUE_OR_PR_COMMENTS_FEED_TYPE,
                "github_issue_or_pr_comments",
            ),
        ];
        for (proj_const, hardcoded) in pairs {
            assert_eq!(
                proj_const, hardcoded,
                "table name drift between arawn-projections and feed_search KNOWN_FEED_TYPES"
            );
            assert!(KNOWN_FEED_TYPES.contains(&proj_const));
        }
    }
}

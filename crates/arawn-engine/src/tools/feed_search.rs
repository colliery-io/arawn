//! `feed_search` — cross-feed semantic + structured search over
//! `arawn-projections`. Phase 2 of I-0040: no workstream needed.
//!
//! Today this runs FTS5 only (embedding pipeline is a separate
//! follow-up — see `<feed_type>_embeddings` in arawn-projections).
//! When the embed pass lands the tool gets a hybrid path with RRF
//! fusion, no API change.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;
use serde_json::{Value, json};

use arawn_projections::ProjectionStore;

use crate::tool::{Tool, ToolCategory, ToolError, ToolOutput};

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
];

pub struct FeedSearchTool {
    store: Arc<ProjectionStore>,
}

impl FeedSearchTool {
    pub fn new(store: Arc<ProjectionStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for FeedSearchTool {
    fn name(&self) -> &str {
        "feed_search"
    }

    fn description(&self) -> &str {
        "Search across continual data feeds (gmail, slack, drive, jira, confluence, calendar). \
         Use this for cross-feed lookups when no workstream is declared. Ranks by FTS5 relevance.\n\n\
         Use `feed_types` to scope (e.g. just slack), `since`/`until` (RFC3339) for time windows."
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

        let mut hits: Vec<Hit> = Vec::new();
        for ft in &feed_types {
            let ids = self
                .store
                .fts_search(ft, query, limit * 2)
                .map_err(|e| ToolError::ExecutionFailed(format!("fts ({ft}): {e}")))?;
            for (rank, id) in ids.into_iter().enumerate() {
                let row = match self
                    .store
                    .get_row(ft, &id)
                    .map_err(|e| ToolError::ExecutionFailed(format!("hydrate ({ft}): {e}")))?
                {
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
                    score: 1.0 / (1.0 + rank as f32),
                    row,
                });
            }
        }
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(limit);

        let results: Vec<Value> = hits
            .into_iter()
            .map(|h| {
                json!({
                    "feed_type": h.row.feed_type,
                    "id": h.row.id,
                    "feed_id": h.row.feed_id,
                    "source_id": h.row.source_id,
                    "source_ts": h.row.source_ts.to_rfc3339(),
                    "title": h.row.title,
                    "snippet": snippet(&h.row.body_text, 240),
                    "score": h.score,
                    "metadata": h.row.metadata,
                })
            })
            .collect();

        let body = json!({
            "results": results,
            "count": results.len(),
        });
        Ok(ToolOutput::success(body.to_string()))
    }
}

struct Hit {
    score: f32,
    row: arawn_projections::ProjectionRow,
}

fn snippet(text: &str, cap: usize) -> String {
    if text.len() <= cap {
        return text.to_string();
    }
    let truncated: String = text.chars().take(cap).collect();
    format!("{truncated}…")
}

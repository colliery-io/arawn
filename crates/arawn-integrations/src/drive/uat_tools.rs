//! ARAWN-I-0062 T-F: UAT-mode Drive tools backed by the projection store.
//!
//! Production Drive has a busy surface (`drive_search`, `drive_list`,
//! `drive_get_metadata`, `drive_read`, `drive_upload`, `drive_update`,
//! `drive_delete`). UAT today only needs the read entry point —
//! `drive_search` — because no UAT scenario reaches further. Additional
//! tools (`drive_list`, `drive_get_metadata`, …) can come in T-G or
//! whenever a scenario calls for them; the shared pattern is identical.

use std::path::{Path, PathBuf};

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Value, json};

use arawn_projections::ProjectionStore;

/// Mirror of `tools::FileSummary` (private there) — wire-shape match so the
/// agent sees identical output regardless of which impl backs it.
#[derive(Debug, Clone, Serialize)]
struct FileSummary {
    id: Option<String>,
    name: Option<String>,
    mime_type: Option<String>,
    size: Option<String>,
    modified_time: Option<String>,
    web_view_link: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    owners: Vec<String>,
}

fn open_store(data_dir: &Path) -> Result<ProjectionStore, ToolError> {
    let store = ProjectionStore::open(&data_dir.join("projections.db"))
        .map_err(|e| ToolError::ExecutionFailed(format!("open projections: {e}")))?;
    store
        .ensure_feed_type(arawn_projections::drive::FEED_TYPE)
        .map_err(|e| ToolError::ExecutionFailed(format!("ensure schema: {e}")))?;
    Ok(store)
}

/// UAT impl of `drive_search`. Uses the projection store's FTS over the
/// `drive_files` projection rows after stripping Drive-query-syntax noise so
/// FTS5 doesn't choke on `mimeType = 'application/pdf'`-style operators.
pub struct UatDriveSearchTool {
    data_dir: PathBuf,
}

impl UatDriveSearchTool {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<FileSummary>, ToolError> {
        let store = open_store(&self.data_dir)?;
        let bag = strip_drive_query(query);
        let ids = store
            .fts_search(arawn_projections::drive::FEED_TYPE, &bag, limit)
            .map_err(|e| ToolError::ExecutionFailed(format!("fts: {e}")))?;
        let mut out = Vec::new();
        for id in ids {
            if let Some(row) = store
                .get_row(arawn_projections::drive::FEED_TYPE, &id)
                .map_err(|e| ToolError::ExecutionFailed(format!("get_row: {e}")))?
            {
                let meta = &row.metadata;
                out.push(FileSummary {
                    id: Some(row.source_id),
                    name: meta.get("name").and_then(|v| v.as_str()).map(String::from),
                    mime_type: meta
                        .get("mime_type")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    size: meta
                        .get("size_bytes")
                        .and_then(|v| v.as_u64())
                        .map(|n| n.to_string()),
                    modified_time: Some(row.source_ts.to_rfc3339()),
                    web_view_link: None,
                    owners: Vec::new(),
                });
            }
        }
        Ok(out)
    }
}

#[async_trait]
impl Tool for UatDriveSearchTool {
    fn name(&self) -> &str {
        "drive_search"
    }
    fn description(&self) -> &str {
        "Search Google Drive using Drive's query syntax. Examples: \
         `name contains 'budget'`, `mimeType = 'application/pdf'`, \
         `modifiedTime > '2026-01-01T00:00:00'`, `'<folder_id>' in parents`. \
         Combine with `and` / `or`. Returns id, name, mime_type, size, \
         modified_time, web_view_link, owners. Use drive_get_metadata \
         for full metadata or drive_read for content. Excludes trashed \
         files by default (the query is `and`-joined with `trashed=false`)."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Drive
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Drive query string"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max files to return (default 10, max 100)",
                    "minimum": 1,
                    "maximum": 100
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'query'".into()))?;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(100) as usize;
        let out = self.search(query, limit)?;
        Ok(ToolOutput::success(serde_json::to_string(&out).unwrap()))
    }
}

/// Strip Drive-query operators (`name contains 'x'`, `mimeType = 'pdf'`,
/// `parents in '<id>'`, etc.) down to a plain whitespace bag suitable for
/// FTS5. Production routes the full syntax to Google; UAT only needs the
/// content tokens to land in the search.
fn strip_drive_query(query: &str) -> String {
    let mut out = String::new();
    let mut in_quote = false;
    for c in query.chars() {
        match c {
            '\'' | '"' => in_quote = !in_quote,
            _ if in_quote => out.push(c),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => out.push(c),
            _ => out.push(' '),
        }
    }
    // Drop Drive-syntax keywords + obvious operator tokens. Lowercased so
    // `t.to_ascii_lowercase()` lookups hit case-insensitively
    // (`mimeType` → `mimetype` etc.).
    let stop = [
        "and",
        "or",
        "not",
        "in",
        "contains",
        "true",
        "false",
        "null",
        "name",
        "mimetype",
        "modifiedtime",
        "parents",
        "trashed",
        "fulltext",
    ];
    out.split_whitespace()
        .filter(|t| !stop.contains(&t.to_ascii_lowercase().as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn uat_drive_tools(data_dir: PathBuf) -> Vec<Box<dyn Tool>> {
    vec![Box::new(UatDriveSearchTool::new(data_dir)) as Box<dyn Tool>]
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_projections::drive::{DriveFileProjection, FEED_TYPE};
    use chrono::Utc;

    fn fixture_dir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let store = ProjectionStore::open(&tmp.path().join("projections.db")).unwrap();
        store.ensure_feed_type(FEED_TYPE).unwrap();
        let now = Utc::now();
        for (i, (name, body, mime)) in [
            (
                "Q3 Budget.xlsx",
                "Q3 budget projections",
                "application/vnd.ms-excel",
            ),
            (
                "RFC-0042.md",
                "Postgres replication topology",
                "text/markdown",
            ),
            ("vacation.jpg", "", "image/jpeg"),
        ]
        .iter()
        .enumerate()
        {
            let proj = DriveFileProjection {
                id: format!("dr-{i}"),
                feed_id: "f".into(),
                source_id: format!("file-{i}"),
                source_ts: now - chrono::Duration::hours(i as i64),
                path: format!("/My Drive/{name}"),
                name: (*name).into(),
                mime_type: Some((*mime).into()),
                size_bytes: 1024 * (i as u64 + 1),
                body_text: (*body).into(),
            };
            store.write(&proj).unwrap();
        }
        tmp
    }

    #[test]
    fn search_finds_by_filename_substring() {
        let tmp = fixture_dir();
        let tool = UatDriveSearchTool::new(tmp.path().to_path_buf());
        let hits = tool.search("name contains 'budget'", 10).unwrap();
        assert!(
            hits.iter()
                .any(|f| f.name.as_deref() == Some("Q3 Budget.xlsx")),
            "expected budget hit, got {hits:?}"
        );
    }

    #[test]
    fn search_finds_by_body_text_content() {
        let tmp = fixture_dir();
        let tool = UatDriveSearchTool::new(tmp.path().to_path_buf());
        let hits = tool.search("Postgres", 10).unwrap();
        assert!(
            hits.iter()
                .any(|f| f.name.as_deref() == Some("RFC-0042.md")),
            "expected RFC hit via body text, got {hits:?}"
        );
    }

    #[test]
    fn strip_keeps_content_drops_operators() {
        let q = strip_drive_query("name contains 'budget' and mimeType = 'application/pdf'");
        assert!(q.contains("budget"));
        assert!(q.contains("application"));
        // Operators / keywords are gone (case-insensitive).
        assert!(
            !q.split_whitespace()
                .any(|t| t.eq_ignore_ascii_case("contains"))
        );
        assert!(
            !q.split_whitespace()
                .any(|t| t.eq_ignore_ascii_case("mimeType"))
        );
    }
}

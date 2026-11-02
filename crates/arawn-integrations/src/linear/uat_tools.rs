//! UAT-mode Linear read tools backed by the projection store
//! (COLLIERY-I-0532), as for Slack and Gmail. Same names and output
//! shape as the production read tools, over seeded `linear_issues` rows.
//! The write tools have no UAT form.

use std::path::{Path, PathBuf};

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_projections::ProjectionStore;
use arawn_projections::linear::FEED_TYPE;

fn open_store(data_dir: &Path) -> Result<ProjectionStore, ToolError> {
    let store = ProjectionStore::open(&data_dir.join("projections.db"))
        .map_err(|e| ToolError::ExecutionFailed(format!("open projections: {e}")))?;
    store
        .ensure_feed_type(FEED_TYPE)
        .map_err(|e| ToolError::ExecutionFailed(format!("ensure schema: {e}")))?;
    Ok(store)
}

/// One `linear_issues` row: `(source_id, title, body_text, source_ts, metadata)`.
type Row = (String, String, String, String, Value);

fn rows(
    store: &ProjectionStore,
    where_sql: &str,
    arg: Option<&str>,
) -> Result<Vec<Row>, ToolError> {
    let conn = store.conn().lock().unwrap();
    let sql = format!(
        "SELECT source_id, title, body_text, source_ts, metadata FROM {FEED_TYPE} \
         WHERE {where_sql} ORDER BY source_ts DESC"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| ToolError::ExecutionFailed(format!("prepare: {e}")))?;
    let read = |r: &rusqlite::Row<'_>| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
        ))
    };
    let mapped = match arg {
        Some(a) => stmt.query_map([a], read),
        None => stmt.query_map([], read),
    }
    .map_err(|e| ToolError::ExecutionFailed(format!("query: {e}")))?;
    let mut out = Vec::new();
    for r in mapped {
        let (id, title, body, ts, meta) =
            r.map_err(|e| ToolError::ExecutionFailed(format!("row: {e}")))?;
        out.push((
            id,
            title,
            body,
            ts,
            serde_json::from_str(&meta).unwrap_or(Value::Null),
        ));
    }
    Ok(out)
}

/// The row title is `ENG-7: Title`; give the bare title.
fn bare_title<'a>(title: &'a str, meta: &Value) -> &'a str {
    meta.get("identifier")
        .and_then(Value::as_str)
        .and_then(|i| title.strip_prefix(i))
        .and_then(|t| t.strip_prefix(": "))
        .unwrap_or(title)
}

fn summary((_, title, _, ts, m): &Row) -> Value {
    json!({
        "identifier": m["identifier"],
        "title": bare_title(title, m),
        "state": m["state"],
        "priority": m["priority_label"],
        "due_date": m["due_date"],
        "team": m["team_key"],
        "assignee": m["assignee"],
        "updated_at": ts,
        "url": m["url"],
    })
}

/// `linear_list_my_issues` UAT impl.
pub struct UatLinearListMyIssuesTool {
    data_dir: PathBuf,
}

impl UatLinearListMyIssuesTool {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn list(&self, state: &str, limit: usize) -> Result<Vec<Value>, ToolError> {
        let closed = "json_extract(metadata, '$.state_type') IN ('completed', 'canceled')";
        let filter = match state {
            "open" => format!("NOT ({closed})"),
            "closed" => closed.to_string(),
            "all" => "1".to_string(),
            other => {
                return Err(ToolError::ExecutionFailed(format!(
                    "unknown state filter `{other}`. Use open, closed or all"
                )));
            }
        };
        let store = open_store(&self.data_dir)?;
        Ok(rows(&store, &filter, None)?
            .iter()
            .take(limit)
            .map(summary)
            .collect())
    }
}

#[async_trait]
impl Tool for UatLinearListMyIssuesTool {
    fn name(&self) -> &str {
        "linear_list_my_issues"
    }
    fn description(&self) -> &str {
        "List the Linear issues assigned to me or created by me, most recently updated \
         first. `state`: open (default), closed or all. Gives identifier, title, state, \
         priority, due date, team, assignee and URL per issue."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Linear
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "state": { "type": "string", "enum": ["open", "closed", "all"] },
                "limit": { "type": "integer" }
            }
        })
    }
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let state = params
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("open");
        let limit = params
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(25)
            .clamp(1, 100) as usize;
        let issues = self.list(state, limit)?;
        Ok(ToolOutput::success(json!({ "issues": issues }).to_string()))
    }
}

/// `linear_get_issue` UAT impl. Comments are part of `body_text` in the
/// projection, so they come back inside `description`.
pub struct UatLinearGetIssueTool {
    data_dir: PathBuf,
}

impl UatLinearGetIssueTool {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn get(&self, issue: &str) -> Result<Value, ToolError> {
        let store = open_store(&self.data_dir)?;
        let found = rows(
            &store,
            "json_extract(metadata, '$.identifier') = ?1 OR source_id = ?1",
            Some(issue),
        )?;
        let Some((id, title, body, ts, m)) = found.first() else {
            return Err(ToolError::ExecutionFailed(format!(
                "Provider error: linear: no issue `{issue}`"
            )));
        };
        let title = bare_title(title, m);
        Ok(json!({
            "id": id,
            "identifier": m["identifier"],
            "title": title,
            "description": body.strip_prefix(title).map(str::trim_start).unwrap_or(body),
            "state": { "name": m["state"], "type": m["state_type"] },
            "priority": m["priority"],
            "priority_label": m["priority_label"],
            "due_date": m["due_date"],
            "team": { "key": m["team_key"] },
            "labels": m["labels"],
            "assignee": m["assignee"],
            "creator": m["creator"],
            "updated_at": ts,
            "url": m["url"],
        }))
    }
}

#[async_trait]
impl Tool for UatLinearGetIssueTool {
    fn name(&self) -> &str {
        "linear_get_issue"
    }
    fn description(&self) -> &str {
        "Get one Linear issue with its description, state, priority, due date, labels, \
         assignee and the ten newest comments."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Linear
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "issue": { "type": "string" } },
            "required": ["issue"]
        })
    }
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let issue = params
            .get("issue")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'issue'".into()))?;
        Ok(ToolOutput::success(self.get(issue)?.to_string()))
    }
}

pub fn uat_linear_tools(data_dir: PathBuf) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(UatLinearListMyIssuesTool::new(data_dir.clone())) as Box<dyn Tool>,
        Box::new(UatLinearGetIssueTool::new(data_dir)) as Box<dyn Tool>,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_projections::linear::LinearIssueProjection;
    use chrono::Utc;

    fn fixture_dir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let store = ProjectionStore::open(&tmp.path().join("projections.db")).unwrap();
        let issue = |n: u32, state_type: &str| LinearIssueProjection {
            id: format!("li-{n}"),
            feed_id: "linear-mine".into(),
            source_id: format!("uuid-{n}"),
            source_ts: Utc::now() - chrono::Duration::hours(n as i64),
            identifier: format!("ENG-{n}"),
            title: format!("Issue {n}"),
            description: format!("Details of {n}"),
            url: Some(format!("https://linear.app/acme/issue/ENG-{n}")),
            state: Some("Todo".into()),
            state_type: Some(state_type.into()),
            priority: 2,
            priority_label: Some("High".into()),
            due_date: None,
            team_key: Some("ENG".into()),
            labels: vec!["bug".into()],
            assignee: Some("Ada".into()),
            creator: None,
            assigned_to_me: true,
            comments: vec![],
        };
        store
            .write_batch(&[issue(1, "started"), issue(2, "completed")])
            .unwrap();
        tmp
    }

    #[test]
    fn list_filters_by_state_newest_first() {
        let tmp = fixture_dir();
        let tool = UatLinearListMyIssuesTool::new(tmp.path().to_path_buf());
        let open = tool.list("open", 10).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0]["identifier"], "ENG-1");
        assert_eq!(open[0]["title"], "Issue 1");
        let all = tool.list("all", 10).unwrap();
        assert_eq!(all[1]["identifier"], "ENG-2");
        assert!(tool.list("weird", 10).is_err());
    }

    #[test]
    fn get_finds_by_identifier_or_id() {
        let tmp = fixture_dir();
        let tool = UatLinearGetIssueTool::new(tmp.path().to_path_buf());
        let i = tool.get("ENG-2").unwrap();
        assert_eq!(i["state"]["type"], "completed");
        assert_eq!(i["description"], "Details of 2");
        assert_eq!(tool.get("uuid-1").unwrap()["identifier"], "ENG-1");
        assert!(tool.get("ENG-9").is_err());
    }
}

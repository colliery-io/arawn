//! Daily ceremony agent tools (`daily_*`).
//!
//! Mirrors `tools::ceremony` (retro family). Five thin adapters over
//! [`arawn_ceremonies::CeremonyService`] for the daily ceremony:
//! - `daily_run` — fires the dispatcher for `kind = "daily"`.
//! - `daily_current` — returns today's daily tablet or `null`.
//! - `daily_list_items` — lists items on a daily tablet.
//! - `daily_patch_item` — toggle done / edit body.
//! - `daily_add_todo` — user-write path: inserts a new todo into
//!   today's tablet **and** writes a row to `ceremony_todos_rolling`
//!   so future daily generations + the retro `rollover_heat`
//!   detector see it.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_ceremonies::{
    AddItemRequest, CeremonyService, DailyCeremony, DispatchOutcome, ItemKind, ItemPatch,
};

use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

fn map_err(e: arawn_ceremonies::CeremonyError) -> ToolOutput {
    ToolOutput::error(format!("ceremony_error: {e}"))
}

// ============================================================================
// daily_run
// ============================================================================

pub struct DailyRunTool {
    svc: Arc<CeremonyService>,
}

impl DailyRunTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for DailyRunTool {
    fn name(&self) -> &str {
        "daily_run"
    }

    fn description(&self) -> &str {
        "Compose today's **daily ceremony tablet** — a pre-curated \
         per-day brief: a few attention items, calendar highlights, \
         and rolling todos. Idempotent — returns `status: skipped` \
         when today's tablet already exists in a non-`open` state. \
         On success returns `{ status: \"generated\", tablet_id }`. \
         \n\nNOT a substitute for raw inbox/feed reads. If the user \
         asks to \"summarize my inbox\", \"read my gmail\", or \
         \"what's in slack\", use `feed_search` / `signal_search` \
         / `gmail_inbox_read` instead — the tablet only contains \
         items the ceremony already filtered + composed, which \
         won't include every inbox row."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        _params: Value,
    ) -> Result<ToolOutput, ToolError> {
        match self.svc.run("daily").await {
            Ok(DispatchOutcome::Generated { tablet_id, .. }) => Ok(ToolOutput::success(
                json!({ "status": "generated", "tablet_id": tablet_id }).to_string(),
            )),
            Ok(DispatchOutcome::Skipped { reason }) => Ok(ToolOutput::success(
                json!({ "status": "skipped", "reason": reason }).to_string(),
            )),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// daily_current
// ============================================================================

pub struct DailyCurrentTool {
    svc: Arc<CeremonyService>,
}

impl DailyCurrentTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for DailyCurrentTool {
    fn name(&self) -> &str {
        "daily_current"
    }

    fn description(&self) -> &str {
        "Return today's **daily ceremony tablet** metadata (id, \
         period_key, status, …), or `null` if one has not been \
         generated yet. Use `daily_run` to generate. \
         \n\nThe tablet is a per-day *brief*, not the full inbox. \
         For \"summarize my inbox / gmail / slack\" intents, use \
         `feed_search` or `signal_search` instead — those read the \
         raw projection rows the ceremony filtered down."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        _params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let today = DailyCeremony::period_date(chrono::Utc::now());
        match self.svc.get_by_period("daily", &today) {
            Ok(Some(t)) => Ok(ToolOutput::success(
                serde_json::to_string(&t).unwrap_or_else(|_| "{}".into()),
            )),
            Ok(None) => Ok(ToolOutput::success("null")),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// daily_list_items
// ============================================================================

pub struct DailyListItemsTool {
    svc: Arc<CeremonyService>,
}

impl DailyListItemsTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for DailyListItemsTool {
    fn name(&self) -> &str {
        "daily_list_items"
    }

    fn description(&self) -> &str {
        "List items the daily ceremony already composed into \
         today's tablet, optionally filtered by section (`todos`, \
         `attention`, `calendar`). Each item has `id`, \
         `section_key`, `ordinal`, `kind`, `body` (JSON), \
         `citation_id` (the source signal/event/todo id — non-null \
         for engine-composed items, null for user-added items), \
         `done_at`, `created_at`. When summarising the daily to \
         the user, **always quote citation_id values verbatim** so \
         the user can trace each claim back to the source row. \
         \n\n**Not for raw inbox reads.** This returns only the \
         tablet's curated subset (typically 3–6 rows). For \
         \"summarize my inbox\", \"what's in gmail today\", etc., \
         use `feed_search` / `signal_search` / `gmail_inbox_read` \
         to read the underlying rows."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "tablet_id": {"type": "string", "description": "Tablet id from daily_current or daily_run."},
                "section_key": {"type": "string", "description": "Optional — restrict to one section (e.g. `todos`, `attention`, `calendar`)."}
            },
            "required": ["tablet_id"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let tablet_id = match params.get("tablet_id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("tablet_id is required")),
        };
        let section_key = params
            .get("section_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        match self.svc.list_items(&tablet_id, section_key.as_deref()) {
            Ok(items) => Ok(ToolOutput::success(
                serde_json::to_string(&items).unwrap_or_else(|_| "[]".into()),
            )),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// daily_patch_item
// ============================================================================

pub struct DailyPatchItemTool {
    svc: Arc<CeremonyService>,
}

impl DailyPatchItemTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for DailyPatchItemTool {
    fn name(&self) -> &str {
        "daily_patch_item"
    }

    fn description(&self) -> &str {
        "Toggle done or edit the body of a daily item. `done: true` \
         stamps `done_at`; `done: false` clears it. `body` (a JSON \
         object) replaces the current body. Both fields are \
         optional but at least one must be set or the call is a \
         no-op. Use this to mark todos done as the user completes \
         them."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "item_id": {"type": "string", "description": "Item id from daily_list_items."},
                "patch": {
                    "type": "object",
                    "properties": {
                        "done": {"type": "boolean"},
                        "body": {"type": "object"}
                    },
                    "additionalProperties": false
                }
            },
            "required": ["item_id", "patch"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let item_id = match params.get("item_id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("item_id is required")),
        };
        let patch_val = params
            .get("patch")
            .cloned()
            .unwrap_or(Value::Object(Default::default()));
        let patch: ItemPatch = match serde_json::from_value(patch_val) {
            Ok(p) => p,
            Err(e) => return Ok(ToolOutput::error(format!("patch invalid: {e}"))),
        };
        match self.svc.patch_item(&item_id, patch) {
            Ok(dto) => Ok(ToolOutput::success(
                serde_json::to_string(&dto).unwrap_or_else(|_| "{}".into()),
            )),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// daily_add_todo
// ============================================================================

pub struct DailyAddTodoTool {
    svc: Arc<CeremonyService>,
}

impl DailyAddTodoTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for DailyAddTodoTool {
    fn name(&self) -> &str {
        "daily_add_todo"
    }

    fn description(&self) -> &str {
        "Add a fresh todo to today's daily tablet. The todo is also \
         written to the rolling-todos table so future daily \
         generations (and the retro `rollover_heat` detector) see \
         it until it is marked done. Requires today's daily tablet \
         to exist — call `daily_run` first if `daily_current` \
         returns `null`. Returns `{ status: \"added\", item_id, \
         todo_id }`."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "body": {"type": "string", "description": "The todo text. Plain text — stored verbatim under body.text."}
            },
            "required": ["body"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let body_input = match params.get("body").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("body is required")),
        };
        let today = DailyCeremony::period_date(chrono::Utc::now());
        let tablet = match self.svc.get_by_period("daily", &today) {
            Ok(Some(t)) => t,
            Ok(None) => {
                return Ok(ToolOutput::error(
                    "no daily tablet for today — call daily_run first",
                ));
            }
            Err(e) => return Ok(map_err(e)),
        };
        let req = AddItemRequest {
            tablet_id: tablet.id.clone(),
            section_key: "todos".into(),
            kind: ItemKind::Todo,
            body: json!({ "text": body_input }),
        };
        let item = match self.svc.add_item(req) {
            Ok(d) => d,
            Err(e) => return Ok(map_err(e)),
        };
        let todo_id = match self.svc.add_rolling_todo(&body_input, &tablet.id) {
            Ok(id) => id,
            Err(e) => return Ok(map_err(e)),
        };
        Ok(ToolOutput::success(
            json!({
                "status": "added",
                "item_id": item.id,
                "todo_id": todo_id,
            })
            .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_ceremonies::{CeremonyDispatcher, CeremonyError, ConnHandle, DispatchOutcome};
    use async_trait::async_trait;
    use rusqlite::{Connection, params};
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;

    struct StubDispatcher;

    #[async_trait]
    impl CeremonyDispatcher for StubDispatcher {
        async fn dispatch(&self, _kind: &str) -> Result<DispatchOutcome, CeremonyError> {
            Ok(DispatchOutcome::Generated {
                tablet_id: "tablet-stub".into(),
                item_count: 0,
            })
        }
    }

    fn open_svc() -> (TempDir, Arc<CeremonyService>) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let _ = arawn_storage::Database::open(&db_path).expect("migrations");
        let conn = Connection::open(&db_path).unwrap();
        let handle = ConnHandle::new(conn);
        let svc = Arc::new(CeremonyService::new(
            handle,
            Arc::new(StubDispatcher) as Arc<dyn CeremonyDispatcher>,
        ));
        (tmp, svc)
    }

    fn seed_today_daily_tablet(tmp_path: &std::path::Path) -> String {
        let today = DailyCeremony::period_date(chrono::Utc::now());
        let id = format!("daily-{today}");
        let conn = Connection::open(tmp_path.join("test.db")).unwrap();
        conn.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, lenses_scanned) \
             VALUES (?1, 'daily', ?2, ?3, 'open', '[]')",
            params![&id, &today, &chrono::Utc::now().to_rfc3339()],
        )
        .unwrap();
        drop(conn);
        id
    }

    use arawn_core::Lens;
    use uuid::Uuid;

    fn ctx() -> crate::context::EngineToolContext {
        let ws = Lens::scratch("/tmp/test");
        crate::context::EngineToolContext::new(&ws, Uuid::new_v4())
    }

    #[tokio::test]
    async fn daily_run_returns_generated_payload() {
        let (_tmp, svc) = open_svc();
        let tool = DailyRunTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(!out.is_error);
        assert!(out.content.contains("\"status\":\"generated\""));
        assert!(out.content.contains("tablet-stub"));
    }

    #[tokio::test]
    async fn daily_current_returns_null_when_no_tablet() {
        let (_tmp, svc) = open_svc();
        let tool = DailyCurrentTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(!out.is_error);
        assert_eq!(out.content, "null");
    }

    #[tokio::test]
    async fn daily_current_returns_tablet_when_present() {
        let (tmp, svc) = open_svc();
        let id = seed_today_daily_tablet(tmp.path());
        let tool = DailyCurrentTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(!out.is_error);
        assert!(
            out.content.contains(&id),
            "expected tablet id in: {}",
            out.content
        );
    }

    #[tokio::test]
    async fn daily_list_items_rejects_missing_tablet_id() {
        let (_tmp, svc) = open_svc();
        let tool = DailyListItemsTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("tablet_id"));
    }

    #[tokio::test]
    async fn daily_patch_item_validates_input() {
        let (_tmp, svc) = open_svc();
        let tool = DailyPatchItemTool::new(svc);
        let out = tool.execute(&ctx(), json!({"patch": {}})).await.unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("item_id"));
    }

    #[tokio::test]
    async fn daily_add_todo_requires_body() {
        let (_tmp, svc) = open_svc();
        let tool = DailyAddTodoTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("body"));
    }

    #[tokio::test]
    async fn daily_add_todo_errors_when_no_tablet() {
        let (_tmp, svc) = open_svc();
        let tool = DailyAddTodoTool::new(svc);
        let out = tool
            .execute(&ctx(), json!({"body": "ship the thing"}))
            .await
            .unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("daily_run"));
    }

    #[tokio::test]
    async fn daily_add_todo_inserts_item_and_rolling_row() {
        let (tmp, svc) = open_svc();
        let tablet_id = seed_today_daily_tablet(tmp.path());
        let tool = DailyAddTodoTool::new(Arc::clone(&svc));
        let out = tool
            .execute(&ctx(), json!({"body": "write the report"}))
            .await
            .unwrap();
        assert!(!out.is_error, "got error: {}", out.content);
        assert!(out.content.contains("\"status\":\"added\""));
        assert!(out.content.contains("item_id"));
        assert!(out.content.contains("todo_id"));
        // Verify rolling row exists.
        let conn = Connection::open(tmp.path().join("test.db")).unwrap();
        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ceremony_todos_rolling WHERE origin_tablet_id = ?1",
                params![&tablet_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 1);
    }

    #[tokio::test]
    async fn schemas_have_required_field_arrays() {
        let (_tmp, svc) = open_svc();
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(DailyRunTool::new(Arc::clone(&svc))),
            Box::new(DailyCurrentTool::new(Arc::clone(&svc))),
            Box::new(DailyListItemsTool::new(Arc::clone(&svc))),
            Box::new(DailyPatchItemTool::new(Arc::clone(&svc))),
            Box::new(DailyAddTodoTool::new(Arc::clone(&svc))),
        ];
        for t in tools {
            let schema = t.parameters_schema();
            assert_eq!(
                schema["type"],
                "object",
                "{} schema missing object type",
                t.name()
            );
        }
    }
}

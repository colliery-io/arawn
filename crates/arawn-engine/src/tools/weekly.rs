//! Weekly ceremony agent tools (`weekly_*`).
//!
//! Mirrors `tools::ceremony` (retro family) and `tools::daily`. Seven
//! thin adapters over [`arawn_ceremonies::CeremonyService`] for the
//! weekly ceremony — the Monday confirmation flow:
//! - `weekly_run` — fires the dispatcher for `kind = "weekly"`.
//! - `weekly_current` — returns this ISO-week's weekly tablet or `null`.
//! - `weekly_list_items` — lists items on a weekly tablet.
//! - `weekly_list_priorities` — union of confirmed + candidate
//!   priorities; the `source` discriminator tells the agent which
//!   bucket each row sits in.
//! - `weekly_confirm_priority` — promote a candidate item into a
//!   confirmed priority row.
//! - `weekly_reject_priority` — delete a candidate item.
//! - `weekly_add_priority` — user-write path: insert a fresh
//!   confirmed priority directly into `ceremony_priorities`.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_ceremonies::{
    AddPriorityRequest, CeremonyService, DispatchOutcome, plugins::weekly::iso_week,
};

use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

fn map_err(e: arawn_ceremonies::CeremonyError) -> ToolOutput {
    ToolOutput::error(format!("ceremony_error: {e}"))
}

// ============================================================================
// weekly_run
// ============================================================================

pub struct WeeklyRunTool {
    svc: Arc<CeremonyService>,
}

impl WeeklyRunTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for WeeklyRunTool {
    fn name(&self) -> &str {
        "weekly_run"
    }

    fn description(&self) -> &str {
        "Fire the weekly ceremony for the current ISO week. Pulls \
         calendar summary, deadlines, prior retro excerpts, rolling \
         todos, and writes this week's weekly tablet with candidate \
         priorities. Idempotent — if a tablet for this ISO week \
         already exists in a non-`open` state the call returns \
         `status: skipped` with a reason. On success returns \
         `{ status: \"generated\", tablet_id }`. Call this before \
         `weekly_current` if no weekly has been generated yet this \
         week."
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
        match self.svc.run("weekly").await {
            Ok(DispatchOutcome::Generated { tablet_id }) => Ok(ToolOutput::success(
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
// weekly_current
// ============================================================================

pub struct WeeklyCurrentTool {
    svc: Arc<CeremonyService>,
}

impl WeeklyCurrentTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for WeeklyCurrentTool {
    fn name(&self) -> &str {
        "weekly_current"
    }

    fn description(&self) -> &str {
        "Return this ISO-week's weekly tablet, or `null` if one has \
         not been generated yet. Use `weekly_run` to generate one. \
         The returned object has `id`, `kind`, `period_key` (the ISO \
         week, e.g. `2026-W20`), `generated_at`, `status` \
         (`open` | `reviewed` | `unreviewed`), and other metadata."
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
        let period = iso_week(chrono::Utc::now());
        match self.svc.get_by_period("weekly", &period) {
            Ok(Some(t)) => Ok(ToolOutput::success(
                serde_json::to_string(&t).unwrap_or_else(|_| "{}".into()),
            )),
            Ok(None) => Ok(ToolOutput::success("null")),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// weekly_list_items
// ============================================================================

pub struct WeeklyListItemsTool {
    svc: Arc<CeremonyService>,
}

impl WeeklyListItemsTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for WeeklyListItemsTool {
    fn name(&self) -> &str {
        "weekly_list_items"
    }

    fn description(&self) -> &str {
        "List items in a weekly tablet, optionally filtered by \
         section. Sections include `priorities` (candidate priority \
         items awaiting confirmation), `calendar`, `attention`, \
         `retro_excerpts`. Each item has `id`, `section_key`, \
         `ordinal`, `kind`, `body` (JSON), `citation_id` (the source \
         signal/event/retro id — non-null for engine-composed items, \
         null for user-added items), `done_at`, `created_at`. When \
         summarising the weekly to the user, **always quote \
         citation_id values verbatim** so the user can trace each \
         claim back to the source row."
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
                "tablet_id": {"type": "string", "description": "Tablet id from weekly_current or weekly_run."},
                "section_key": {"type": "string", "description": "Optional — restrict to one section (e.g. `priorities`, `calendar`, `attention`)."}
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
// weekly_list_priorities
// ============================================================================

pub struct WeeklyListPrioritiesTool {
    svc: Arc<CeremonyService>,
}

impl WeeklyListPrioritiesTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for WeeklyListPrioritiesTool {
    fn name(&self) -> &str {
        "weekly_list_priorities"
    }

    fn description(&self) -> &str {
        "List priorities for a weekly tablet — the union of confirmed \
         priority rows and yet-unconfirmed candidate items. Each row \
         carries a `source` discriminator: `\"confirmed\"` (already \
         promoted into `ceremony_priorities`) or `\"candidate\"` (a \
         priority-section item still awaiting `weekly_confirm_priority` \
         or `weekly_reject_priority`). **Group by `source` when \
         summarising for the user** so they know which priorities \
         are still pending decision. Each entry has `id`, `tablet_id`, \
         `body`, `rationale`, `citation_id`, `confirmed_at`, `done_at`, \
         `ordinal`."
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
                "tablet_id": {"type": "string", "description": "Tablet id from weekly_current or weekly_run."}
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
        match self.svc.list_priorities(&tablet_id) {
            Ok(items) => Ok(ToolOutput::success(
                serde_json::to_string(&items).unwrap_or_else(|_| "[]".into()),
            )),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// weekly_confirm_priority
// ============================================================================

pub struct WeeklyConfirmPriorityTool {
    svc: Arc<CeremonyService>,
}

impl WeeklyConfirmPriorityTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for WeeklyConfirmPriorityTool {
    fn name(&self) -> &str {
        "weekly_confirm_priority"
    }

    fn description(&self) -> &str {
        "Promote a candidate priority item into a confirmed priority \
         row in `ceremony_priorities`. Idempotent — re-calling on the \
         same item returns the existing priority row. The `item_id` \
         must reference an item in the `priorities` section of a \
         weekly tablet (kind=`priority`). Returns \
         `{ status: \"confirmed\", priority_id }`."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "item_id": {"type": "string", "description": "Item id from weekly_list_items (priorities section)."}
            },
            "required": ["item_id"],
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
        match self.svc.confirm_priority(&item_id) {
            Ok(dto) => Ok(ToolOutput::success(
                json!({ "status": "confirmed", "priority_id": dto.id }).to_string(),
            )),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// weekly_reject_priority
// ============================================================================

pub struct WeeklyRejectPriorityTool {
    svc: Arc<CeremonyService>,
}

impl WeeklyRejectPriorityTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for WeeklyRejectPriorityTool {
    fn name(&self) -> &str {
        "weekly_reject_priority"
    }

    fn description(&self) -> &str {
        "Delete a candidate priority item. Also removes any \
         `ceremony_priorities` row that cites it, so a confirm-then- \
         reject leaves no orphan. Returns \
         `{ status: \"rejected\" }`."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "item_id": {"type": "string", "description": "Item id from weekly_list_items (priorities section)."}
            },
            "required": ["item_id"],
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
        match self.svc.reject_priority(&item_id) {
            Ok(()) => Ok(ToolOutput::success(
                json!({ "status": "rejected" }).to_string(),
            )),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// weekly_add_priority
// ============================================================================

pub struct WeeklyAddPriorityTool {
    svc: Arc<CeremonyService>,
}

impl WeeklyAddPriorityTool {
    pub fn new(svc: Arc<CeremonyService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl Tool for WeeklyAddPriorityTool {
    fn name(&self) -> &str {
        "weekly_add_priority"
    }

    fn description(&self) -> &str {
        "Add a fresh confirmed priority directly to a weekly tablet \
         (user-write path — bypasses the candidate stage). Inserts a \
         row in `ceremony_priorities` with `citation_id = NULL`. Use \
         this when the user wants to add a priority that wasn't \
         surfaced as a candidate by the weekly generator. Returns \
         `{ status: \"added\", priority_id }`."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "tablet_id": {"type": "string", "description": "Tablet id from weekly_current or weekly_run."},
                "body": {"type": "string", "description": "The priority text. Plain text — stored verbatim under body.text."},
                "rationale": {"type": "string", "description": "Optional — why this is a priority this week. Empty string if unset."}
            },
            "required": ["tablet_id", "body"],
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
        let body_input = match params.get("body").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("body is required")),
        };
        let rationale = params
            .get("rationale")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let req = AddPriorityRequest {
            tablet_id,
            body: json!({ "text": body_input }),
            rationale,
        };
        match self.svc.add_priority(req) {
            Ok(dto) => Ok(ToolOutput::success(
                json!({ "status": "added", "priority_id": dto.id }).to_string(),
            )),
            Err(e) => Ok(map_err(e)),
        }
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

    fn seed_current_weekly_tablet(tmp_path: &std::path::Path) -> String {
        let period = iso_week(chrono::Utc::now());
        let id = format!("weekly-{period}");
        let conn = Connection::open(tmp_path.join("test.db")).unwrap();
        conn.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
             VALUES (?1, 'weekly', ?2, ?3, 'open', '[]')",
            params![&id, &period, &chrono::Utc::now().to_rfc3339()],
        )
        .unwrap();
        drop(conn);
        id
    }

    fn seed_priority_candidate_item(tmp_path: &std::path::Path, tablet_id: &str) -> String {
        let item_id = format!("item-{}", uuid::Uuid::new_v4());
        let conn = Connection::open(tmp_path.join("test.db")).unwrap();
        conn.execute(
            "INSERT INTO ceremony_items (id, tablet_id, section_key, ordinal, kind, body, citation_id, created_at) \
             VALUES (?1, ?2, 'priorities', 0, 'priority', ?3, NULL, ?4)",
            params![
                &item_id,
                tablet_id,
                r#"{"text":"ship the thing"}"#,
                &chrono::Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();
        drop(conn);
        item_id
    }

    use arawn_core::Workstream;
    use uuid::Uuid;

    fn ctx() -> crate::context::EngineToolContext {
        let ws = Workstream::scratch("/tmp/test");
        crate::context::EngineToolContext::new(&ws, Uuid::new_v4())
    }

    #[tokio::test]
    async fn weekly_run_returns_generated_payload() {
        let (_tmp, svc) = open_svc();
        let tool = WeeklyRunTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(!out.is_error);
        assert!(out.content.contains("\"status\":\"generated\""));
        assert!(out.content.contains("tablet-stub"));
    }

    #[tokio::test]
    async fn weekly_current_returns_null_when_no_tablet() {
        let (_tmp, svc) = open_svc();
        let tool = WeeklyCurrentTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(!out.is_error);
        assert_eq!(out.content, "null");
    }

    #[tokio::test]
    async fn weekly_current_returns_tablet_when_present() {
        let (tmp, svc) = open_svc();
        let id = seed_current_weekly_tablet(tmp.path());
        let tool = WeeklyCurrentTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(!out.is_error);
        assert!(
            out.content.contains(&id),
            "expected tablet id in: {}",
            out.content
        );
    }

    #[tokio::test]
    async fn weekly_list_items_rejects_missing_tablet_id() {
        let (_tmp, svc) = open_svc();
        let tool = WeeklyListItemsTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("tablet_id"));
    }

    #[tokio::test]
    async fn weekly_list_priorities_rejects_missing_tablet_id() {
        let (_tmp, svc) = open_svc();
        let tool = WeeklyListPrioritiesTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("tablet_id"));
    }

    #[tokio::test]
    async fn weekly_list_priorities_returns_array_for_empty_tablet() {
        let (tmp, svc) = open_svc();
        let tablet_id = seed_current_weekly_tablet(tmp.path());
        let tool = WeeklyListPrioritiesTool::new(svc);
        let out = tool
            .execute(&ctx(), json!({ "tablet_id": tablet_id }))
            .await
            .unwrap();
        assert!(!out.is_error);
        assert_eq!(out.content, "[]");
    }

    #[tokio::test]
    async fn weekly_confirm_priority_requires_item_id() {
        let (_tmp, svc) = open_svc();
        let tool = WeeklyConfirmPriorityTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("item_id"));
    }

    #[tokio::test]
    async fn weekly_confirm_priority_promotes_candidate() {
        let (tmp, svc) = open_svc();
        let tablet_id = seed_current_weekly_tablet(tmp.path());
        let item_id = seed_priority_candidate_item(tmp.path(), &tablet_id);
        let tool = WeeklyConfirmPriorityTool::new(Arc::clone(&svc));
        let out = tool
            .execute(&ctx(), json!({ "item_id": item_id }))
            .await
            .unwrap();
        assert!(!out.is_error, "got error: {}", out.content);
        assert!(out.content.contains("\"status\":\"confirmed\""));
        assert!(out.content.contains("priority_id"));
        let conn = Connection::open(tmp.path().join("test.db")).unwrap();
        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ceremony_priorities cp \
                 JOIN todos t ON t.id = cp.todo_id \
                 WHERE json_extract(t.attrs, '$.citation_id') = ?1",
                params![&item_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 1);
    }

    #[tokio::test]
    async fn weekly_reject_priority_requires_item_id() {
        let (_tmp, svc) = open_svc();
        let tool = WeeklyRejectPriorityTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("item_id"));
    }

    #[tokio::test]
    async fn weekly_reject_priority_deletes_candidate() {
        let (tmp, svc) = open_svc();
        let tablet_id = seed_current_weekly_tablet(tmp.path());
        let item_id = seed_priority_candidate_item(tmp.path(), &tablet_id);
        let tool = WeeklyRejectPriorityTool::new(Arc::clone(&svc));
        let out = tool
            .execute(&ctx(), json!({ "item_id": item_id }))
            .await
            .unwrap();
        assert!(!out.is_error, "got error: {}", out.content);
        assert!(out.content.contains("\"status\":\"rejected\""));
        let conn = Connection::open(tmp.path().join("test.db")).unwrap();
        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ceremony_items WHERE id = ?1",
                params![&item_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 0);
    }

    #[tokio::test]
    async fn weekly_add_priority_requires_tablet_and_body() {
        let (_tmp, svc) = open_svc();
        let tool = WeeklyAddPriorityTool::new(svc);
        let out = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("tablet_id"));
        let (_tmp2, svc2) = open_svc();
        let tool2 = WeeklyAddPriorityTool::new(svc2);
        let out2 = tool2
            .execute(&ctx(), json!({ "tablet_id": "t1" }))
            .await
            .unwrap();
        assert!(out2.is_error);
        assert!(out2.content.contains("body"));
    }

    #[tokio::test]
    async fn weekly_add_priority_inserts_row() {
        let (tmp, svc) = open_svc();
        let tablet_id = seed_current_weekly_tablet(tmp.path());
        let tool = WeeklyAddPriorityTool::new(Arc::clone(&svc));
        let out = tool
            .execute(
                &ctx(),
                json!({
                    "tablet_id": tablet_id,
                    "body": "land the weekly tools",
                    "rationale": "blocks I-0042",
                }),
            )
            .await
            .unwrap();
        assert!(!out.is_error, "got error: {}", out.content);
        assert!(out.content.contains("\"status\":\"added\""));
        assert!(out.content.contains("priority_id"));
        let conn = Connection::open(tmp.path().join("test.db")).unwrap();
        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ceremony_priorities cp \
                 JOIN todos t ON t.id = cp.todo_id \
                 WHERE cp.tablet_id = ?1 \
                   AND json_extract(t.attrs, '$.citation_id') IS NULL",
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
            Box::new(WeeklyRunTool::new(Arc::clone(&svc))),
            Box::new(WeeklyCurrentTool::new(Arc::clone(&svc))),
            Box::new(WeeklyListItemsTool::new(Arc::clone(&svc))),
            Box::new(WeeklyListPrioritiesTool::new(Arc::clone(&svc))),
            Box::new(WeeklyConfirmPriorityTool::new(Arc::clone(&svc))),
            Box::new(WeeklyRejectPriorityTool::new(Arc::clone(&svc))),
            Box::new(WeeklyAddPriorityTool::new(Arc::clone(&svc))),
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

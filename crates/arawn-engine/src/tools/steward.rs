//! `/workstream journal`, `/workstream refine`, `/workstream rollback`
//! — agent-facing surface over the steward's journal. Phase 5 of I-0040
//! (T-0259).
//!
//! All three operate on the active workstream by default and accept an
//! optional `workstream` arg to target a named one. Rollback is the
//! only one that mutates state; it dispatches per-subroutine inverse
//! via `arawn_steward::rollback::apply_inverse`.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_steward::{Journal, rollback};

use crate::tool::{Tool, ToolCategory, ToolError, ToolOutput};
use crate::workstream_router::{MemoryHandle, WorkstreamMemoryRouter};

fn open_journal(data_dir: &PathBuf, workstream: &str) -> Result<Journal, ToolError> {
    Journal::open(data_dir, workstream)
        .map_err(|e| ToolError::ExecutionFailed(format!("open journal `{workstream}`: {e}")))
}

fn resolve_workstream(
    memory: &MemoryHandle,
    explicit: Option<&str>,
) -> Result<String, ToolError> {
    if let Some(name) = explicit {
        return Ok(name.to_string());
    }
    match memory {
        MemoryHandle::Routed(r) => Ok(r.current()
            .map_err(|e| ToolError::ExecutionFailed(format!("memory routing: {e}")))?
            .embedder()
            .map(|_| String::new()) // unused; we only want the workstream name through the session
            .unwrap_or_default()),
        MemoryHandle::Fixed(_) => Err(ToolError::ExecutionFailed(
            "workstream arg required when memory handle is fixed".into(),
        )),
    }
}

/// Lightweight summary of one journal row for tool output.
fn row_summary(row: &arawn_steward::JournalRow) -> Value {
    json!({
        "id": row.id,
        "ts": row.ts.to_rfc3339(),
        "subroutine": row.subroutine,
        "action": row.action,
        "applied": row.applied,
        "reverted_at": row.reverted_at.map(|t| t.to_rfc3339()),
        "model": row.model,
        "inputs": serde_json::from_str::<Value>(&row.inputs_json).unwrap_or(Value::Null),
        "outputs": serde_json::from_str::<Value>(&row.outputs_json).unwrap_or(Value::Null),
    })
}

// ─────────────────────────────────────────────────────────────────────────
// /workstream journal
// ─────────────────────────────────────────────────────────────────────────

pub struct WorkstreamJournalTool {
    data_dir: PathBuf,
    router: Arc<WorkstreamMemoryRouter>,
}

impl WorkstreamJournalTool {
    pub fn new(data_dir: impl Into<PathBuf>, router: Arc<WorkstreamMemoryRouter>) -> Self {
        Self {
            data_dir: data_dir.into(),
            router,
        }
    }
}

#[async_trait]
impl Tool for WorkstreamJournalTool {
    fn name(&self) -> &str {
        "workstream_journal"
    }

    fn description(&self) -> &str {
        "List recent steward actions for the active workstream (or one passed via `workstream`). \
         Shows merges, deletes, and pending proposals with enough payload to inspect what the \
         steward did."
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
                "workstream": { "type": "string" },
                "limit": { "type": "integer", "description": "Default 20, max 200" }
            }
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let workstream = match params.get("workstream").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => self.router.current_name(),
        };
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20)
            .min(200) as usize;
        let j = open_journal(&self.data_dir, &workstream)?;
        let rows = j
            .recent(limit)
            .map_err(|e| ToolError::ExecutionFailed(format!("journal recent: {e}")))?;
        let payload = json!({
            "workstream": workstream,
            "count": rows.len(),
            "rows": rows.iter().map(row_summary).collect::<Vec<_>>(),
        });
        Ok(ToolOutput::success(payload.to_string()))
    }
}

// ─────────────────────────────────────────────────────────────────────────
// /workstream refine — pending proposals only
// ─────────────────────────────────────────────────────────────────────────

pub struct WorkstreamRefineTool {
    data_dir: PathBuf,
    router: Arc<WorkstreamMemoryRouter>,
}

impl WorkstreamRefineTool {
    pub fn new(data_dir: impl Into<PathBuf>, router: Arc<WorkstreamMemoryRouter>) -> Self {
        Self {
            data_dir: data_dir.into(),
            router,
        }
    }
}

#[async_trait]
impl Tool for WorkstreamRefineTool {
    fn name(&self) -> &str {
        "workstream_refine"
    }

    fn description(&self) -> &str {
        "List pending steward proposals (map + door-watch) for the active workstream. \
         Proposals are not applied automatically — the user reviews them. Reject via \
         `workstream_rollback <id>`. Accept/apply is a future v2."
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
                "workstream": { "type": "string" },
                "limit": { "type": "integer", "description": "Default 20, max 200" }
            }
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let workstream = match params.get("workstream").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => self.router.current_name(),
        };
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20)
            .min(200) as usize;
        let j = open_journal(&self.data_dir, &workstream)?;
        let rows = j
            .pending_proposals(limit)
            .map_err(|e| ToolError::ExecutionFailed(format!("journal pending: {e}")))?;
        let payload = json!({
            "workstream": workstream,
            "count": rows.len(),
            "proposals": rows.iter().map(row_summary).collect::<Vec<_>>(),
        });
        Ok(ToolOutput::success(payload.to_string()))
    }
}

// ─────────────────────────────────────────────────────────────────────────
// /workstream rollback ID
// ─────────────────────────────────────────────────────────────────────────

pub struct WorkstreamRollbackTool {
    data_dir: PathBuf,
    router: Arc<WorkstreamMemoryRouter>,
}

impl WorkstreamRollbackTool {
    pub fn new(data_dir: impl Into<PathBuf>, router: Arc<WorkstreamMemoryRouter>) -> Self {
        Self {
            data_dir: data_dir.into(),
            router,
        }
    }
}

#[async_trait]
impl Tool for WorkstreamRollbackTool {
    fn name(&self) -> &str {
        "workstream_rollback"
    }

    fn description(&self) -> &str {
        "Revert one steward action by journal id. For reshelve merges/deletes the inverse \
         mutation is applied to the KB; for map/door-watch proposals the rollback is a \
         metadata flip. Idempotent. Returns a confirmation by id."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Memory
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "integer", "description": "Journal row id" },
                "workstream": { "type": "string" }
            },
            "required": ["id"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let id = params
            .get("id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'id'".into()))?;
        let workstream = match params.get("workstream").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => self.router.current_name(),
        };
        let j = open_journal(&self.data_dir, &workstream)?;
        let row = j
            .get(id)
            .map_err(|e| ToolError::ExecutionFailed(format!("journal get: {e}")))?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("no journal row {id}")))?;
        if row.reverted_at.is_some() {
            return Ok(ToolOutput::success(
                json!({"id": id, "status": "already_reverted"}).to_string(),
            ));
        }
        // Apply the per-subroutine inverse mutation against the
        // workstream's KB, then flip the metadata.
        let kb = self
            .router
            .for_workstream(&workstream)
            .map_err(|e| ToolError::ExecutionFailed(format!("memory routing: {e}")))?;
        rollback::apply_inverse(&row, &kb)
            .map_err(|e| ToolError::ExecutionFailed(format!("rollback: {e}")))?;
        let _ = j
            .revert(id)
            .map_err(|e| ToolError::ExecutionFailed(format!("journal revert: {e}")))?;
        Ok(ToolOutput::success(
            json!({"id": id, "status": "reverted"}).to_string(),
        ))
    }
}

// Suppress unused-import warning while resolve_workstream stays for
// future callers — currently each tool inlines its routing.
#[allow(dead_code)]
fn _unused(memory: &MemoryHandle, explicit: Option<&str>) -> Result<String, ToolError> {
    resolve_workstream(memory, explicit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Workstream;
    use arawn_memory::{Entity, EntityType};
    use arawn_steward::{Journal, JournalRecord};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn setup() -> (
        TempDir,
        Arc<WorkstreamMemoryRouter>,
        crate::context::EngineToolContext,
    ) {
        let tmp = TempDir::new().unwrap();
        let session = crate::tools::SessionWorkstream::new("ws-pat");
        let router = Arc::new(WorkstreamMemoryRouter::new(
            tmp.path(),
            None,
            None,
            session,
        ));
        let ws = Workstream::scratch(tmp.path());
        let ctx = crate::context::EngineToolContext::new(&ws, Uuid::new_v4());
        (tmp, router, ctx)
    }

    fn write_proposal_row(j: &Journal) -> i64 {
        let rec = JournalRecord {
            subroutine: "map".into(),
            action: "propose_relation".into(),
            inputs_json: "{}".into(),
            outputs_json: json!({"from_id": Uuid::new_v4(), "rel": "relates_to", "to_id": Uuid::new_v4()})
                .to_string(),
            model: "test".into(),
            prompt_hash: "h".into(),
            applied: false,
        };
        j.write_ahead(&rec).unwrap()
    }

    fn write_delete_row(j: &Journal, e: &Entity) -> i64 {
        let rec = JournalRecord {
            subroutine: "reshelve".into(),
            action: "delete".into(),
            inputs_json: "{}".into(),
            outputs_json: json!({"entity": e}).to_string(),
            model: "test".into(),
            prompt_hash: "h".into(),
            applied: true,
        };
        j.write_ahead(&rec).unwrap()
    }

    #[tokio::test]
    async fn journal_lists_recent_rows() {
        let (tmp, router, ctx) = setup();
        let j = Journal::open(tmp.path(), "ws-pat").unwrap();
        let _ = write_proposal_row(&j);
        let _ = write_proposal_row(&j);
        let tool = WorkstreamJournalTool::new(tmp.path(), Arc::clone(&router));
        let r = tool.execute(&ctx, json!({})).await.unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["count"], 2);
        assert_eq!(v["workstream"], "ws-pat");
    }

    #[tokio::test]
    async fn refine_returns_pending_proposals_only() {
        let (tmp, router, ctx) = setup();
        let j = Journal::open(tmp.path(), "ws-pat").unwrap();
        let _pid = write_proposal_row(&j);
        // Add an applied row that should NOT appear.
        let rec = JournalRecord {
            subroutine: "reshelve".into(),
            action: "delete".into(),
            inputs_json: "{}".into(),
            outputs_json: json!({"entity": Entity::new(EntityType::Fact, "x")}).to_string(),
            model: "test".into(),
            prompt_hash: "h".into(),
            applied: true,
        };
        j.write_ahead(&rec).unwrap();

        let tool = WorkstreamRefineTool::new(tmp.path(), Arc::clone(&router));
        let r = tool.execute(&ctx, json!({})).await.unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["count"], 1);
        assert_eq!(v["proposals"][0]["action"], "propose_relation");
    }

    #[tokio::test]
    async fn rollback_reverts_delete_action_end_to_end() {
        let (tmp, router, ctx) = setup();
        // Open the KB through the router so the entity lives in the
        // same db the tool will reach.
        let kb = router.for_workstream("ws-pat").unwrap();
        let e = Entity::new(EntityType::Fact, "important fact").with_content("v1");
        // Pretend reshelve already deleted this entity — we journal the
        // delete with the full snapshot and remove it from the KB.
        let j = Journal::open(tmp.path(), "ws-pat").unwrap();
        let id = write_delete_row(&j, &e);

        let tool = WorkstreamRollbackTool::new(tmp.path(), Arc::clone(&router));
        let r = tool
            .execute(&ctx, json!({"id": id}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["status"], "reverted");
        // Entity restored
        let restored = kb.workstream.get_entity(e.id).unwrap().unwrap();
        assert_eq!(restored.title, "important fact");
    }

    #[tokio::test]
    async fn rollback_is_idempotent() {
        let (tmp, router, ctx) = setup();
        let j = Journal::open(tmp.path(), "ws-pat").unwrap();
        let id = write_proposal_row(&j);
        let tool = WorkstreamRollbackTool::new(tmp.path(), Arc::clone(&router));
        let r1: Value =
            serde_json::from_str(&tool.execute(&ctx, json!({"id": id})).await.unwrap().content)
                .unwrap();
        assert_eq!(r1["status"], "reverted");
        let r2: Value =
            serde_json::from_str(&tool.execute(&ctx, json!({"id": id})).await.unwrap().content)
                .unwrap();
        assert_eq!(r2["status"], "already_reverted");
    }

    #[tokio::test]
    async fn rollback_unknown_id_errors() {
        let (tmp, router, ctx) = setup();
        let tool = WorkstreamRollbackTool::new(tmp.path(), Arc::clone(&router));
        let r = tool.execute(&ctx, json!({"id": 9999})).await;
        assert!(r.is_err());
    }
}

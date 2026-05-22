use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_storage::Store;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

use super::SessionWorkstream;

pub struct WorkstreamSwitchTool {
    store: Arc<Mutex<Store>>,
    active: SessionWorkstream,
}

impl WorkstreamSwitchTool {
    pub fn new(store: Arc<Mutex<Store>>, active: SessionWorkstream) -> Self {
        Self { store, active }
    }
}

#[async_trait]
impl Tool for WorkstreamSwitchTool {
    fn name(&self) -> &str {
        "workstream_switch"
    }

    fn description(&self) -> &str {
        "Switch the session-active workstream. Subsequent memory operations \
         in this session route to that workstream's KB + global. Errors if \
         the named workstream doesn't exist."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "name": {"type": "string"} },
            "required": ["name"]
        })
    }

    async fn execute(
        &self,
        ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("name is required".to_string())),
        };
        let store = self.store.lock().unwrap();
        let ws = store
            .find_workstream_by_name(&name)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let Some(ws) = ws else {
            return Ok(ToolOutput::error(format!("workstream '{name}' not found")));
        };
        if ws.archived {
            return Ok(ToolOutput::error(format!(
                "workstream '{name}' is archived; un-archive it first"
            )));
        }
        let prev = self.active.current();
        // Update the in-memory shim first so any tool call running in
        // the same turn sees the new active workstream.
        self.active.set(&ws.name);
        // Persist on the session record too — `LocalService` re-reads
        // `meta.workstream_name` on every session-load (i.e. every
        // turn over WS) and pushes it back into the SessionWorkstream
        // shim. Without this write, the in-memory set above would be
        // overwritten by the stale persisted value on the next turn
        // and the switch would silently revert.
        let session_id = ctx.session_id();
        if let Err(e) = store.update_session_workstream_name(session_id, &ws.name) {
            tracing::warn!(
                error = %e,
                session = %session_id,
                workstream = %ws.name,
                "workstream_switch: failed to persist on session — switch is in-memory only, will revert on next turn"
            );
        }
        // Lead with the human-readable banner so the TUI / agent
        // surfaces the switch clearly; structured fields trail.
        let banner = format!(
            "→ now in workstream '{}' — next messages contribute to {}'s KB (was: {})",
            ws.name, ws.name, prev
        );
        Ok(ToolOutput::success(format!(
            "{banner}\n{}",
            json!({
                "switched_to": ws.name,
                "previous": prev,
            })
        )))
    }
}


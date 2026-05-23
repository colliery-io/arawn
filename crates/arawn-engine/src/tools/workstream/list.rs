use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_storage::Store;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

use super::SessionWorkstream;

pub struct WorkstreamListTool {
    store: Arc<Mutex<Store>>,
    active: SessionWorkstream,
}

impl WorkstreamListTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self {
            store,
            active: SessionWorkstream::default(),
        }
    }

    pub fn with_active(mut self, active: SessionWorkstream) -> Self {
        self.active = active;
        self
    }
}

#[async_trait]
impl Tool for WorkstreamListTool {
    fn name(&self) -> &str {
        "workstream_list"
    }

    fn description(&self) -> &str {
        "List active workstreams (newest update first). Pass `all: true` to include archived."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "all": {"type": "boolean", "description": "Include archived (soft-deleted) workstreams"}
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let include_archived = params.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
        let active = self.active.current();
        let store = self.store.lock().unwrap();
        let workstreams = if include_archived {
            store.list_all_workstreams()
        } else {
            store.list_workstreams()
        }
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let items: Vec<Value> = workstreams
            .iter()
            .map(|ws| {
                json!({
                    "name": ws.name,
                    "display_name": ws.display_name,
                    "description": ws.description,
                    "bindings": ws.bindings,
                    "archived": ws.archived,
                    "active": ws.name == active,
                })
            })
            .collect();

        Ok(ToolOutput::success(
            json!({ "active": active, "workstreams": items }).to_string(),
        ))
    }
}


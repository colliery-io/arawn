use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_storage::Store;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

use super::SessionWorkstream;

pub struct WorkstreamDeleteTool {
    store: Arc<Mutex<Store>>,
    active: SessionWorkstream,
}

impl WorkstreamDeleteTool {
    pub fn new(store: Arc<Mutex<Store>>, active: SessionWorkstream) -> Self {
        Self { store, active }
    }
}

#[async_trait]
impl Tool for WorkstreamDeleteTool {
    fn name(&self) -> &str {
        "workstream_delete"
    }

    fn description(&self) -> &str {
        "Soft-delete a workstream (sets archived = 1). On-disk KB is left intact. \
         Refuses 'scratch' and refuses the currently-active workstream."
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
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("name is required".to_string())),
        };
        if name == self.active.current() {
            return Ok(ToolOutput::error(format!(
                "workstream '{name}' is currently active; switch away before deleting"
            )));
        }
        let store = self.store.lock().unwrap();
        match store.soft_delete_workstream(&name) {
            Ok(()) => Ok(ToolOutput::success(
                json!({
                    "deleted": name,
                    "note": "soft delete; on-disk KB intact"
                })
                .to_string(),
            )),
            Err(e) => Ok(ToolOutput::error(format!("failed: {e}"))),
        }
    }
}


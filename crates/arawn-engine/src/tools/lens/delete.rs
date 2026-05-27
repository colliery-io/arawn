use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_storage::Store;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

use super::SessionLens;

pub struct LensDeleteTool {
    store: Arc<Mutex<Store>>,
    active: SessionLens,
}

impl LensDeleteTool {
    pub fn new(store: Arc<Mutex<Store>>, active: SessionLens) -> Self {
        Self { store, active }
    }
}

#[async_trait]
impl Tool for LensDeleteTool {
    fn name(&self) -> &str {
        "lens_delete"
    }

    fn description(&self) -> &str {
        "Soft-delete a lens (sets archived = 1). On-disk KB is left intact. \
         Refuses 'scratch' and refuses the currently-active lens."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Lens
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
                "lens '{name}' is currently active; switch away before deleting"
            )));
        }
        let store = self.store.lock().unwrap();
        match store.soft_delete_lens(&name) {
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

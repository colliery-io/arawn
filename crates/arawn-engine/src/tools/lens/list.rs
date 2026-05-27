use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_storage::Store;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

use super::SessionLens;

pub struct LensListTool {
    store: Arc<Mutex<Store>>,
    active: SessionLens,
}

impl LensListTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self {
            store,
            active: SessionLens::default(),
        }
    }

    pub fn with_active(mut self, active: SessionLens) -> Self {
        self.active = active;
        self
    }
}

#[async_trait]
impl Tool for LensListTool {
    fn name(&self) -> &str {
        "lens_list"
    }

    fn description(&self) -> &str {
        "List active lenses (newest update first). Pass `all: true` to include archived."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Lens
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "all": {"type": "boolean", "description": "Include archived (soft-deleted) lenses"}
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
        let lenses = if include_archived {
            store.list_all_lenses()
        } else {
            store.list_lenses()
        }
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let items: Vec<Value> = lenses
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
            json!({ "active": active, "lenses": items }).to_string(),
        ))
    }
}

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_storage::Store;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

pub struct LensDescribeTool {
    store: Arc<Mutex<Store>>,
}

impl LensDescribeTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for LensDescribeTool {
    fn name(&self) -> &str {
        "lens_describe"
    }

    fn description(&self) -> &str {
        "Set or update a lens's description. The description feeds the \
         per-lens extractor in Phase 4."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Lens
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "description": {"type": "string"}
            },
            "required": ["name", "description"]
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
        let description = params
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let store = self.store.lock().unwrap();
        match store.update_lens_description(&name, &description) {
            Ok(()) => Ok(ToolOutput::success(
                json!({"name": name, "description": description}).to_string(),
            )),
            Err(e) => Ok(ToolOutput::error(format!("failed: {e}"))),
        }
    }
}

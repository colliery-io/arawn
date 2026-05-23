use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_storage::Store;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

use super::SessionWorkstream;

pub struct WorkstreamShowTool {
    store: Arc<Mutex<Store>>,
    active: SessionWorkstream,
}

impl WorkstreamShowTool {
    pub fn new(store: Arc<Mutex<Store>>, active: SessionWorkstream) -> Self {
        Self { store, active }
    }
}

#[async_trait]
impl Tool for WorkstreamShowTool {
    fn name(&self) -> &str {
        "workstream_show"
    }

    fn description(&self) -> &str {
        "Show the active workstream's details (or a named one). \
         Includes display_name, description, bindings, and — crucially \
         for downstream tools — the workstream's declared tag ontology. \
         Use this before calling `workstream_dust` or `signal_query` \
         with a tag filter so you pick a tag that actually exists in \
         the ontology."
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
                "name": {"type": "string", "description": "Defaults to the active workstream"}
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| self.active.current());
        let store = self.store.lock().unwrap();
        let ws = store
            .find_workstream_by_name(&name)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let Some(ws) = ws else {
            return Ok(ToolOutput::error(format!("workstream '{name}' not found")));
        };
        drop(store);

        // Surface the workstream's declared tag ontology — agents that
        // call this tool before `workstream_dust` / `signal_query` can
        // pick valid tags instead of guessing. Soft-fail to empty when
        // the data dir is unavailable or the ontology table is missing.
        let ontology_tags: Vec<String> = match ctx.data_dir() {
            Some(dir) => arawn_memory::TagOntologyStore::open(dir, &ws.name)
                .and_then(|s| s.tags())
                .unwrap_or_default(),
            None => Vec::new(),
        };

        Ok(ToolOutput::success(
            json!({
                "name": ws.name,
                "display_name": ws.display_name,
                "description": ws.description,
                "bindings": ws.bindings,
                "archived": ws.archived,
                "created_at": ws.created_at.to_rfc3339(),
                "updated_at": ws.updated_at.to_rfc3339(),
                "active": ws.name == self.active.current(),
                "tags_ontology": ontology_tags,
            })
            .to_string(),
        ))
    }
}


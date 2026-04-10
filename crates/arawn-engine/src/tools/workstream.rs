use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::context::ToolContext;
use crate::error::EngineError;
use crate::tool::{Tool, ToolCategory, ToolOutput};

use arawn_core::Workstream;
use arawn_storage::Store;

/// Tool for creating a new workstream.
pub struct WorkstreamCreateTool {
    store: Arc<Mutex<Store>>,
}

impl WorkstreamCreateTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for WorkstreamCreateTool {
    fn name(&self) -> &str {
        "workstream_create"
    }

    fn description(&self) -> &str {
        "Create a new workstream — an isolated context for a project or topic. \
         Each workstream gets its own sessions, sandbox directory, and scoped memory. \
         Use this when the user wants to organize work into a separate area."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name for the workstream (e.g., 'finances', 'arawn-dev', 'home-maintenance')"
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if name.is_empty() {
            return Ok(ToolOutput::error("name is required".to_string()));
        }

        let store = self.store.lock().unwrap();

        // Check if it already exists
        if let Ok(Some(_)) = store.find_workstream_by_name(&name) {
            return Ok(ToolOutput::error(format!("workstream '{name}' already exists")));
        }

        // Default root_dir to data_dir/workstreams/{name}/
        let root_dir = ctx.data_dir().map(|d| d.join("workstreams").join(&name))
            .unwrap_or_else(|| std::path::PathBuf::from(&name));

        let ws = Workstream::new(&name, &root_dir);
        match store.create_workstream(&ws) {
            Ok(()) => {
                let result = json!({
                    "id": ws.id.to_string(),
                    "name": ws.name,
                    "root_dir": ws.root_dir.display().to_string(),
                });
                Ok(ToolOutput::success(result.to_string()))
            }
            Err(e) => Ok(ToolOutput::error(format!("failed to create workstream: {e}"))),
        }
    }
}

/// Tool for listing available workstreams.
pub struct WorkstreamListTool {
    store: Arc<Mutex<Store>>,
}

impl WorkstreamListTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for WorkstreamListTool {
    fn name(&self) -> &str {
        "workstream_list"
    }

    fn description(&self) -> &str {
        "List all available workstreams with their names, IDs, and session counts."
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
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _ctx: &ToolContext, _params: Value) -> Result<ToolOutput, EngineError> {
        let store = self.store.lock().unwrap();
        let workstreams = store.list_workstreams()
            .map_err(|e| EngineError::Tool(e.to_string()))?;

        let items: Vec<Value> = workstreams
            .iter()
            .map(|ws| {
                let session_count = store
                    .list_sessions_for_workstream(ws.id)
                    .map(|s| s.len())
                    .unwrap_or(0);
                json!({
                    "id": ws.id.to_string(),
                    "name": ws.name,
                    "session_count": session_count,
                })
            })
            .collect();

        Ok(ToolOutput::success(json!(items).to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn setup() -> (tempfile::TempDir, Arc<Mutex<Store>>) {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        let ws = Workstream::scratch(tmp.path());
        store.create_workstream(&ws).unwrap();
        (tmp, Arc::new(Mutex::new(store)))
    }

    fn test_ctx(tmp: &tempfile::TempDir) -> ToolContext {
        let ws = Workstream::scratch(tmp.path());
        ToolContext::new(&ws, Uuid::new_v4())
            .with_data_dir(tmp.path().to_path_buf())
    }

    #[tokio::test]
    async fn create_workstream_succeeds() {
        let (tmp, store) = setup();
        let tool = WorkstreamCreateTool::new(store.clone());
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "test-project"}))
            .await
            .unwrap();
        assert!(!result.is_error, "expected success, got: {}", result.content);
        assert!(result.content.contains("test-project"));
    }

    #[tokio::test]
    async fn create_duplicate_workstream_errors() {
        let (tmp, store) = setup();
        let tool = WorkstreamCreateTool::new(store.clone());
        // scratch already exists
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "scratch"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("already exists"));
    }

    #[tokio::test]
    async fn create_workstream_empty_name_errors() {
        let (tmp, store) = setup();
        let tool = WorkstreamCreateTool::new(store.clone());
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": ""}))
            .await
            .unwrap();
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn list_workstreams_includes_scratch() {
        let (tmp, store) = setup();
        let tool = WorkstreamListTool::new(store.clone());
        let result = tool
            .execute(&test_ctx(&tmp), json!({}))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("scratch"));
    }
}

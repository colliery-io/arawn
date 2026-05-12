//! Workstream slash commands.
//!
//! Eight tools that cover the workstream lifecycle: new, list, switch,
//! show, describe, bind, unbind, delete. Each is a `Tool` impl with a
//! `workstream_*` name so the slash dispatcher routes naturally.
//!
//! Session-active workstream is held by `SessionWorkstream` (a shared
//! `Arc<Mutex<String>>`) — T-0250 replaces this with the real
//! `Session::workstream_name` field once sessions gain it. For now
//! the shim is enough to make `switch` / `show` work.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_core::{SCRATCH_NAME, Workstream};
use arawn_storage::Store;

use crate::tool::{Tool, ToolCategory, ToolError, ToolOutput};

/// Holder for the session-active workstream name. Cheap to clone
/// (`Arc<Mutex<String>>`). T-0250 will retire this in favor of the
/// `Session::workstream_name` field.
#[derive(Clone, Debug)]
pub struct SessionWorkstream {
    inner: Arc<Mutex<String>>,
}

impl SessionWorkstream {
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(initial.into())),
        }
    }

    pub fn scratch() -> Self {
        Self::new(SCRATCH_NAME)
    }

    pub fn current(&self) -> String {
        self.inner.lock().unwrap().clone()
    }

    pub fn set(&self, name: impl Into<String>) {
        *self.inner.lock().unwrap() = name.into();
    }
}

impl Default for SessionWorkstream {
    fn default() -> Self {
        Self::scratch()
    }
}

// ============================================================================
// workstream_new
// ============================================================================

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
        "workstream_new"
    }

    fn description(&self) -> &str {
        "Create a new workstream — an isolated scope for one thing you track \
         (a person, a project, a hobby). Name must be a slug \
         (lowercase, digits, '-' and '_' only). Does not switch into it."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Slug name (e.g. 'pat', 'auth-migration')"},
                "display_name": {"type": "string", "description": "Optional human label (defaults to name)"},
                "description": {"type": "string", "description": "Optional free-text description"}
            },
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
        if name == SCRATCH_NAME {
            return Ok(ToolOutput::error(
                "the name 'scratch' is reserved — it always exists".to_string(),
            ));
        }
        let display_name = params
            .get("display_name")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| name.clone());
        let description = params
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let root_dir = ctx
            .data_dir()
            .map(|d| d.join("workstreams").join(&name))
            .unwrap_or_else(|| std::path::PathBuf::from(&name));

        let mut ws = Workstream::new(&name, &root_dir);
        ws.display_name = display_name;
        ws.description = description;

        let store = self.store.lock().unwrap();
        match store.create_workstream(&ws) {
            Ok(()) => Ok(ToolOutput::success(
                json!({
                    "name": ws.name,
                    "display_name": ws.display_name,
                    "root_dir": ws.root_dir.display().to_string(),
                })
                .to_string(),
            )),
            Err(e) => Ok(ToolOutput::error(format!(
                "failed to create workstream: {e}"
            ))),
        }
    }
}

// ============================================================================
// workstream_list
// ============================================================================

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
        let include_archived = params
            .get("all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
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

// ============================================================================
// workstream_switch
// ============================================================================

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
        _ctx: &dyn arawn_tool::ToolContext,
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
            return Ok(ToolOutput::error(format!(
                "workstream '{name}' not found"
            )));
        };
        if ws.archived {
            return Ok(ToolOutput::error(format!(
                "workstream '{name}' is archived; un-archive it first"
            )));
        }
        let prev = self.active.current();
        self.active.set(&ws.name);
        Ok(ToolOutput::success(
            json!({
                "switched_to": ws.name,
                "previous": prev,
                "banner": format!(
                    "now in workstream '{}' — next messages contribute to {}'s KB",
                    ws.name, ws.name
                ),
            })
            .to_string(),
        ))
    }
}

// ============================================================================
// workstream_show
// ============================================================================

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
         Includes display_name, description, bindings."
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
        _ctx: &dyn arawn_tool::ToolContext,
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
            return Ok(ToolOutput::error(format!(
                "workstream '{name}' not found"
            )));
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
            })
            .to_string(),
        ))
    }
}

// ============================================================================
// workstream_describe
// ============================================================================

pub struct WorkstreamDescribeTool {
    store: Arc<Mutex<Store>>,
}

impl WorkstreamDescribeTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for WorkstreamDescribeTool {
    fn name(&self) -> &str {
        "workstream_describe"
    }

    fn description(&self) -> &str {
        "Set or update a workstream's description. The description feeds the \
         per-workstream extractor in Phase 4."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
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
        match store.update_workstream_description(&name, &description) {
            Ok(()) => Ok(ToolOutput::success(
                json!({"name": name, "description": description}).to_string(),
            )),
            Err(e) => Ok(ToolOutput::error(format!("failed: {e}"))),
        }
    }
}

// ============================================================================
// workstream_bind / workstream_unbind
// ============================================================================

pub struct WorkstreamBindTool {
    store: Arc<Mutex<Store>>,
}

impl WorkstreamBindTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for WorkstreamBindTool {
    fn name(&self) -> &str {
        "workstream_bind"
    }

    fn description(&self) -> &str {
        "Bind a feed to a workstream. Bindings hint to the Phase 4 extractor \
         which feed items should land in this workstream's KB. Idempotent."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "feed_id": {"type": "string"}
            },
            "required": ["name", "feed_id"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let feed_id = params
            .get("feed_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if name.is_empty() || feed_id.is_empty() {
            return Ok(ToolOutput::error("name and feed_id are required".to_string()));
        }
        let store = self.store.lock().unwrap();
        match store.add_workstream_binding(&name, &feed_id) {
            Ok(()) => Ok(ToolOutput::success(
                json!({"name": name, "feed_id": feed_id}).to_string(),
            )),
            Err(e) => Ok(ToolOutput::error(format!("failed: {e}"))),
        }
    }
}

pub struct WorkstreamUnbindTool {
    store: Arc<Mutex<Store>>,
}

impl WorkstreamUnbindTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for WorkstreamUnbindTool {
    fn name(&self) -> &str {
        "workstream_unbind"
    }

    fn description(&self) -> &str {
        "Remove a feed binding from a workstream. Silent no-op if not bound."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Workstream
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "feed_id": {"type": "string"}
            },
            "required": ["name", "feed_id"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let feed_id = params
            .get("feed_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if name.is_empty() || feed_id.is_empty() {
            return Ok(ToolOutput::error("name and feed_id are required".to_string()));
        }
        let store = self.store.lock().unwrap();
        match store.remove_workstream_binding(&name, &feed_id) {
            Ok(()) => Ok(ToolOutput::success(
                json!({"name": name, "feed_id": feed_id}).to_string(),
            )),
            Err(e) => Ok(ToolOutput::error(format!("failed: {e}"))),
        }
    }
}

// ============================================================================
// workstream_delete (soft)
// ============================================================================

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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn setup() -> (tempfile::TempDir, Arc<Mutex<Store>>, SessionWorkstream) {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        store.ensure_scratch_workstream().unwrap();
        (tmp, Arc::new(Mutex::new(store)), SessionWorkstream::scratch())
    }

    fn test_ctx(tmp: &tempfile::TempDir) -> crate::context::EngineToolContext {
        let ws = Workstream::scratch(tmp.path());
        crate::context::EngineToolContext::new(&ws, Uuid::new_v4())
            .with_data_dir(tmp.path().to_path_buf())
    }

    #[tokio::test]
    async fn create_succeeds_with_valid_slug() {
        let (tmp, store, _) = setup();
        let tool = WorkstreamCreateTool::new(store.clone());
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "pat"}))
            .await
            .unwrap();
        assert!(!result.is_error, "got: {}", result.content);
        assert!(result.content.contains("pat"));
    }

    #[tokio::test]
    async fn create_refuses_scratch() {
        let (tmp, store, _) = setup();
        let tool = WorkstreamCreateTool::new(store.clone());
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "scratch"}))
            .await
            .unwrap();
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn switch_updates_active() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("workstreams/pat")))
            .unwrap();
        let tool = WorkstreamSwitchTool::new(store.clone(), active.clone());
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "pat"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert_eq!(active.current(), "pat");
    }

    #[tokio::test]
    async fn switch_unknown_errors() {
        let (tmp, store, active) = setup();
        let tool = WorkstreamSwitchTool::new(store.clone(), active);
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "ghost"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("not found"));
    }

    #[tokio::test]
    async fn show_defaults_to_active() {
        let (tmp, store, active) = setup();
        let tool = WorkstreamShowTool::new(store.clone(), active);
        let result = tool.execute(&test_ctx(&tmp), json!({})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("scratch"));
    }

    #[tokio::test]
    async fn describe_updates_description() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        let tool = WorkstreamDescribeTool::new(store.clone());
        let result = tool
            .execute(
                &test_ctx(&tmp),
                json!({"name": "pat", "description": "skip-level for pat"}),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let fetched = store
            .lock()
            .unwrap()
            .find_workstream_by_name("pat")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.description, "skip-level for pat");
    }

    #[tokio::test]
    async fn bind_and_unbind_round_trip() {
        let (tmp, store, _) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        let bind = WorkstreamBindTool::new(store.clone());
        bind.execute(&test_ctx(&tmp), json!({"name": "pat", "feed_id": "f1"}))
            .await
            .unwrap();
        let fetched = store
            .lock()
            .unwrap()
            .find_workstream_by_name("pat")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.bindings, vec!["f1"]);
        let unbind = WorkstreamUnbindTool::new(store.clone());
        unbind
            .execute(&test_ctx(&tmp), json!({"name": "pat", "feed_id": "f1"}))
            .await
            .unwrap();
        let fetched = store
            .lock()
            .unwrap()
            .find_workstream_by_name("pat")
            .unwrap()
            .unwrap();
        assert!(fetched.bindings.is_empty());
    }

    #[tokio::test]
    async fn delete_refuses_scratch() {
        let (tmp, store, active) = setup();
        let tool = WorkstreamDeleteTool::new(store.clone(), active);
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "scratch"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("active") || result.content.contains("scratch"));
    }

    #[tokio::test]
    async fn delete_refuses_currently_active() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        active.set("pat");
        let tool = WorkstreamDeleteTool::new(store.clone(), active);
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "pat"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("active"));
    }

    #[tokio::test]
    async fn delete_soft_marks_archived() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("temp", tmp.path().join("ws/temp")))
            .unwrap();
        let tool = WorkstreamDeleteTool::new(store.clone(), active);
        let result = tool
            .execute(&test_ctx(&tmp), json!({"name": "temp"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        // listed via list_all_workstreams should still show it as archived.
        let all = store.lock().unwrap().list_all_workstreams().unwrap();
        let found = all.iter().find(|w| w.name == "temp").unwrap();
        assert!(found.archived);
    }

    #[tokio::test]
    async fn list_marks_active() {
        let (tmp, store, active) = setup();
        store
            .lock()
            .unwrap()
            .create_workstream(&Workstream::new("pat", tmp.path().join("ws/pat")))
            .unwrap();
        active.set("pat");
        let tool = WorkstreamListTool::new(store.clone()).with_active(active);
        let result = tool.execute(&test_ctx(&tmp), json!({})).await.unwrap();
        assert!(!result.is_error);
        // Active workstream should be flagged.
        assert!(result.content.contains("\"active\":true"));
    }
}

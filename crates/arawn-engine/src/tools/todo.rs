//! Generic `todo_*` agent tools (I-0049 T-0313).
//!
//! First-class todo surface for the agent — usable anywhere in
//! chat, not just inside ceremony flows. Each tool wraps
//! [`arawn_storage::TodoService`] with the event sender wired so
//! mutations propagate onto the existing notice broadcast (the TUI
//! and any other subscriber sees `category="todo_event"` notices).
//!
//! For the higher-level ceremony tool family (`daily_add_todo`,
//! `weekly_confirm_priority`, `weekly_add_priority`), the existing
//! `CeremonyService` paths already route through the `todos` table
//! after T-0312's schema cutover, so they implicitly use the new
//! source of truth without further changes.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_storage::{ListFilter, NewTodo, Store, TodoEventSender, TodoPatch, TodoService};

use crate::tool::{Tool, ToolCategory, ToolError, ToolOutput};

fn build_service<'a>(
    store: &'a Store,
    events: &Option<TodoEventSender>,
) -> TodoService<'a> {
    let svc = TodoService::new(store.database());
    match events {
        Some(tx) => svc.with_events(tx.clone()),
        None => svc,
    }
}

fn map_err(e: arawn_storage::StorageError) -> ToolOutput {
    ToolOutput::error(format!("todo_error: {e}"))
}

fn json_or_null(v: serde_json::Result<String>) -> String {
    v.unwrap_or_else(|_| "null".into())
}

// ============================================================================
// todo_create
// ============================================================================

pub struct TodoCreateTool {
    store: Arc<Mutex<Store>>,
    events: Option<TodoEventSender>,
}

impl TodoCreateTool {
    pub fn new(store: Arc<Mutex<Store>>, events: Option<TodoEventSender>) -> Self {
        Self { store, events }
    }
}

#[async_trait]
impl Tool for TodoCreateTool {
    fn name(&self) -> &str {
        "todo_create"
    }

    fn description(&self) -> &str {
        "Create a new todo. Use this when the user says \"remind me \
         to ...\", \"I should ...\", or similar. `body` is required; \
         `kind` defaults to `\"user\"` (free-form text — future \
         kinds like `linear` or `github` will drop in here); \
         `workstream` is optional and defaults to NULL (global). \
         Returns the new todo row. Surfaces in `/todo` and via \
         `todo_list`."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "body": {"type": "string", "description": "The todo text."},
                "kind": {"type": "string", "description": "Discriminant; defaults to `user`."},
                "workstream": {"type": "string", "description": "Optional workstream scope."},
                "due_at": {"type": "string", "description": "Optional RFC3339 due date."},
                "rationale": {"type": "string", "description": "Optional why-line."}
            },
            "required": ["body"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let body = match params.get("body").and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("body is required")),
        };
        let kind = params
            .get("kind")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "user".into());
        let workstream = params
            .get("workstream")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let rationale = params
            .get("rationale")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let due_at = params
            .get("due_at")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&chrono::Utc));

        let req = NewTodo {
            id: None,
            body,
            rationale,
            kind,
            workstream,
            due_at,
            attrs: None,
        };
        let store = match self.store.lock() {
            Ok(s) => s,
            Err(_) => return Ok(ToolOutput::error("store lock poisoned")),
        };
        match build_service(&store, &self.events).create(req) {
            Ok(t) => Ok(ToolOutput::success(json_or_null(serde_json::to_string(&t)))),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// todo_list
// ============================================================================

pub struct TodoListTool {
    store: Arc<Mutex<Store>>,
    events: Option<TodoEventSender>,
}

impl TodoListTool {
    pub fn new(store: Arc<Mutex<Store>>, events: Option<TodoEventSender>) -> Self {
        Self { store, events }
    }
}

#[async_trait]
impl Tool for TodoListTool {
    fn name(&self) -> &str {
        "todo_list"
    }

    fn description(&self) -> &str {
        "List todos. All filters are optional and combine as AND: \
         `kind` matches the discriminant; `workstream` scopes to a \
         single workstream; `open_only: true` returns only \
         un-done. Defaults exclude archived rows. Returns an array \
         sorted by due_at NULLS LAST, then created_at desc."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "kind": {"type": "string"},
                "workstream": {"type": "string"},
                "open_only": {"type": "boolean"},
                "include_archived": {"type": "boolean"}
            },
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let filter = ListFilter {
            kind: params
                .get("kind")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            workstream: params
                .get("workstream")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            open_only: params.get("open_only").and_then(|v| v.as_bool()),
            due_from: None,
            due_to: None,
            include_archived: params.get("include_archived").and_then(|v| v.as_bool()),
        };
        let store = match self.store.lock() {
            Ok(s) => s,
            Err(_) => return Ok(ToolOutput::error("store lock poisoned")),
        };
        match build_service(&store, &self.events).list(filter) {
            Ok(rows) => Ok(ToolOutput::success(json_or_null(serde_json::to_string(
                &rows,
            )))),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// todo_get / todo_done / todo_undo / todo_archive — single-id ops
// ============================================================================

macro_rules! single_id_tool {
    ($struct_name:ident, $tool_name:literal, $desc:literal, $method:ident, $allow_none:expr) => {
        pub struct $struct_name {
            store: Arc<Mutex<Store>>,
            events: Option<TodoEventSender>,
        }

        impl $struct_name {
            pub fn new(store: Arc<Mutex<Store>>, events: Option<TodoEventSender>) -> Self {
                Self { store, events }
            }
        }

        #[async_trait]
        impl Tool for $struct_name {
            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> &str {
                $desc
            }

            fn category(&self) -> ToolCategory {
                ToolCategory::Ceremony
            }

            fn parameters_schema(&self) -> Value {
                json!({
                    "type": "object",
                    "properties": {"id": {"type": "string"}},
                    "required": ["id"],
                    "additionalProperties": false
                })
            }

            async fn execute(
                &self,
                _ctx: &dyn arawn_tool::ToolContext,
                params: Value,
            ) -> Result<ToolOutput, ToolError> {
                let id = match params.get("id").and_then(|v| v.as_str()) {
                    Some(s) if !s.is_empty() => s.to_string(),
                    _ => return Ok(ToolOutput::error("id is required")),
                };
                let store = match self.store.lock() {
                    Ok(s) => s,
                    Err(_) => return Ok(ToolOutput::error("store lock poisoned")),
                };
                let svc = build_service(&store, &self.events);
                match svc.$method(&id) {
                    Ok(out) => {
                        let _ = $allow_none;
                        Ok(ToolOutput::success(json_or_null(serde_json::to_string(&out))))
                    }
                    Err(e) => Ok(map_err(e)),
                }
            }
        }
    };
}

single_id_tool!(
    TodoDoneTool,
    "todo_done",
    "Mark a todo done. Idempotent — calling on an already-done row \
     preserves the original `done_at` timestamp. Emits a \
     `TodoEvent::Completed` notice so TUI views and other \
     subscribers re-render.",
    mark_done,
    ()
);

single_id_tool!(
    TodoUndoTool,
    "todo_undo",
    "Clear `done_at` on a todo, returning it to the open state. \
     Idempotent on already-open rows. Emits `TodoEvent::Updated`.",
    undo,
    ()
);

// `todo_get` has a different return shape (Option<Todo>), so it
// doesn't fit the macro. Inlined below.

pub struct TodoGetTool {
    store: Arc<Mutex<Store>>,
    events: Option<TodoEventSender>,
}

impl TodoGetTool {
    pub fn new(store: Arc<Mutex<Store>>, events: Option<TodoEventSender>) -> Self {
        Self { store, events }
    }
}

#[async_trait]
impl Tool for TodoGetTool {
    fn name(&self) -> &str {
        "todo_get"
    }

    fn description(&self) -> &str {
        "Fetch one todo by id, or `null` if no such row exists."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"id": {"type": "string"}},
            "required": ["id"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let id = match params.get("id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("id is required")),
        };
        let store = match self.store.lock() {
            Ok(s) => s,
            Err(_) => return Ok(ToolOutput::error("store lock poisoned")),
        };
        match build_service(&store, &self.events).get(&id) {
            Ok(Some(t)) => Ok(ToolOutput::success(json_or_null(serde_json::to_string(&t)))),
            Ok(None) => Ok(ToolOutput::success("null".to_string())),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// archive returns () not a Todo — handle specially.
pub struct TodoArchiveTool {
    store: Arc<Mutex<Store>>,
    events: Option<TodoEventSender>,
}

impl TodoArchiveTool {
    pub fn new(store: Arc<Mutex<Store>>, events: Option<TodoEventSender>) -> Self {
        Self { store, events }
    }
}

#[async_trait]
impl Tool for TodoArchiveTool {
    fn name(&self) -> &str {
        "todo_archive"
    }

    fn description(&self) -> &str {
        "Soft-delete a todo. The row stays in the table (so any \
         ceremony backrefs don't dangle) but is excluded from \
         `todo_list` and `todo_search` by default. Use this for \
         user-driven cancellation; for a state-transition \
         (done/undone) use `todo_done` / `todo_undo` instead."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"id": {"type": "string"}},
            "required": ["id"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let id = match params.get("id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("id is required")),
        };
        let store = match self.store.lock() {
            Ok(s) => s,
            Err(_) => return Ok(ToolOutput::error("store lock poisoned")),
        };
        match build_service(&store, &self.events).archive(&id) {
            Ok(()) => Ok(ToolOutput::success(
                json!({ "status": "archived", "id": id }).to_string(),
            )),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// todo_patch
// ============================================================================

pub struct TodoPatchTool {
    store: Arc<Mutex<Store>>,
    events: Option<TodoEventSender>,
}

impl TodoPatchTool {
    pub fn new(store: Arc<Mutex<Store>>, events: Option<TodoEventSender>) -> Self {
        Self { store, events }
    }
}

#[async_trait]
impl Tool for TodoPatchTool {
    fn name(&self) -> &str {
        "todo_patch"
    }

    fn description(&self) -> &str {
        "Edit a todo. `patch` carries the optional fields to update \
         — `body`, `rationale`, `workstream`, `due_at`. Anything \
         omitted is left unchanged. Returns the updated row. Emits \
         `TodoEvent::Updated`."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "patch": {
                    "type": "object",
                    "properties": {
                        "body": {"type": "string"},
                        "rationale": {"type": "string"},
                        "workstream": {"type": "string"},
                        "due_at": {"type": "string"}
                    },
                    "additionalProperties": false
                }
            },
            "required": ["id", "patch"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let id = match params.get("id").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("id is required")),
        };
        let patch_val = params.get("patch").cloned().unwrap_or(Value::Null);
        let patch: TodoPatch = if patch_val.is_null() {
            TodoPatch::default()
        } else {
            match serde_json::from_value(patch_val) {
                Ok(p) => p,
                Err(e) => return Ok(ToolOutput::error(format!("patch invalid: {e}"))),
            }
        };
        let store = match self.store.lock() {
            Ok(s) => s,
            Err(_) => return Ok(ToolOutput::error("store lock poisoned")),
        };
        match build_service(&store, &self.events).patch(&id, patch) {
            Ok(t) => Ok(ToolOutput::success(json_or_null(serde_json::to_string(&t)))),
            Err(e) => Ok(map_err(e)),
        }
    }
}

// ============================================================================
// todo_search
// ============================================================================

pub struct TodoSearchTool {
    store: Arc<Mutex<Store>>,
    events: Option<TodoEventSender>,
}

impl TodoSearchTool {
    pub fn new(store: Arc<Mutex<Store>>, events: Option<TodoEventSender>) -> Self {
        Self { store, events }
    }
}

#[async_trait]
impl Tool for TodoSearchTool {
    fn name(&self) -> &str {
        "todo_search"
    }

    fn description(&self) -> &str {
        "Search todos by body substring. v1 uses LIKE — full-text \
         search is deferred. Excludes archived rows. Returns rows \
         sorted by created_at desc."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Ceremony
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"query": {"type": "string"}},
            "required": ["query"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let q = match params.get("query").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("query is required")),
        };
        let store = match self.store.lock() {
            Ok(s) => s,
            Err(_) => return Ok(ToolOutput::error("store lock poisoned")),
        };
        match build_service(&store, &self.events).search(&q) {
            Ok(rows) => Ok(ToolOutput::success(json_or_null(serde_json::to_string(
                &rows,
            )))),
            Err(e) => Ok(map_err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Workstream;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn ctx() -> crate::context::EngineToolContext {
        let ws = Workstream::scratch("/tmp/test");
        crate::context::EngineToolContext::new(&ws, Uuid::new_v4())
    }

    fn open_store() -> (TempDir, Arc<Mutex<Store>>) {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        (tmp, Arc::new(Mutex::new(store)))
    }

    #[tokio::test]
    async fn create_round_trip_with_default_kind() {
        let (_tmp, store) = open_store();
        let tool = TodoCreateTool::new(store.clone(), None);
        let out = tool
            .execute(&ctx(), json!({"body": "ship the tools"}))
            .await
            .unwrap();
        assert!(!out.is_error, "got error: {}", out.content);
        let row: serde_json::Value = serde_json::from_str(&out.content).unwrap();
        assert_eq!(row["body"], "ship the tools");
        assert_eq!(row["kind"], "user");
        assert!(row["workstream"].is_null());
    }

    #[tokio::test]
    async fn create_rejects_empty_body() {
        let (_tmp, store) = open_store();
        let tool = TodoCreateTool::new(store.clone(), None);
        let out = tool.execute(&ctx(), json!({"body": ""})).await.unwrap();
        assert!(out.is_error);
    }

    #[tokio::test]
    async fn list_filters_by_kind_and_open_only() {
        let (_tmp, store) = open_store();
        let create = TodoCreateTool::new(store.clone(), None);
        let _ = create
            .execute(&ctx(), json!({"body": "a", "kind": "user"}))
            .await
            .unwrap();
        let _ = create
            .execute(&ctx(), json!({"body": "b", "kind": "rollover"}))
            .await
            .unwrap();
        let list = TodoListTool::new(store.clone(), None);
        let out = list
            .execute(&ctx(), json!({"kind": "user"}))
            .await
            .unwrap();
        let rows: Vec<serde_json::Value> = serde_json::from_str(&out.content).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["body"], "a");
    }

    #[tokio::test]
    async fn done_and_undo_round_trip() {
        let (_tmp, store) = open_store();
        let create = TodoCreateTool::new(store.clone(), None);
        let row: serde_json::Value = serde_json::from_str(
            &create
                .execute(&ctx(), json!({"body": "x"}))
                .await
                .unwrap()
                .content,
        )
        .unwrap();
        let id = row["id"].as_str().unwrap().to_string();

        let done = TodoDoneTool::new(store.clone(), None);
        let after: serde_json::Value = serde_json::from_str(
            &done
                .execute(&ctx(), json!({"id": &id}))
                .await
                .unwrap()
                .content,
        )
        .unwrap();
        assert!(after["done_at"].is_string());

        let undo = TodoUndoTool::new(store.clone(), None);
        let after: serde_json::Value = serde_json::from_str(
            &undo
                .execute(&ctx(), json!({"id": &id}))
                .await
                .unwrap()
                .content,
        )
        .unwrap();
        assert!(after["done_at"].is_null());
    }

    #[tokio::test]
    async fn done_unknown_id_returns_todo_error() {
        let (_tmp, store) = open_store();
        let done = TodoDoneTool::new(store.clone(), None);
        let out = done
            .execute(&ctx(), json!({"id": "missing"}))
            .await
            .unwrap();
        assert!(out.is_error);
        assert!(out.content.contains("todo_error"));
    }

    #[tokio::test]
    async fn patch_replaces_body() {
        let (_tmp, store) = open_store();
        let create = TodoCreateTool::new(store.clone(), None);
        let row: serde_json::Value = serde_json::from_str(
            &create
                .execute(&ctx(), json!({"body": "draft"}))
                .await
                .unwrap()
                .content,
        )
        .unwrap();
        let id = row["id"].as_str().unwrap().to_string();
        let patch = TodoPatchTool::new(store.clone(), None);
        let after: serde_json::Value = serde_json::from_str(
            &patch
                .execute(&ctx(), json!({"id": &id, "patch": {"body": "final"}}))
                .await
                .unwrap()
                .content,
        )
        .unwrap();
        assert_eq!(after["body"], "final");
    }

    #[tokio::test]
    async fn archive_then_list_hides_row() {
        let (_tmp, store) = open_store();
        let create = TodoCreateTool::new(store.clone(), None);
        let row: serde_json::Value = serde_json::from_str(
            &create
                .execute(&ctx(), json!({"body": "drop me"}))
                .await
                .unwrap()
                .content,
        )
        .unwrap();
        let id = row["id"].as_str().unwrap().to_string();
        let arc = TodoArchiveTool::new(store.clone(), None);
        let out = arc.execute(&ctx(), json!({"id": &id})).await.unwrap();
        assert!(!out.is_error);
        let list = TodoListTool::new(store.clone(), None);
        let out = list.execute(&ctx(), json!({})).await.unwrap();
        let rows: Vec<serde_json::Value> = serde_json::from_str(&out.content).unwrap();
        assert!(rows.is_empty());
        // include_archived returns it.
        let out = list
            .execute(&ctx(), json!({"include_archived": true}))
            .await
            .unwrap();
        let rows: Vec<serde_json::Value> = serde_json::from_str(&out.content).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn search_finds_body_substring() {
        let (_tmp, store) = open_store();
        let create = TodoCreateTool::new(store.clone(), None);
        let _ = create
            .execute(&ctx(), json!({"body": "ship the migration"}))
            .await
            .unwrap();
        let _ = create
            .execute(&ctx(), json!({"body": "write the README"}))
            .await
            .unwrap();
        let search = TodoSearchTool::new(store.clone(), None);
        let out = search
            .execute(&ctx(), json!({"query": "migration"}))
            .await
            .unwrap();
        let rows: Vec<serde_json::Value> = serde_json::from_str(&out.content).unwrap();
        assert_eq!(rows.len(), 1);
    }
}

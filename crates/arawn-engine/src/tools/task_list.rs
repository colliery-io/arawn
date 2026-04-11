use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::tool::{Tool, ToolCategory, ToolError, ToolOutput};

/// Session-scoped task status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Completed => write!(f, "completed"),
        }
    }
}

/// A single session-scoped task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTask {
    pub id: String,
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,
    pub status: TaskStatus,
}

/// Shared in-memory task store for a session.
/// Not persisted — lives only for the duration of the engine run.
#[derive(Debug, Clone, Default)]
pub struct SessionTaskStore {
    tasks: Arc<RwLock<HashMap<String, SessionTask>>>,
    order: Arc<RwLock<Vec<String>>>,
}

impl SessionTaskStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn create(
        &self,
        subject: String,
        description: Option<String>,
        active_form: Option<String>,
    ) -> SessionTask {
        let id = format!("task_{}", &Uuid::new_v4().to_string()[..8]);
        let task = SessionTask {
            id: id.clone(),
            subject,
            description,
            active_form,
            status: TaskStatus::Pending,
        };
        self.tasks.write().unwrap().insert(id.clone(), task.clone());
        self.order.write().unwrap().push(id);
        task
    }

    fn update(&self, id: &str, updates: TaskUpdates) -> Option<SessionTask> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            if let Some(status) = updates.status {
                task.status = status;
            }
            if let Some(subject) = updates.subject {
                task.subject = subject;
            }
            if let Some(description) = updates.description {
                task.description = Some(description);
            }
            if let Some(active_form) = updates.active_form {
                task.active_form = Some(active_form);
            }
            Some(task.clone())
        } else {
            None
        }
    }

    fn get(&self, id: &str) -> Option<SessionTask> {
        self.tasks.read().unwrap().get(id).cloned()
    }

    fn delete(&self, id: &str) -> bool {
        let removed = self.tasks.write().unwrap().remove(id).is_some();
        if removed {
            self.order.write().unwrap().retain(|i| i != id);
        }
        removed
    }

    fn list(&self) -> Vec<SessionTask> {
        let tasks = self.tasks.read().unwrap();
        let order = self.order.read().unwrap();
        order
            .iter()
            .filter_map(|id| tasks.get(id).cloned())
            .collect()
    }
}

struct TaskUpdates {
    status: Option<TaskStatus>,
    subject: Option<String>,
    description: Option<String>,
    active_form: Option<String>,
}

// ---------------------------------------------------------------------------
// TaskCreate tool
// ---------------------------------------------------------------------------

/// Creates a new session-scoped task for tracking work within the current session.
pub struct TaskCreateTool {
    store: SessionTaskStore,
}

impl TaskCreateTool {
    pub fn new(store: SessionTaskStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "task_create"
    }

    fn description(&self) -> &str {
        "Create a new task to track work within this session. Tasks help organize \
         multi-step work and show progress to the user.\n\n\
         ## When to Use\n\
         - Complex multi-step tasks (3+ distinct steps)\n\
         - When the user provides multiple things to do\n\
         - After receiving new instructions — capture requirements as tasks\n\n\
         ## When NOT to Use\n\
         - Single, straightforward tasks\n\
         - Trivial work that needs no tracking\n\n\
         All tasks are created with status `pending`."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Task
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "subject": {
                    "type": "string",
                    "description": "A brief, actionable title in imperative form (e.g., \"Fix authentication bug in login flow\")"
                },
                "description": {
                    "type": "string",
                    "description": "What needs to be done"
                },
                "activeForm": {
                    "type": "string",
                    "description": "Present continuous form shown in spinner when in_progress (e.g., \"Fixing authentication bug\"). If omitted, the subject is used."
                }
            },
            "required": ["subject"]
        })
    }

    async fn execute(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let subject = params
            .get("subject")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'subject' parameter".into()))?;

        let description = params
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);
        let active_form = params
            .get("activeForm")
            .and_then(|v| v.as_str())
            .map(String::from);

        let task = self
            .store
            .create(subject.to_string(), description, active_form);
        Ok(ToolOutput::success(
            serde_json::to_string_pretty(&task).unwrap_or_default(),
        ))
    }
}

// ---------------------------------------------------------------------------
// TaskUpdate tool
// ---------------------------------------------------------------------------

/// Updates a session task's status or details.
pub struct TaskUpdateTool {
    store: SessionTaskStore,
}

impl TaskUpdateTool {
    pub fn new(store: SessionTaskStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "task_update"
    }

    fn description(&self) -> &str {
        "Update a task in the task list.\n\n\
         ## Status Workflow\n\
         Status progresses: `pending` → `in_progress` → `completed`\n\
         Use `deleted` to permanently remove a task.\n\n\
         - Mark tasks as `in_progress` BEFORE starting work\n\
         - Mark as `completed` only when FULLY accomplished\n\
         - If blocked or errored, keep as `in_progress`\n\
         - After completing, call task_list to find your next task"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Task
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The task ID to update"
                },
                "status": {
                    "type": "string",
                    "enum": ["pending", "in_progress", "completed", "deleted"],
                    "description": "New status for the task"
                },
                "subject": {
                    "type": "string",
                    "description": "Change the task title"
                },
                "description": {
                    "type": "string",
                    "description": "Change the task description"
                },
                "activeForm": {
                    "type": "string",
                    "description": "Present continuous form shown in spinner when in_progress (e.g., \"Running tests\")"
                }
            },
            "required": ["task_id"]
        })
    }

    async fn execute(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let task_id = params
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'task_id' parameter".into()))?;

        let status_str = params.get("status").and_then(|v| v.as_str());

        // Handle deletion
        if status_str == Some("deleted") {
            if self.store.delete(task_id) {
                return Ok(ToolOutput::success(format!("Task '{task_id}' deleted.")));
            } else {
                return Ok(ToolOutput::error(format!("task '{task_id}' not found")));
            }
        }

        let status = match status_str {
            Some("pending") => Some(TaskStatus::Pending),
            Some("in_progress") => Some(TaskStatus::InProgress),
            Some("completed") => Some(TaskStatus::Completed),
            Some(other) => {
                return Err(ToolError::ExecutionFailed(format!(
                    "invalid status '{other}', expected: pending, in_progress, completed, deleted"
                )));
            }
            None => None,
        };

        let updates = TaskUpdates {
            status,
            subject: params
                .get("subject")
                .and_then(|v| v.as_str())
                .map(String::from),
            description: params
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            active_form: params
                .get("activeForm")
                .and_then(|v| v.as_str())
                .map(String::from),
        };

        // At least one field must be updated
        if updates.status.is_none()
            && updates.subject.is_none()
            && updates.description.is_none()
            && updates.active_form.is_none()
        {
            return Err(ToolError::ExecutionFailed(
                "at least one field (status, subject, description, activeForm) must be provided"
                    .into(),
            ));
        }

        match self.store.update(task_id, updates) {
            Some(task) => Ok(ToolOutput::success(
                serde_json::to_string_pretty(&task).unwrap_or_default(),
            )),
            None => Ok(ToolOutput::error(format!("task '{task_id}' not found"))),
        }
    }
}

// ---------------------------------------------------------------------------
// TaskList tool
// ---------------------------------------------------------------------------

/// Lists all session tasks with their status.
pub struct TaskListTool {
    store: SessionTaskStore,
}

impl TaskListTool {
    pub fn new(store: SessionTaskStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "task_list"
    }

    fn description(&self) -> &str {
        "List all tasks in the current session with their status.\n\n\
         ## Output\n\
         Returns each task with:\n\
         - **id**: Task identifier (use with task_update)\n\
         - **subject**: Brief description of the task\n\
         - **status**: pending, in_progress, or completed\n\n\
         After completing a task, call this to find the next available work."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Task
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _ctx: &dyn arawn_tool::ToolContext, _params: Value) -> Result<ToolOutput, ToolError> {
        let tasks = self.store.list();

        if tasks.is_empty() {
            return Ok(ToolOutput::success("No tasks."));
        }

        let mut output = String::new();
        for task in &tasks {
            let icon = match task.status {
                TaskStatus::Pending => "[ ]",
                TaskStatus::InProgress => "[~]",
                TaskStatus::Completed => "[x]",
            };
            output.push_str(&format!("{} {} ({})\n", icon, task.subject, task.id));
        }

        Ok(ToolOutput::success(output.trim()))
    }
}

// ---------------------------------------------------------------------------
// TaskGet tool
// ---------------------------------------------------------------------------

/// Gets full details of a session task by ID.
pub struct TaskGetTool {
    store: SessionTaskStore,
}

impl TaskGetTool {
    pub fn new(store: SessionTaskStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TaskGetTool {
    fn name(&self) -> &str {
        "task_get"
    }

    fn description(&self) -> &str {
        "Get a task by ID from the task list.\n\n\
         ## When to Use\n\
         - When you need the full description and context before starting work\n\
         - After being assigned a task, to get complete requirements\n\n\
         Use task_list to see all tasks in summary form."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Task
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The task ID to retrieve"
                }
            },
            "required": ["task_id"]
        })
    }

    async fn execute(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let task_id = params
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'task_id' parameter".into()))?;

        match self.store.get(task_id) {
            Some(task) => Ok(ToolOutput::success(
                serde_json::to_string_pretty(&task).unwrap_or_default(),
            )),
            None => Ok(ToolOutput::error(format!("task '{task_id}' not found"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Workstream;
    use serde_json::json;
    use uuid::Uuid;

    fn test_ctx() -> crate::context::EngineToolContext {
        let ws = Workstream::scratch("/tmp/test");
        crate::context::EngineToolContext::new(&ws, Uuid::new_v4())
    }

    #[test]
    fn store_create_and_list() {
        let store = SessionTaskStore::new();
        store.create("First task".into(), None, None);
        store.create("Second task".into(), Some("Details".into()), None);
        let tasks = store.list();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].subject, "First task");
        assert_eq!(tasks[1].subject, "Second task");
        assert_eq!(tasks[1].description.as_deref(), Some("Details"));
        assert_eq!(tasks[0].status, TaskStatus::Pending);
    }

    #[test]
    fn store_update_status() {
        let store = SessionTaskStore::new();
        let task = store.create("Do something".into(), None, None);
        let updated = store
            .update(
                &task.id,
                TaskUpdates {
                    status: Some(TaskStatus::InProgress),
                    subject: None,
                    description: None,
                    active_form: None,
                },
            )
            .unwrap();
        assert_eq!(updated.status, TaskStatus::InProgress);
    }

    #[test]
    fn store_update_subject_and_description() {
        let store = SessionTaskStore::new();
        let task = store.create("Original".into(), None, None);
        let updated = store
            .update(
                &task.id,
                TaskUpdates {
                    status: None,
                    subject: Some("Updated".into()),
                    description: Some("New details".into()),
                    active_form: Some("Updating...".into()),
                },
            )
            .unwrap();
        assert_eq!(updated.subject, "Updated");
        assert_eq!(updated.description.as_deref(), Some("New details"));
        assert_eq!(updated.active_form.as_deref(), Some("Updating..."));
    }

    #[test]
    fn store_delete() {
        let store = SessionTaskStore::new();
        let task = store.create("Delete me".into(), None, None);
        assert!(store.delete(&task.id));
        assert!(store.list().is_empty());
    }

    #[test]
    fn store_delete_nonexistent() {
        let store = SessionTaskStore::new();
        assert!(!store.delete("nope"));
    }

    #[test]
    fn store_update_nonexistent() {
        let store = SessionTaskStore::new();
        assert!(
            store
                .update(
                    "nope",
                    TaskUpdates {
                        status: Some(TaskStatus::Completed),
                        subject: None,
                        description: None,
                        active_form: None,
                    }
                )
                .is_none()
        );
    }

    #[test]
    fn store_preserves_order() {
        let store = SessionTaskStore::new();
        store.create("A".into(), None, None);
        store.create("B".into(), None, None);
        store.create("C".into(), None, None);
        let tasks = store.list();
        let names: Vec<&str> = tasks.iter().map(|t| t.subject.as_str()).collect();
        assert_eq!(names, vec!["A", "B", "C"]);
    }

    #[tokio::test]
    async fn task_create_tool() {
        let store = SessionTaskStore::new();
        let tool = TaskCreateTool::new(store.clone());
        let ctx = test_ctx();

        let result = tool
            .execute(
                &ctx,
                json!({"subject": "Write tests", "description": "Unit tests for all tools"}),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Write tests"));
        assert!(result.content.contains("pending"));
        assert_eq!(store.list().len(), 1);
    }

    #[tokio::test]
    async fn task_create_with_active_form() {
        let store = SessionTaskStore::new();
        let tool = TaskCreateTool::new(store.clone());
        let ctx = test_ctx();

        let result = tool
            .execute(
                &ctx,
                json!({"subject": "Run tests", "activeForm": "Running tests..."}),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let task: SessionTask = serde_json::from_str(&result.content).unwrap();
        assert_eq!(task.active_form.as_deref(), Some("Running tests..."));
    }

    #[tokio::test]
    async fn task_update_status() {
        let store = SessionTaskStore::new();
        let task = store.create("Fix bug".into(), None, None);
        let tool = TaskUpdateTool::new(store);
        let ctx = test_ctx();

        let result = tool
            .execute(&ctx, json!({"task_id": task.id, "status": "in_progress"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("in_progress"));
    }

    #[tokio::test]
    async fn task_update_delete() {
        let store = SessionTaskStore::new();
        let task = store.create("Remove me".into(), None, None);
        let tool = TaskUpdateTool::new(store.clone());
        let ctx = test_ctx();

        let result = tool
            .execute(&ctx, json!({"task_id": task.id, "status": "deleted"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("deleted"));
        assert!(store.list().is_empty());
    }

    #[tokio::test]
    async fn task_update_invalid_status() {
        let store = SessionTaskStore::new();
        let task = store.create("Fix bug".into(), None, None);
        let tool = TaskUpdateTool::new(store);
        let ctx = test_ctx();

        let result = tool
            .execute(&ctx, json!({"task_id": task.id, "status": "banana"}))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn task_update_no_fields_errors() {
        let store = SessionTaskStore::new();
        let task = store.create("Fix bug".into(), None, None);
        let tool = TaskUpdateTool::new(store);
        let ctx = test_ctx();

        let result = tool.execute(&ctx, json!({"task_id": task.id})).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn task_update_not_found() {
        let store = SessionTaskStore::new();
        let tool = TaskUpdateTool::new(store);
        let ctx = test_ctx();

        let result = tool
            .execute(&ctx, json!({"task_id": "nope", "status": "completed"}))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("not found"));
    }

    #[tokio::test]
    async fn task_list_empty() {
        let store = SessionTaskStore::new();
        let tool = TaskListTool::new(store);
        let ctx = test_ctx();

        let result = tool.execute(&ctx, json!({})).await.unwrap();
        assert_eq!(result.content, "No tasks.");
    }

    #[tokio::test]
    async fn task_list_with_tasks() {
        let store = SessionTaskStore::new();
        store.create("Task A".into(), None, None);
        let b = store.create("Task B".into(), None, None);
        store.update(
            &b.id,
            TaskUpdates {
                status: Some(TaskStatus::Completed),
                subject: None,
                description: None,
                active_form: None,
            },
        );

        let tool = TaskListTool::new(store);
        let ctx = test_ctx();

        let result = tool.execute(&ctx, json!({})).await.unwrap();
        assert!(result.content.contains("[ ] Task A"));
        assert!(result.content.contains("[x] Task B"));
    }

    #[tokio::test]
    async fn full_lifecycle() {
        let store = SessionTaskStore::new();
        let ctx = test_ctx();

        // Create
        let create = TaskCreateTool::new(store.clone());
        let r = create
            .execute(
                &ctx,
                json!({"subject": "Implement feature", "description": "Add new endpoint"}),
            )
            .await
            .unwrap();
        let created: SessionTask = serde_json::from_str(&r.content).unwrap();

        // Start
        let update = TaskUpdateTool::new(store.clone());
        update
            .execute(
                &ctx,
                json!({"task_id": created.id, "status": "in_progress"}),
            )
            .await
            .unwrap();

        // Complete
        update
            .execute(&ctx, json!({"task_id": created.id, "status": "completed"}))
            .await
            .unwrap();

        // List
        let list = TaskListTool::new(store);
        let r = list.execute(&ctx, json!({})).await.unwrap();
        assert!(r.content.contains("[x] Implement feature"));
    }

    #[test]
    fn schemas_are_valid() {
        let store = SessionTaskStore::new();
        let create = TaskCreateTool::new(store.clone());
        let update = TaskUpdateTool::new(store.clone());
        let list = TaskListTool::new(store.clone());
        let get = TaskGetTool::new(store);

        assert_eq!(create.parameters_schema()["type"], "object");
        assert!(create.parameters_schema()["properties"]["subject"].is_object());
        assert!(create.parameters_schema()["properties"]["description"].is_object());
        assert!(create.parameters_schema()["properties"]["activeForm"].is_object());

        assert_eq!(update.parameters_schema()["type"], "object");
        assert!(update.parameters_schema()["properties"]["status"]["enum"].is_array());

        assert_eq!(list.parameters_schema()["type"], "object");

        assert_eq!(get.parameters_schema()["type"], "object");
        assert!(get.parameters_schema()["properties"]["task_id"].is_object());
    }

    #[tokio::test]
    async fn task_get_found() {
        let store = SessionTaskStore::new();
        let task = store.create("Test task".into(), Some("Details here".into()), None);
        let tool = TaskGetTool::new(store);
        let ctx = test_ctx();

        let result = tool
            .execute(&ctx, json!({"task_id": task.id}))
            .await
            .unwrap();

        assert!(!result.is_error);
        let got: SessionTask = serde_json::from_str(&result.content).unwrap();
        assert_eq!(got.subject, "Test task");
        assert_eq!(got.description.as_deref(), Some("Details here"));
    }

    #[tokio::test]
    async fn task_get_not_found() {
        let store = SessionTaskStore::new();
        let tool = TaskGetTool::new(store);
        let ctx = test_ctx();

        let result = tool
            .execute(&ctx, json!({"task_id": "nope"}))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("not found"));
    }
}

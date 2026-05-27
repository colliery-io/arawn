//! Background sub-agent task inspection tools — `task_list` and `task_get`.
//!
//! These operate on [`BackgroundTaskManager`], which tracks tasks spawned via
//! the `agent` tool with `run_in_background: true` (and any future shell
//! background spawns). Companion tools: `task_output` (block/poll) and
//! `task_stop` (cancel).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::background::{BackgroundTaskManager, BackgroundTaskStatus};
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

/// List all background sub-agent tasks tracked in the current session.
pub struct TaskListTool {
    bg_manager: Arc<BackgroundTaskManager>,
}

impl TaskListTool {
    pub fn new(bg_manager: Arc<BackgroundTaskManager>) -> Self {
        Self { bg_manager }
    }
}

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "task_list"
    }

    fn description(&self) -> &str {
        "List background sub-agent tasks tracked in the current session. \
         Each entry is a snapshot — id, description, status (running / completed / \
         failed / killed), and elapsed seconds since launch."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::BackgroundTask
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        _params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let tasks = self.bg_manager.list();
        if tasks.is_empty() {
            return Ok(ToolOutput::success(
                "No background tasks in this session.".to_string(),
            ));
        }
        let mut lines = Vec::with_capacity(tasks.len() + 1);
        lines.push(format!(
            "Background tasks ({} total, {} running):",
            tasks.len(),
            self.bg_manager.running_count(),
        ));
        for t in tasks {
            lines.push(format!(
                "- {id}  {status}  {elapsed}s  {desc}",
                id = t.id,
                status = t.status,
                elapsed = t.elapsed_secs,
                desc = t.description,
            ));
        }
        Ok(ToolOutput::success(lines.join("\n")))
    }
}

/// Get a point-in-time snapshot of a single background sub-agent task.
///
/// Differs from `task_output` in that it never blocks — use this for "what's
/// the current state of bg_xxxx?" without waiting. Use `task_output` when
/// you want to block until completion.
pub struct TaskGetTool {
    bg_manager: Arc<BackgroundTaskManager>,
}

impl TaskGetTool {
    pub fn new(bg_manager: Arc<BackgroundTaskManager>) -> Self {
        Self { bg_manager }
    }
}

#[async_trait]
impl Tool for TaskGetTool {
    fn name(&self) -> &str {
        "task_get"
    }

    fn description(&self) -> &str {
        "Get the current status and buffered output for a single background \
         sub-agent task by id. Never blocks — for blocking/polling behaviour use \
         `task_output`."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::BackgroundTask
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The background task ID (e.g. bg_a1b2c3d4)"
                }
            },
            "required": ["task_id"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let task_id = params
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'task_id' parameter".into()))?;

        let Some(status) = self.bg_manager.status(task_id) else {
            return Ok(ToolOutput::error(format!("Unknown task: {task_id}")));
        };
        let output = self.bg_manager.read_output(task_id).unwrap_or_default();

        let status_line = match &status {
            BackgroundTaskStatus::Running => format!("Task {task_id}: running"),
            BackgroundTaskStatus::Completed { exit_code } => {
                let code = exit_code
                    .map(|c| format!(" (exit code {c})"))
                    .unwrap_or_default();
                format!("Task {task_id}: completed{code}")
            }
            BackgroundTaskStatus::Failed { error } => {
                format!("Task {task_id}: failed — {error}")
            }
            BackgroundTaskStatus::Killed => format!("Task {task_id}: killed"),
        };

        let is_error = matches!(status, BackgroundTaskStatus::Failed { .. });
        let body = if output.is_empty() {
            status_line
        } else {
            format!("{status_line}\n\nOutput:\n{output}")
        };
        Ok(if is_error {
            ToolOutput::error(body)
        } else {
            ToolOutput::success(body)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::background::BackgroundTaskKind;
    use arawn_core::Lens;
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    fn ctx() -> crate::context::EngineToolContext {
        let ws = Lens::scratch("/tmp/test");
        crate::context::EngineToolContext::new(&ws, Uuid::new_v4())
    }

    fn spawn_task(mgr: &BackgroundTaskManager, desc: &str) -> String {
        let token = CancellationToken::new();
        let token_clone = token.clone();
        let handle = tokio::spawn(async move { token_clone.cancelled().await });
        let (id, _) = mgr.register(
            BackgroundTaskKind::Shell {
                command: "noop".into(),
            },
            desc.into(),
            handle,
            token,
        );
        id
    }

    #[tokio::test]
    async fn list_empty() {
        let mgr = Arc::new(BackgroundTaskManager::new());
        let tool = TaskListTool::new(mgr);
        let result = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("No background tasks"));
    }

    #[tokio::test]
    async fn list_mixed_states() {
        let mgr = Arc::new(BackgroundTaskManager::new());
        let running = spawn_task(&mgr, "still going");
        let done = spawn_task(&mgr, "all done");
        mgr.complete(
            &done,
            BackgroundTaskStatus::Completed { exit_code: Some(0) },
        );
        let failed = spawn_task(&mgr, "blew up");
        mgr.complete(
            &failed,
            BackgroundTaskStatus::Failed {
                error: "boom".into(),
            },
        );

        let tool = TaskListTool::new(Arc::clone(&mgr));
        let result = tool.execute(&ctx(), json!({})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("3 total"));
        assert!(result.content.contains("1 running"));
        assert!(result.content.contains(&running));
        assert!(result.content.contains(&done));
        assert!(result.content.contains(&failed));
        assert!(result.content.contains("still going"));
    }

    #[tokio::test]
    async fn get_by_id_running() {
        let mgr = Arc::new(BackgroundTaskManager::new());
        let id = spawn_task(&mgr, "alive");
        let tool = TaskGetTool::new(mgr);
        let result = tool.execute(&ctx(), json!({"task_id": id})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("running"));
    }

    #[tokio::test]
    async fn get_unknown_id() {
        let mgr = Arc::new(BackgroundTaskManager::new());
        let tool = TaskGetTool::new(mgr);
        let result = tool
            .execute(&ctx(), json!({"task_id": "bg_nope"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("Unknown task"));
    }

    #[tokio::test]
    async fn get_missing_param() {
        let mgr = Arc::new(BackgroundTaskManager::new());
        let tool = TaskGetTool::new(mgr);
        let result = tool.execute(&ctx(), json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn metadata() {
        let mgr = Arc::new(BackgroundTaskManager::new());
        let list = TaskListTool::new(Arc::clone(&mgr));
        let get = TaskGetTool::new(mgr);
        assert_eq!(list.name(), "task_list");
        assert_eq!(get.name(), "task_get");
        assert!(list.is_read_only());
        assert!(get.is_read_only());
    }
}

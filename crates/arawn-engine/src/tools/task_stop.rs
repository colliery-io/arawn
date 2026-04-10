use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::info;

use crate::background::{BackgroundTaskManager, BackgroundTaskStatus};
use crate::context::ToolContext;
use crate::error::EngineError;
use crate::tool::{Tool, ToolCategory, ToolOutput};

/// Stop a running background task.
pub struct TaskStopTool {
    bg_manager: Arc<BackgroundTaskManager>,
}

impl TaskStopTool {
    pub fn new(bg_manager: Arc<BackgroundTaskManager>) -> Self {
        Self { bg_manager }
    }
}

#[async_trait]
impl Tool for TaskStopTool {
    fn name(&self) -> &str {
        "task_stop"
    }

    fn description(&self) -> &str {
        "Stop a running background task. The task is cancelled and its status \
         changes to 'killed'."
    }

    fn is_read_only(&self) -> bool {
        false
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
                    "description": "The background task ID to stop (e.g. bg_a1b2c3d4)"
                }
            },
            "required": ["task_id"]
        })
    }

    async fn execute(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        let task_id = params
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Tool("missing 'task_id' parameter".into()))?;

        // Check task exists
        let status = self.bg_manager.status(task_id);
        match status {
            None => Ok(ToolOutput::error(format!("Unknown task: {task_id}"))),
            Some(s) if s.is_terminal() => Ok(ToolOutput::error(format!(
                "Task {task_id} already finished (status: {})",
                s.label()
            ))),
            Some(_) => {
                info!(task_id, "stopping background task");
                self.bg_manager.cancel(task_id);
                // The spawned task will detect cancellation and call complete(Killed)
                // Give it a moment to propagate
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                self.bg_manager.complete(task_id, BackgroundTaskStatus::Killed);
                Ok(ToolOutput::success(format!("Task {task_id} stopped.")))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::background::BackgroundTaskKind;
    use arawn_core::Workstream;
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    fn test_ctx() -> ToolContext {
        let ws = Workstream::scratch("/tmp/test");
        ToolContext::new(&ws, Uuid::new_v4())
    }

    #[tokio::test]
    async fn stop_unknown_task() {
        let mgr = Arc::new(BackgroundTaskManager::new());
        let tool = TaskStopTool::new(mgr);
        let result = tool
            .execute(&test_ctx(), json!({"task_id": "bg_nonexistent"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("Unknown task"));
    }

    #[tokio::test]
    async fn stop_running_task() {
        let mgr = Arc::new(BackgroundTaskManager::new());
        let token = CancellationToken::new();
        let token_clone = token.clone();
        let handle = tokio::spawn(async move { token_clone.cancelled().await });

        let (id, _) = mgr.register(
            BackgroundTaskKind::Shell { command: "sleep 999".into() },
            "sleep".into(),
            handle,
            token,
        );

        let tool = TaskStopTool::new(Arc::clone(&mgr));
        let result = tool
            .execute(&test_ctx(), json!({"task_id": id}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("stopped"));
        assert_eq!(
            mgr.status(&id),
            Some(BackgroundTaskStatus::Killed)
        );
    }

    #[tokio::test]
    async fn stop_already_completed_task() {
        let mgr = Arc::new(BackgroundTaskManager::new());
        let token = CancellationToken::new();
        let handle = tokio::spawn(async {});

        let (id, _) = mgr.register(
            BackgroundTaskKind::Shell { command: "echo".into() },
            "echo".into(),
            handle,
            token,
        );
        mgr.complete(&id, BackgroundTaskStatus::Completed { exit_code: Some(0) });

        let tool = TaskStopTool::new(mgr);
        let result = tool
            .execute(&test_ctx(), json!({"task_id": id}))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("already finished"));
    }
}

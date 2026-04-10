use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::debug;

use crate::background::{BackgroundTaskManager, BackgroundTaskStatus};
use crate::context::ToolContext;
use crate::error::EngineError;
use crate::tool::{Tool, ToolCategory, ToolOutput};

/// Read the output and status of a background task.
/// Can block/poll until the task completes or return immediately.
pub struct TaskOutputTool {
    bg_manager: Arc<BackgroundTaskManager>,
}

impl TaskOutputTool {
    pub fn new(bg_manager: Arc<BackgroundTaskManager>) -> Self {
        Self { bg_manager }
    }
}

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        "task_output"
    }

    fn description(&self) -> &str {
        "Read the output and status of a background task. \
         By default, blocks until the task completes (up to timeout). \
         Set block=false to return the current status immediately."
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
                },
                "block": {
                    "type": "boolean",
                    "description": "Wait for task completion (default: true)"
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Max wait time in ms when blocking (default: 30000)"
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

        let block = params
            .get("block")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(30_000);

        // Check task exists
        let status = self.bg_manager.status(task_id);
        if status.is_none() {
            return Ok(ToolOutput::error(format!("Unknown task: {task_id}")));
        }

        // If blocking and task is still running, poll until done or timeout
        if block {
            let deadline = tokio::time::Instant::now()
                + tokio::time::Duration::from_millis(timeout_ms);

            loop {
                if let Some(status) = self.bg_manager.status(task_id) {
                    if status.is_terminal() {
                        break;
                    }
                }
                if tokio::time::Instant::now() >= deadline {
                    let output = self.bg_manager.read_output(task_id).unwrap_or_default();
                    return Ok(ToolOutput::success(format!(
                        "Task {task_id}: still running (timed out waiting after {timeout_ms}ms)\n\n\
                         Output so far:\n{output}"
                    )));
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        // Return current status and output
        let status = self.bg_manager.status(task_id).unwrap();
        let output = self.bg_manager.read_output(task_id).unwrap_or_default();

        let status_line = match &status {
            BackgroundTaskStatus::Running => format!("Task {task_id}: running"),
            BackgroundTaskStatus::Completed { exit_code } => {
                let code = exit_code.map(|c| format!(" (exit code {c})")).unwrap_or_default();
                format!("Task {task_id}: completed{code}")
            }
            BackgroundTaskStatus::Failed { error } => {
                format!("Task {task_id}: failed — {error}")
            }
            BackgroundTaskStatus::Killed => format!("Task {task_id}: killed"),
        };

        let is_error = matches!(status, BackgroundTaskStatus::Failed { .. });

        if output.is_empty() {
            if is_error {
                Ok(ToolOutput::error(status_line))
            } else {
                Ok(ToolOutput::success(status_line))
            }
        } else if is_error {
            Ok(ToolOutput::error(format!("{status_line}\n\nOutput:\n{output}")))
        } else {
            Ok(ToolOutput::success(format!("{status_line}\n\nOutput:\n{output}")))
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
    async fn unknown_task_returns_error() {
        let mgr = Arc::new(BackgroundTaskManager::new());
        let tool = TaskOutputTool::new(mgr);
        let result = tool
            .execute(&test_ctx(), json!({"task_id": "bg_nonexistent", "block": false}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("Unknown task"));
    }

    #[tokio::test]
    async fn completed_task_returns_output() {
        let mgr = Arc::new(BackgroundTaskManager::new());
        let token = CancellationToken::new();
        let handle = tokio::spawn(async {});

        let (id, output_buf) = mgr.register(
            BackgroundTaskKind::Shell { command: "echo hi".into() },
            "echo hi".into(),
            handle,
            token,
        );

        // Simulate output and completion
        crate::background::append_output(&output_buf, "hi\n");
        mgr.complete(&id, BackgroundTaskStatus::Completed { exit_code: Some(0) });

        let tool = TaskOutputTool::new(mgr);
        let result = tool
            .execute(&test_ctx(), json!({"task_id": id, "block": false}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("completed"));
        assert!(result.content.contains("hi"));
    }

    #[tokio::test]
    async fn running_task_non_blocking() {
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

        let tool = TaskOutputTool::new(mgr);
        let result = tool
            .execute(&test_ctx(), json!({"task_id": id, "block": false}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("running"));
    }
}

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::plan::PlanModeState;
use crate::tool::{Tool, ToolCategory, ToolError, ToolOutput};

/// Tool that exits plan mode — writes the plan to disk and deactivates plan mode
/// so all tools become available again. The plan content is returned for the user
/// to review in the conversation.
pub struct ExitPlanModeTool {
    plan_state: Arc<PlanModeState>,
}

impl ExitPlanModeTool {
    pub fn new(plan_state: Arc<PlanModeState>) -> Self {
        Self { plan_state }
    }
}

#[async_trait]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &str {
        "exit_plan_mode"
    }

    fn description(&self) -> &str {
        "Exit plan mode and present your plan to the user. The plan is written to \
         disk and all tools become available again. \
         \n\nCall this when your plan is complete and ready for review. Present the \
         plan clearly so the user can approve, request changes, or reject it."
    }

    fn is_read_only(&self) -> bool {
        // Must be callable from within plan mode
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Plan
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "plan": {
                    "type": "string",
                    "description": "The complete plan content. This will be written to the plan file and returned for user review."
                }
            },
            "required": ["plan"]
        })
    }

    async fn execute(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        if !self.plan_state.is_active() {
            return Ok(ToolOutput::error(
                "Not in plan mode. Use EnterPlanMode first.",
            ));
        }

        let plan_content = params
            .get("plan")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if plan_content.is_empty() {
            return Ok(ToolOutput::error(
                "Plan content is empty. Write a plan before calling ExitPlanMode.",
            ));
        }

        // Write the plan to disk
        self.plan_state
            .write_plan(plan_content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write plan: {e}")))?;

        let plan_file = self
            .plan_state
            .plan_file()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        // Exit plan mode — all tools become available again
        self.plan_state.exit();

        Ok(ToolOutput::success(format!(
            "Plan mode deactivated. All tools are now available.\n\n\
             Plan saved at: {plan_file}\n\n\
             Present the plan to the user for review before proceeding with implementation."
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::EngineToolContext;
    use crate::permissions::PermissionMode;
    use arawn_core::Workstream;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn test_ctx() -> EngineToolContext {
        let ws = Workstream::scratch("/tmp/test");
        EngineToolContext::new(&ws, Uuid::new_v4())
    }

    fn setup() -> (Arc<PlanModeState>, ExitPlanModeTool, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        let tmp_path = tmp.path().to_path_buf();
        std::mem::forget(tmp);

        let plan_state = Arc::new(PlanModeState::new());
        let tool = ExitPlanModeTool::new(plan_state.clone());
        (plan_state, tool, tmp_path)
    }

    #[tokio::test]
    async fn exit_not_in_plan_mode() {
        let (_, tool, _) = setup();
        let result = tool
            .execute(&test_ctx(), json!({"plan": "my plan"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("Not in plan mode"));
    }

    #[tokio::test]
    async fn exit_with_empty_plan() {
        let (plan_state, tool, tmp_path) = setup();
        plan_state.enter(PermissionMode::Default, "test", &tmp_path).unwrap();

        let result = tool.execute(&test_ctx(), json!({"plan": ""})).await.unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("empty"));
    }

    #[tokio::test]
    async fn exit_deactivates_plan_mode() {
        let (plan_state, tool, tmp_path) = setup();
        plan_state.enter(PermissionMode::Default, "exit-test", &tmp_path).unwrap();
        assert!(plan_state.is_active());

        let result = tool
            .execute(&test_ctx(), json!({"plan": "# Plan\n\nDo the thing."}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Plan mode deactivated"));
        assert!(!plan_state.is_active());
    }

    #[tokio::test]
    async fn plan_written_to_disk() {
        let (plan_state, tool, tmp_path) = setup();
        plan_state.enter(PermissionMode::Default, "disk-test", &tmp_path).unwrap();

        tool.execute(&test_ctx(), json!({"plan": "# My Plan\n\nStep 1: Go."}))
            .await
            .unwrap();

        let content = std::fs::read_to_string(tmp_path.join("disk-test.plan.md")).unwrap();
        assert!(content.contains("Step 1: Go"));
    }

    #[test]
    fn exit_plan_mode_is_read_only() {
        let plan_state = Arc::new(PlanModeState::new());
        let tool = ExitPlanModeTool::new(plan_state);
        assert!(tool.is_read_only());
    }
}

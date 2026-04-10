use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::context::ToolContext;
use crate::error::EngineError;
use crate::plan::{PlanModeState, generate_slug};
use crate::tool::{Tool, ToolCategory, ToolOutput};

/// Tool that enters plan mode — restricts the agent to observation-only tools
/// while it researches and designs an approach. The plan is written to a file
/// on disk and must be approved via ExitPlanMode before any actions are taken.
pub struct EnterPlanModeTool {
    plan_state: Arc<PlanModeState>,
}

impl EnterPlanModeTool {
    pub fn new(plan_state: Arc<PlanModeState>) -> Self {
        Self { plan_state }
    }
}

#[async_trait]
impl Tool for EnterPlanModeTool {
    fn name(&self) -> &str {
        "enter_plan_mode"
    }

    fn description(&self) -> &str {
        "Enter plan mode to research and design an approach before taking action. \
         In plan mode, only observation tools are available (Read, Glob, Grep, Think, \
         WebSearch, etc.). Tools with side effects (Edit, Write, Bash, etc.) are blocked. \
         \n\nUse this when you need to:\n\
         - Explore and understand before making changes\n\
         - Design a multi-step approach for the user to review\n\
         - Research options across files, docs, or web before committing to one\n\
         \n\nThe plan is saved to disk and persists across sessions. \
         When done planning, call ExitPlanMode to present the plan for approval."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Plan
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Brief description of what you're planning (used to name the plan file)"
                }
            },
            "required": ["description"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        if self.plan_state.is_active() {
            return Ok(ToolOutput::error(
                "Already in plan mode. Use ExitPlanMode to present your plan, or continue planning.",
            ));
        }

        let description = params
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("plan");

        let slug = generate_slug(description);

        // Enter plan mode — plan file lives in the session's working directory.
        // The QueryEngine's execute_tool checks plan_state.is_active()
        // and blocks non-read-only tools automatically.
        let plan_file = self
            .plan_state
            .enter(crate::permissions::PermissionMode::Default, &slug, &ctx.working_dir)
            .map_err(|e| EngineError::Tool(format!("Failed to create plan file: {e}")))?;

        Ok(ToolOutput::success(format!(
            "Plan mode activated. Only observation tools are now available.\n\n\
             Plan file: {}\n\n\
             Research the problem, then write your plan. When ready, call ExitPlanMode \
             to present it for approval. DO NOT attempt to use tools with side effects \
             (Edit, Write, Bash, etc.) — they will be blocked.",
            plan_file.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Workstream;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn test_ctx(dir: &std::path::Path) -> ToolContext {
        let ws = Workstream::scratch(dir);
        ToolContext::new(&ws, Uuid::new_v4())
    }

    #[tokio::test]
    async fn enter_plan_mode_activates() {
        let tmp = TempDir::new().unwrap();
        let plan_state = Arc::new(PlanModeState::new());
        let tool = EnterPlanModeTool::new(plan_state.clone());

        let result = tool
            .execute(&test_ctx(tmp.path()), json!({"description": "refactor auth"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Plan mode activated"));
        assert!(plan_state.is_active());
        // Plan file should be in the working dir
        assert!(tmp.path().join("refactor-auth.plan.md").exists());
    }

    #[tokio::test]
    async fn enter_plan_mode_when_already_active() {
        let tmp = TempDir::new().unwrap();
        let plan_state = Arc::new(PlanModeState::new());
        let tool = EnterPlanModeTool::new(plan_state.clone());

        tool.execute(&test_ctx(tmp.path()), json!({"description": "first"}))
            .await
            .unwrap();

        let result = tool
            .execute(&test_ctx(tmp.path()), json!({"description": "second"}))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("Already in plan mode"));
    }

    #[test]
    fn enter_plan_mode_is_read_only() {
        let plan_state = Arc::new(PlanModeState::new());
        let tool = EnterPlanModeTool::new(plan_state);
        assert!(tool.is_read_only());
    }
}

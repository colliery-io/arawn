use async_trait::async_trait;
use serde_json::{Value, json};

use crate::tool::{Tool, ToolError, ToolOutput};

/// A no-op reasoning scratchpad tool.
/// The LLM uses this to think step-by-step without side effects.
pub struct ThinkTool;

#[async_trait]
impl Tool for ThinkTool {
    fn name(&self) -> &str {
        "think"
    }

    fn description(&self) -> &str {
        "Use this tool as a scratchpad for step-by-step reasoning. No side effects — the thought is recorded but nothing else happens.\n\n\
         When to use:\n\
         - Before making a complex decision or choosing between approaches.\n\
         - To analyze tool results before deciding next steps.\n\
         - To reason about error messages or unexpected behavior.\n\
         - When you need to plan a multi-step operation before executing it."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "thought": {
                    "type": "string",
                    "description": "Your step-by-step reasoning"
                }
            },
            "required": ["thought"]
        })
    }

    async fn execute(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let thought = params
            .get("thought")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(ToolOutput::success(thought))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::EngineToolContext;
    use arawn_core::Workstream;
    use serde_json::json;
    use uuid::Uuid;

    fn test_ctx() -> EngineToolContext {
        let ws = Workstream::scratch("/tmp/test");
        EngineToolContext::new(&ws, Uuid::new_v4())
    }

    #[tokio::test]
    async fn think_returns_thought() {
        let tool = ThinkTool;
        let result = tool
            .execute(&test_ctx(), json!({"thought": "step 1: check the file"}))
            .await
            .unwrap();
        assert_eq!(result.content, "step 1: check the file");
        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn think_with_empty_thought() {
        let tool = ThinkTool;
        let result = tool.execute(&test_ctx(), json!({})).await.unwrap();
        assert_eq!(result.content, "");
        assert!(!result.is_error);
    }

    #[test]
    fn think_schema_is_valid() {
        let tool = ThinkTool;
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["thought"].is_object());
    }
}

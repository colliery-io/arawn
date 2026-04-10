use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::context::ToolContext;
use crate::error::EngineError;
use crate::skills::SkillRegistry;
use crate::tool::{Tool, ToolOutput};

/// Tool that executes skills (reusable prompt-based workflows).
///
/// The model calls this tool with a skill name and optional arguments.
/// The skill's prompt is returned as the tool output, which the model
/// then uses to guide its next response.
pub struct SkillTool {
    registry: Arc<SkillRegistry>,
}

impl SkillTool {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "skill"
    }

    fn description(&self) -> &str {
        "Execute a skill within the current conversation. \
         Skills provide specialized capabilities and domain knowledge. \
         When users reference a \"slash command\" or \"/<something>\" (e.g., \"/commit\", \"/review\"), \
         they are referring to a skill. Use this tool to invoke it."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "The skill name (e.g., \"commit\", \"review\")"
                },
                "args": {
                    "type": "string",
                    "description": "Optional arguments for the skill"
                }
            },
            "required": ["skill"]
        })
    }

    async fn execute(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        let skill_name = params
            .get("skill")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Tool("missing 'skill' parameter".into()))?;

        let args = params.get("args").and_then(|v| v.as_str()).unwrap_or("");

        let skill = match self.registry.get(skill_name) {
            Some(s) => s,
            None => {
                let available: Vec<String> = self
                    .registry
                    .user_invocable()
                    .iter()
                    .map(|s| s.name.clone())
                    .collect();
                return Ok(ToolOutput::error(format!(
                    "Skill '{}' not found. Available skills: {}",
                    skill_name,
                    if available.is_empty() {
                        "none".into()
                    } else {
                        available.join(", ")
                    }
                )));
            }
        };

        // Build the skill prompt with args if provided
        let prompt = if args.is_empty() {
            skill.prompt.clone()
        } else {
            format!("{}\n\nArguments: {}", skill.prompt, args)
        };

        Ok(ToolOutput::success(prompt))
    }

    fn is_read_only(&self) -> bool {
        // Skills may trigger write operations
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::{SkillDefinition, SkillSource};

    fn make_registry() -> Arc<SkillRegistry> {
        let registry = Arc::new(SkillRegistry::new());
        registry.register(SkillDefinition {
            name: "commit".into(),
            description: "Create a git commit".into(),
            prompt: "Review staged changes and create a commit with a conventional message.".into(),
            argument_hint: Some("[-m message]".into()),
            allowed_tools: Some(vec!["Bash(git *)".into(), "Read".into()]),
            model: None,
            user_invocable: true,
            source: SkillSource::Project,
        });
        registry.register(SkillDefinition {
            name: "review".into(),
            description: "Review code quality".into(),
            prompt: "Review the code for bugs, performance, and style.".into(),
            argument_hint: None,
            allowed_tools: None,
            model: None,
            user_invocable: true,
            source: SkillSource::Project,
        });
        registry.register(SkillDefinition {
            name: "internal".into(),
            description: "Internal skill".into(),
            prompt: "Internal use only.".into(),
            argument_hint: None,
            allowed_tools: None,
            model: None,
            user_invocable: false,
            source: SkillSource::BuiltIn,
        });
        registry
    }

    fn ctx() -> ToolContext {
        use arawn_core::Workstream;
        ToolContext::new(&Workstream::new("test", "/tmp"), uuid::Uuid::new_v4())
    }

    #[tokio::test]
    async fn execute_existing_skill() {
        let tool = SkillTool::new(make_registry());
        let result = tool
            .execute(&ctx(), serde_json::json!({"skill": "commit"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("staged changes"));
    }

    #[tokio::test]
    async fn execute_with_args() {
        let tool = SkillTool::new(make_registry());
        let result = tool
            .execute(
                &ctx(),
                serde_json::json!({"skill": "commit", "args": "-m 'fix bug'"}),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("staged changes"));
        assert!(result.content.contains("-m 'fix bug'"));
    }

    #[tokio::test]
    async fn execute_missing_skill() {
        let tool = SkillTool::new(make_registry());
        let result = tool
            .execute(&ctx(), serde_json::json!({"skill": "nonexistent"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("not found"));
        assert!(result.content.contains("commit"));
        assert!(result.content.contains("review"));
    }

    #[tokio::test]
    async fn execute_missing_param() {
        let tool = SkillTool::new(make_registry());
        let result = tool.execute(&ctx(), serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn tool_metadata() {
        let tool = SkillTool::new(make_registry());
        assert_eq!(tool.name(), "skill");
        assert!(!tool.is_read_only());
        assert!(tool.description().contains("slash command"));
    }

    #[test]
    fn schema_has_required_skill() {
        let tool = SkillTool::new(make_registry());
        let schema = tool.parameters_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "skill"));
    }
}

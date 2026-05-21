use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::skills::SkillRegistry;
use arawn_tool::{Tool, ToolError, ToolOutput};

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

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let skill_name = params
            .get("skill")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'skill' parameter".into()))?;

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

        let mut prompt = skill.prompt.clone();
        if !args.is_empty() {
            prompt.push_str(&format!("\n\nArguments: {}", args));
        }
        if let Some(constraints) = render_constraints(&skill) {
            prompt.push_str("\n\n");
            prompt.push_str(&constraints);
        }

        Ok(ToolOutput::success(prompt))
    }

    fn is_read_only(&self) -> bool {
        // Skills may trigger write operations
        false
    }
}

fn render_constraints(skill: &crate::skills::SkillDefinition) -> Option<String> {
    if skill.allowed_tools.is_none() && skill.model.is_none() {
        return None;
    }
    let mut out =
        String::from("---\nSkill constraints (advisory — the agent should self-comply):");
    if let Some(tools) = &skill.allowed_tools {
        out.push_str(&format!("\n- allowed-tools: {}", tools.join(", ")));
    }
    if let Some(model) = &skill.model {
        out.push_str(&format!("\n- recommended model: {}", model));
    }
    Some(out)
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

    fn ctx() -> crate::context::EngineToolContext {
        use arawn_core::Workstream;
        crate::context::EngineToolContext::new(
            &Workstream::new("test", "/tmp"),
            uuid::Uuid::new_v4(),
        )
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
        // commit skill has allowed_tools, so constraints footer must appear
        assert!(result.content.contains("Skill constraints"));
        assert!(result.content.contains("allowed-tools: Bash(git *), Read"));
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
        // constraints footer appears AFTER args
        let args_pos = result.content.find("Arguments:").unwrap();
        let footer_pos = result.content.find("Skill constraints").unwrap();
        assert!(footer_pos > args_pos);
    }

    #[tokio::test]
    async fn execute_no_constraints_footer_when_neither_field_set() {
        let tool = SkillTool::new(make_registry());
        let result = tool
            .execute(&ctx(), serde_json::json!({"skill": "review"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(!result.content.contains("Skill constraints"));
        assert_eq!(result.content, "Review the code for bugs, performance, and style.");
    }

    #[tokio::test]
    async fn execute_renders_model_only() {
        let registry = Arc::new(SkillRegistry::new());
        registry.register(SkillDefinition {
            name: "fast".into(),
            description: "fast".into(),
            prompt: "Do the thing.".into(),
            argument_hint: None,
            allowed_tools: None,
            model: Some("claude-haiku-4".into()),
            user_invocable: true,
            source: SkillSource::Project,
        });
        let tool = SkillTool::new(registry);
        let result = tool
            .execute(&ctx(), serde_json::json!({"skill": "fast"}))
            .await
            .unwrap();
        assert!(result.content.contains("Skill constraints"));
        assert!(result.content.contains("recommended model: claude-haiku-4"));
        assert!(!result.content.contains("allowed-tools"));
    }

    #[tokio::test]
    async fn execute_renders_both_fields() {
        let registry = Arc::new(SkillRegistry::new());
        registry.register(SkillDefinition {
            name: "both".into(),
            description: "both".into(),
            prompt: "Body.".into(),
            argument_hint: None,
            allowed_tools: Some(vec!["Read".into()]),
            model: Some("claude-sonnet-4-6".into()),
            user_invocable: true,
            source: SkillSource::Project,
        });
        let tool = SkillTool::new(registry);
        let result = tool
            .execute(&ctx(), serde_json::json!({"skill": "both"}))
            .await
            .unwrap();
        assert!(result.content.contains("allowed-tools: Read"));
        assert!(result.content.contains("recommended model: claude-sonnet-4-6"));
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

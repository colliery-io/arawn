use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use tracing::info;

use arawn_core::{Message, Session};

use crate::agent_defs::{AgentDefinition, build_agent_registry, find_agent};
use crate::background::{
    BackgroundTaskKind, BackgroundTaskManager, BackgroundTaskStatus, append_output,
};
use crate::compactor::Compactor;
use crate::context::ToolContext;
use crate::error::EngineError;
use crate::query_engine::{QueryEngine, QueryEngineConfig};
use crate::tool::{Tool, ToolCategory, ToolOutput, ToolRegistry};

const DEFAULT_MAX_TURNS: usize = 20;

/// Spawns a sub-agent that runs a full `QueryEngine` loop in an isolated
/// session. The `subagent_type` parameter selects an agent definition that
/// controls the system prompt, allowed tools, model, and max turns.
///
/// Sub-agents inherit the parent's model limits and compaction settings.
/// Nesting is limited to 3 levels deep.
pub struct AgentTool {
    registry: Arc<ToolRegistry>,
    definitions: Vec<AgentDefinition>,
    bg_manager: Option<Arc<BackgroundTaskManager>>,
}

impl AgentTool {
    pub fn new(registry: Arc<ToolRegistry>, definitions: Vec<AgentDefinition>) -> Self {
        Self {
            registry,
            definitions,
            bg_manager: None,
        }
    }

    /// Attach a background task manager for `run_in_background` support.
    pub fn with_background_manager(mut self, mgr: Arc<BackgroundTaskManager>) -> Self {
        self.bg_manager = Some(mgr);
        self
    }
}

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "agent"
    }

    fn description(&self) -> &str {
        // Return a static base description; the full one with agent types
        // is built dynamically but we can't return a borrowed &str from a
        // computed String. The agent type list is available via the schema.
        "Spawn a sub-agent to handle a complex task in an isolated context window. \
         The sub-agent runs a full agentic loop with a fresh conversation and its own tools.\n\n\
         Use `subagent_type` to select a specialized agent (e.g., \"Explore\", \"Plan\"). \
         If omitted, uses the general-purpose agent.\n\n\
         WHEN to use:\n\
         - Deep research or exploration that would fill your context with intermediate output\n\
         - Parallel investigation of independent sub-tasks\n\
         - Complex implementation work requiring many file reads/edits\n\n\
         WHEN NOT to use:\n\
         - Simple file reads or searches — use file_read, grep, glob directly\n\
         - Tasks requiring fewer than 3 tool calls\n\
         - When you need the results immediately in your current context\n\n\
         Write the prompt like a brief to a colleague who just walked in — explain what you're \
         trying to accomplish, what you already know, and what specifically you need. \
         Never delegate understanding: include file paths, context, and specifics."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The task for the sub-agent to perform"
                },
                "description": {
                    "type": "string",
                    "description": "Short (3-5 word) summary of the task"
                },
                "subagent_type": {
                    "type": "string",
                    "description": "The type of specialized agent to use for this task"
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Run the agent in the background. Returns a task ID immediately. Use TaskOutput to check status and read the result later."
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        let prompt = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Tool("missing 'prompt' parameter".into()))?;

        let description = params
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("sub-agent");

        let subagent_type = params
            .get("subagent_type")
            .and_then(|v| v.as_str())
            .unwrap_or("general-purpose");

        let run_in_background = params
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Guard against infinite recursion
        if !ctx.can_spawn_agent() {
            return Ok(ToolOutput::error(format!(
                "Cannot spawn sub-agent: maximum nesting depth ({}) reached.",
                ctx.agent_depth()
            )));
        }

        let llm = ctx
            .llm()
            .ok_or_else(|| EngineError::Tool("no LLM available for sub-agent".into()))?
            .clone();

        let parent_model = ctx
            .model()
            .ok_or_else(|| EngineError::Tool("no model configured for sub-agent".into()))?
            .to_string();

        // Look up agent definition
        let definition = find_agent(&self.definitions, subagent_type);

        info!(
            description,
            agent_type = %definition.name,
            depth = ctx.agent_depth() + 1,
            background = run_in_background,
            "spawning sub-agent"
        );

        // Resolve model: definition override > parent model
        let model = definition.model.as_ref().cloned().unwrap_or(parent_model);

        let max_turns = definition.max_turns.unwrap_or(DEFAULT_MAX_TURNS);

        // Build filtered tool registry based on agent definition
        let agent_registry = build_agent_registry(&self.registry, &definition);

        // Build config
        let config = QueryEngineConfig {
            model: model.clone(),
            max_iterations: max_turns,
            system_prompt: definition.system_prompt.clone(),
            max_tokens: Some(4096),
            model_limits: ctx.model_limits().clone(),
            data_dir: ctx.data_dir().cloned(),
            prompt_context: None,
        };

        // Sub-agent gets a compactor
        let compactor = Compactor::new(llm.clone(), model.clone());
        let mut engine =
            QueryEngine::with_config(llm.clone(), agent_registry, config).with_compactor(compactor);

        // Create child context with incremented depth
        let child_ctx = ctx.for_sub_agent();

        // Background execution: spawn and return immediately
        if run_in_background {
            let mgr = self.bg_manager.as_ref().ok_or_else(|| {
                EngineError::Tool("Background execution not available (no task manager configured)".into())
            })?;

            let cancel_token = CancellationToken::new();
            let mgr_clone = Arc::clone(mgr);
            let prompt_owned = prompt.to_string();
            let agent_type_owned = definition.name.clone();

            // Register with a placeholder handle first to get the task_id,
            // then spawn the real task with the id in scope.
            let placeholder = tokio::spawn(async {});
            let (task_id, output_buf) = mgr.register(
                BackgroundTaskKind::Agent {
                    prompt: prompt_owned.clone(),
                    agent_type: Some(agent_type_owned),
                },
                description.to_string(),
                placeholder,
                cancel_token,
            );

            let task_id_clone = task_id.clone();
            tokio::spawn(async move {
                let mut session = Session::new(child_ctx.session_id);
                session.add_message(Message::User {
                    content: prompt_owned,
                });

                let result = engine.run(&mut session, &child_ctx).await;

                match result {
                    Ok(response) => {
                        append_output(&output_buf, &response);
                        mgr_clone.complete(
                            &task_id_clone,
                            BackgroundTaskStatus::Completed { exit_code: None },
                        );
                    }
                    Err(e) => {
                        append_output(&output_buf, &format!("Error: {e}"));
                        mgr_clone.complete(
                            &task_id_clone,
                            BackgroundTaskStatus::Failed { error: e.to_string() },
                        );
                    }
                }
            });

            return Ok(ToolOutput::success(format!(
                "Background agent {task_id} started: {description}\n\n\
                 Use TaskOutput with task_id=\"{task_id}\" to check status and read the result."
            )));
        }

        // Foreground execution (existing behavior)
        let mut session = Session::new(ctx.session_id);
        session.add_message(Message::User {
            content: prompt.to_string(),
        });

        match engine.run(&mut session, &child_ctx).await {
            Ok(response) => {
                info!(description, agent_type = %definition.name, "sub-agent completed");
                Ok(ToolOutput::success(response))
            }
            Err(EngineError::MaxIterations { .. }) => {
                let last_text = session
                    .messages()
                    .iter()
                    .rev()
                    .find_map(|m| match m {
                        Message::Assistant { content, .. } if !content.is_empty() => {
                            Some(content.clone())
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        "Sub-agent reached maximum turns without a final response.".to_string()
                    });
                Ok(ToolOutput::success(last_text))
            }
            Err(e) => Err(EngineError::Tool(format!("sub-agent error: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_defs::built_in_agents;
    use arawn_core::Workstream;
    use arawn_llm::{MockLlmClient, MockResponse};
    use uuid::Uuid;

    fn test_ctx_with_mock(
        responses: Vec<MockResponse>,
    ) -> (ToolContext, Arc<MockLlmClient>, Arc<ToolRegistry>) {
        let mock = Arc::new(MockLlmClient::new(responses));
        let registry = Arc::new(ToolRegistry::new());
        let ws = Workstream::scratch("/tmp/test");
        let ctx =
            ToolContext::new(&ws, Uuid::new_v4()).with_llm(mock.clone(), "test-model".to_string());
        (ctx, mock, registry)
    }

    #[test]
    fn schema_is_valid() {
        let registry = Arc::new(ToolRegistry::new());
        let tool = AgentTool::new(registry, built_in_agents());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["prompt"].is_object());
        assert!(schema["properties"]["subagent_type"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("prompt")));
    }

    #[tokio::test]
    async fn text_only_sub_agent() {
        let (ctx, mock, registry) = test_ctx_with_mock(vec![MockResponse::text(
            "Here is the answer to your question.",
        )]);

        let tool = AgentTool::new(registry, built_in_agents());
        let result = tool
            .execute(
                &ctx,
                json!({"prompt": "What is 2+2?", "description": "Math question"}),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content, "Here is the answer to your question.");
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn sub_agent_with_tool_call() {
        let (ctx, mock, registry) = test_ctx_with_mock(vec![
            MockResponse::tool_call("call_1", "think", r#"{"thought":"Let me reason"}"#),
            MockResponse::text("After thinking, the answer is 4."),
        ]);

        registry.register(Box::new(crate::tools::think::ThinkTool));

        let tool = AgentTool::new(registry, built_in_agents());
        let result = tool
            .execute(&ctx, json!({"prompt": "What is 2+2?"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content, "After thinking, the answer is 4.");
        assert_eq!(mock.call_count(), 2);
    }

    #[tokio::test]
    async fn sub_agent_no_llm_errors() {
        let ws = Workstream::scratch("/tmp/test");
        let ctx = ToolContext::new(&ws, Uuid::new_v4());
        let registry = Arc::new(ToolRegistry::new());

        let tool = AgentTool::new(registry, built_in_agents());
        let result = tool.execute(&ctx, json!({"prompt": "Do something"})).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn sub_agent_max_iterations_returns_last_text() {
        let (ctx, _mock, registry) = test_ctx_with_mock(
            (0..21)
                .map(|i| {
                    MockResponse::tool_call(
                        format!("call_{i}"),
                        "think",
                        format!(r#"{{"thought":"iteration {i}"}}"#),
                    )
                })
                .collect(),
        );

        registry.register(Box::new(crate::tools::think::ThinkTool));

        let tool = AgentTool::new(registry, built_in_agents());
        let result = tool
            .execute(&ctx, json!({"prompt": "Loop forever"}))
            .await
            .unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn depth_limit_prevents_infinite_recursion() {
        let (ctx, _mock, registry) = test_ctx_with_mock(vec![]);

        let deep_ctx = ctx.for_sub_agent().for_sub_agent().for_sub_agent();
        assert!(!deep_ctx.can_spawn_agent());

        let tool = AgentTool::new(registry, built_in_agents());
        let result = tool
            .execute(&deep_ctx, json!({"prompt": "Spawn another"}))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("maximum nesting depth"));
    }

    #[tokio::test]
    async fn explore_agent_type_used() {
        let (ctx, mock, registry) =
            test_ctx_with_mock(vec![MockResponse::text("Found 3 matching files.")]);

        let tool = AgentTool::new(registry, built_in_agents());
        let result = tool
            .execute(
                &ctx,
                json!({"prompt": "Find all .rs files", "subagent_type": "Explore"}),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content, "Found 3 matching files.");
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn unknown_type_falls_back_to_general() {
        let (ctx, mock, registry) = test_ctx_with_mock(vec![MockResponse::text("Done.")]);

        let tool = AgentTool::new(registry, built_in_agents());
        let result = tool
            .execute(
                &ctx,
                json!({"prompt": "Do something", "subagent_type": "nonexistent"}),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(mock.call_count(), 1);
    }

    #[test]
    fn for_sub_agent_increments_depth() {
        let ws = Workstream::scratch("/tmp/test");
        let ctx = ToolContext::new(&ws, Uuid::new_v4());
        assert_eq!(ctx.agent_depth(), 0);

        let child = ctx.for_sub_agent();
        assert_eq!(child.agent_depth(), 1);

        let grandchild = child.for_sub_agent();
        assert_eq!(grandchild.agent_depth(), 2);

        let too_deep = grandchild.for_sub_agent();
        assert_eq!(too_deep.agent_depth(), 3);
        assert!(!too_deep.can_spawn_agent());
    }
}

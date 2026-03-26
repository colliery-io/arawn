use super::*;
use crate::tool::{MockTool, ToolResult};
use arawn_llm::{CompletionResponse, ContentBlock, MockBackend, MockResponse, StopReason, Usage};

fn mock_text_response(text: &str) -> CompletionResponse {
    CompletionResponse::new(
        "msg_1",
        "test-model",
        vec![ContentBlock::Text {
            text: text.to_string(),
            cache_control: None,
        }],
        StopReason::EndTurn,
        Usage::new(10, 20),
    )
}

fn mock_tool_use_response(
    tool_id: &str,
    tool_name: &str,
    args: serde_json::Value,
) -> CompletionResponse {
    CompletionResponse::new(
        "msg_1",
        "test-model",
        vec![ContentBlock::ToolUse {
            id: tool_id.to_string(),
            name: tool_name.to_string(),
            input: args,
            cache_control: None,
        }],
        StopReason::ToolUse,
        Usage::new(10, 20),
    )
}

#[test]
fn test_agent_builder_no_backend() {
    let result = Agent::builder().build();
    assert!(result.is_err());
}

#[test]
fn test_agent_builder_with_backend() {
    let backend = MockBackend::with_text("Hello");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_model("test-model")
        .with_system_prompt("You are helpful.")
        .build()
        .unwrap();

    assert_eq!(agent.config().model, "test-model");
    assert_eq!(
        agent.config().system_prompt,
        Some("You are helpful.".to_string())
    );
}

#[tokio::test]
async fn test_simple_turn_no_tools() {
    let backend = MockBackend::with_text("Hello! How can I help?");
    let agent = Agent::builder().with_backend(backend).build().unwrap();

    let mut session = Session::new();
    let response = agent.turn(&mut session, "Hi there", None).await.unwrap();

    assert_eq!(response.text, "Hello! How can I help?");
    assert!(response.tool_calls.is_empty());
    assert!(!response.truncated);
    assert_eq!(response.iterations, 1);
    assert_eq!(session.turn_count(), 1);
}

#[tokio::test]
async fn test_turn_with_tool_use() {
    // First response: tool call
    // Second response: final text
    let backend = MockBackend::new(vec![
        mock_tool_use_response("call_1", "test_tool", serde_json::json!({"arg": "value"})),
        mock_text_response("Done! I used the tool."),
    ]);

    let mut tools = ToolRegistry::new();
    tools.register(MockTool::new("test_tool").with_response(ToolResult::text("tool output")));

    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(tools)
        .build()
        .unwrap();

    let mut session = Session::new();
    let response = agent
        .turn(&mut session, "Use the tool", None)
        .await
        .unwrap();

    assert_eq!(response.text, "Done! I used the tool.");
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].name, "test_tool");
    assert_eq!(response.tool_results.len(), 1);
    assert!(response.tool_results[0].success);
    assert_eq!(response.iterations, 2);
}

#[tokio::test]
async fn test_turn_max_iterations() {
    // Keep returning tool calls to hit max iterations
    let responses: Vec<CompletionResponse> = (0..20)
        .map(|i| mock_tool_use_response(&format!("call_{}", i), "test_tool", serde_json::json!({})))
        .collect();

    let backend = MockBackend::new(responses);

    let mut tools = ToolRegistry::new();
    tools.register(MockTool::new("test_tool"));

    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(tools)
        .with_max_iterations(5)
        .build()
        .unwrap();

    let mut session = Session::new();
    let response = agent
        .turn(&mut session, "Keep using tools", None)
        .await
        .unwrap();

    assert!(response.truncated);
    assert_eq!(response.iterations, 6); // 5 + 1 that exceeded
}

#[tokio::test]
async fn test_turn_token_budget_exceeded() {
    // Each mock response uses Usage::new(10, 20) = 30 tokens per iteration.
    // With a budget of 50, the second iteration (60 total) should trigger truncation.
    let responses: Vec<CompletionResponse> = (0..10)
        .map(|i| mock_tool_use_response(&format!("call_{}", i), "test_tool", serde_json::json!({})))
        .collect();

    let backend = MockBackend::new(responses);

    let mut tools = ToolRegistry::new();
    tools.register(MockTool::new("test_tool"));

    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(tools)
        .with_max_total_tokens(50)
        .build()
        .unwrap();

    let mut session = Session::new();
    let response = agent.turn(&mut session, "Use tools", None).await.unwrap();

    assert!(response.truncated);
    // Budget exceeded on iteration 2 (60 tokens > 50 budget)
    assert_eq!(response.iterations, 2);
    assert!(response.usage.total() > 50);
}

#[tokio::test]
async fn test_turn_no_token_budget() {
    // Without a budget, token usage is not limited (only iterations)
    let backend = MockBackend::new(vec![mock_text_response("Hello!")]);

    let agent = Agent::builder().with_backend(backend).build().unwrap();

    assert!(agent.config().max_total_tokens.is_none());

    let mut session = Session::new();
    let response = agent.turn(&mut session, "Hi", None).await.unwrap();

    assert!(!response.truncated);
    assert_eq!(response.text, "Hello!");
}

#[tokio::test]
async fn test_turn_tool_error_handling() {
    // First response: tool call
    // Second response: final text
    let backend = MockBackend::new(vec![
        mock_tool_use_response("call_1", "failing_tool", serde_json::json!({})),
        mock_text_response("I see the tool failed."),
    ]);

    let mut tools = ToolRegistry::new();
    tools.register(
        MockTool::new("failing_tool").with_response(ToolResult::error("Something went wrong")),
    );

    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(tools)
        .build()
        .unwrap();

    let mut session = Session::new();
    let response = agent
        .turn(&mut session, "Try the failing tool", None)
        .await
        .unwrap();

    assert_eq!(response.text, "I see the tool failed.");
    assert!(!response.tool_results[0].success);
    assert!(
        response.tool_results[0]
            .content
            .contains("Something went wrong")
    );
}

#[tokio::test]
async fn test_turn_unknown_tool() {
    // Request a tool that doesn't exist
    let backend = MockBackend::new(vec![
        mock_tool_use_response("call_1", "nonexistent_tool", serde_json::json!({})),
        mock_text_response("I couldn't find that tool."),
    ]);

    let agent = Agent::builder().with_backend(backend).build().unwrap();

    let mut session = Session::new();
    let response = agent
        .turn(&mut session, "Use unknown tool", None)
        .await
        .unwrap();

    assert!(!response.tool_results[0].success);
    assert!(response.tool_results[0].content.contains("not found"));
}

#[tokio::test]
async fn test_tool_validation_error_retry() {
    // Test that when the backend returns a tool validation error (LLM hallucinated
    // a tool name), the agent injects feedback and retries instead of failing.
    let tool_validation_error = "tool call validation failed: attempted to call tool 'read_file' which was not in request.tools".to_string();

    let backend = MockBackend::with_results(vec![
        // First call: backend rejects with tool validation error
        MockResponse::Error(tool_validation_error),
        // Second call: LLM corrects itself and returns text
        MockResponse::Success(mock_text_response("I'll use the correct tool name.")),
    ]);

    let agent = Agent::builder().with_backend(backend).build().unwrap();

    let mut session = Session::new();
    let response = agent
        .turn(&mut session, "Read the file", None)
        .await
        .unwrap();

    // Should succeed after retry
    assert_eq!(response.text, "I'll use the correct tool name.");
}

#[tokio::test]
async fn test_tool_validation_error_exhausts_retries() {
    // Test that repeated tool validation errors eventually hit the iteration limit
    let tool_validation_error = "tool call validation failed: attempted to call tool 'bad_tool' which was not in request.tools".to_string();

    // Return errors for more iterations than max_iterations
    let errors: Vec<MockResponse> = (0..15)
        .map(|_| MockResponse::Error(tool_validation_error.clone()))
        .collect();

    let backend = MockBackend::with_results(errors);

    let agent = Agent::builder()
        .with_backend(backend)
        .with_max_iterations(3) // Low limit to speed up test
        .build()
        .unwrap();

    let mut session = Session::new();
    let result = agent
        .turn(&mut session, "Keep failing", None)
        .await
        .unwrap();

    // Should hit max iterations and return truncated response
    assert!(result.text.contains("truncated") || result.text.contains("max iterations"));
}

#[tokio::test]
async fn test_multi_turn_conversation() {
    let backend = MockBackend::new(vec![
        mock_text_response("Hello!"),
        mock_text_response("I'm doing great, thanks for asking!"),
    ]);

    let agent = Agent::builder().with_backend(backend).build().unwrap();

    let mut session = Session::new();

    let r1 = agent.turn(&mut session, "Hi", None).await.unwrap();
    assert_eq!(r1.text, "Hello!");
    assert_eq!(session.turn_count(), 1);

    let r2 = agent
        .turn(&mut session, "How are you?", None)
        .await
        .unwrap();
    assert_eq!(r2.text, "I'm doing great, thanks for asking!");
    assert_eq!(session.turn_count(), 2);
}

#[test]
fn test_agent_with_prompt_builder() {
    use crate::prompt::{PromptMode, SystemPromptBuilder};

    let backend = MockBackend::with_text("Hello");

    let prompt_builder = SystemPromptBuilder::new()
        .with_mode(PromptMode::Full)
        .with_identity("TestAgent", "a test assistant");

    let agent = Agent::builder()
        .with_backend(backend)
        .with_tool(MockTool::new("test_tool").with_description("A test tool"))
        .with_workspace("/test/workspace")
        .with_prompt_builder(prompt_builder)
        .build()
        .unwrap();

    // Verify the system prompt was generated
    let system_prompt = agent.system_prompt().unwrap();
    assert!(system_prompt.contains("You are TestAgent"));
    assert!(system_prompt.contains("test assistant"));
    assert!(system_prompt.contains("test_tool"));
    assert!(system_prompt.contains("/test/workspace"));
}

#[test]
fn test_agent_prompt_builder_with_static_fallback() {
    // When no prompt builder is set, system_prompt from config should be used
    let backend = MockBackend::with_text("Hello");

    let agent = Agent::builder()
        .with_backend(backend)
        .with_system_prompt("Static system prompt")
        .build()
        .unwrap();

    assert_eq!(
        agent.config().system_prompt,
        Some("Static system prompt".to_string())
    );
}

#[test]
fn test_agent_prompt_builder_overrides_static() {
    use crate::prompt::{PromptMode, SystemPromptBuilder};

    let backend = MockBackend::with_text("Hello");

    let prompt_builder = SystemPromptBuilder::new()
        .with_mode(PromptMode::Full)
        .with_identity("Dynamic", "agent");

    // Set both static and builder - builder should win
    let agent = Agent::builder()
        .with_backend(backend)
        .with_system_prompt("This should be overridden")
        .with_prompt_builder(prompt_builder)
        .build()
        .unwrap();

    let system_prompt = agent.system_prompt().unwrap();
    assert!(system_prompt.contains("You are Dynamic"));
    assert!(!system_prompt.contains("This should be overridden"));
}

#[test]
fn test_agent_with_bootstrap_dir() {
    use crate::prompt::{PromptMode, SystemPromptBuilder};
    use std::fs;
    use tempfile::TempDir;

    let backend = MockBackend::with_text("Hello");

    // Create temp dir with a BEHAVIOR.md file
    let temp_dir = TempDir::new().unwrap();
    fs::write(
        temp_dir.path().join("BEHAVIOR.md"),
        "# Soul\n\nYou are kind and helpful.",
    )
    .unwrap();

    let prompt_builder = SystemPromptBuilder::new()
        .with_mode(PromptMode::Full)
        .with_identity("BootstrapAgent", "an agent with soul");

    let agent = Agent::builder()
        .with_backend(backend)
        .with_prompt_builder(prompt_builder)
        .with_bootstrap_dir(temp_dir.path())
        .build()
        .unwrap();

    let system_prompt = agent.system_prompt().unwrap();
    assert!(system_prompt.contains("You are BootstrapAgent"));
    assert!(system_prompt.contains("kind and helpful"));
    assert!(system_prompt.contains("BEHAVIOR.md"));
}

#[test]
fn test_agent_bootstrap_dir_creates_builder_if_none() {
    use std::fs;
    use tempfile::TempDir;

    let backend = MockBackend::with_text("Hello");

    // Create temp dir with a BEHAVIOR.md file
    let temp_dir = TempDir::new().unwrap();
    fs::write(
        temp_dir.path().join("BEHAVIOR.md"),
        "Be excellent to each other.",
    )
    .unwrap();

    // No prompt builder set - bootstrap_dir should create one
    let agent = Agent::builder()
        .with_backend(backend)
        .with_bootstrap_dir(temp_dir.path())
        .build()
        .unwrap();

    let system_prompt = agent.system_prompt().unwrap();
    assert!(system_prompt.contains("Be excellent"));
}

#[test]
fn test_agent_bootstrap_dir_nonexistent_is_ok() {
    let backend = MockBackend::with_text("Hello");

    // Non-existent directory should not cause an error
    let agent = Agent::builder()
        .with_backend(backend)
        .with_bootstrap_dir("/nonexistent/path/to/prompts")
        .build()
        .unwrap();

    // Should build successfully, just with no bootstrap content
    assert!(agent.config().system_prompt.is_none());
}

#[test]
fn test_agent_with_prompt_file() {
    use std::fs;
    use tempfile::TempDir;

    let backend = MockBackend::with_text("Hello");

    // Create temp file
    let temp_dir = TempDir::new().unwrap();
    let custom_file = temp_dir.path().join("custom_persona.md");
    fs::write(&custom_file, "You have a friendly personality.").unwrap();

    let agent = Agent::builder()
        .with_backend(backend)
        .with_prompt_file(&custom_file)
        .build()
        .unwrap();

    let system_prompt = agent.system_prompt().unwrap();
    assert!(system_prompt.contains("friendly personality"));
    assert!(system_prompt.contains("custom_persona.md"));
}

#[test]
fn test_agent_with_multiple_prompt_files() {
    use std::fs;
    use tempfile::TempDir;

    let backend = MockBackend::with_text("Hello");

    // Create multiple temp files
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("persona.md");
    let file2 = temp_dir.path().join("guidelines.md");
    fs::write(&file1, "Be helpful and kind.").unwrap();
    fs::write(&file2, "Always verify your answers.").unwrap();

    let agent = Agent::builder()
        .with_backend(backend)
        .with_prompt_file(&file1)
        .with_prompt_file(&file2)
        .build()
        .unwrap();

    let system_prompt = agent.system_prompt().unwrap();
    assert!(system_prompt.contains("helpful and kind"));
    assert!(system_prompt.contains("verify your answers"));
}

#[test]
fn test_agent_combine_bootstrap_dir_and_prompt_file() {
    use std::fs;
    use tempfile::TempDir;

    let backend = MockBackend::with_text("Hello");

    // Create bootstrap dir with BEHAVIOR.md
    let bootstrap_dir = TempDir::new().unwrap();
    fs::write(
        bootstrap_dir.path().join("BEHAVIOR.md"),
        "Core values here.",
    )
    .unwrap();

    // Create custom file elsewhere
    let custom_dir = TempDir::new().unwrap();
    let custom_file = custom_dir.path().join("extra.md");
    fs::write(&custom_file, "Additional guidelines.").unwrap();

    let agent = Agent::builder()
        .with_backend(backend)
        .with_bootstrap_dir(bootstrap_dir.path())
        .with_prompt_file(&custom_file)
        .build()
        .unwrap();

    let system_prompt = agent.system_prompt().unwrap();
    // Should have both
    assert!(system_prompt.contains("Core values"));
    assert!(system_prompt.contains("Additional guidelines"));
}

// ── End-to-End Agent Loop Tests ──────────────────────────────────

mod e2e_tests {
    use super::*;
    use std::sync::Arc;

    /// Verify that tool output is sent back to the LLM as a tool result message.
    #[tokio::test]
    async fn test_tool_output_flows_back_to_llm() {
        let backend = Arc::new(MockBackend::new(vec![
            mock_tool_use_response("call_1", "echo", serde_json::json!({"text": "hello world"})),
            mock_text_response("The echo tool returned: hello world"),
        ]));

        let mut tools = ToolRegistry::new();
        tools.register(MockTool::new("echo").with_response(ToolResult::text("ECHO: hello world")));

        let agent = Agent::builder()
            .with_shared_backend(backend.clone())
            .with_tools(tools)
            .build()
            .unwrap();

        let mut session = Session::new();
        agent
            .turn(&mut session, "Echo hello world", None)
            .await
            .unwrap();

        // The second request should contain the tool result from the first call
        let requests = backend.requests();
        assert_eq!(requests.len(), 2);

        // Inspect the second request's messages for tool result content
        let second_req_messages = &requests[1].messages;
        let has_tool_result = second_req_messages.iter().any(|msg| {
            msg.content.blocks().iter().any(|block| match block {
                ContentBlock::ToolResult {
                    content: Some(arawn_llm::ToolResultContent::Text(text)),
                    ..
                } => text.contains("ECHO: hello world"),
                _ => false,
            })
        });
        assert!(
            has_tool_result,
            "Tool output should appear in the follow-up LLM request"
        );
    }

    /// Verify tool arguments are passed through to the tool exactly as the LLM specified.
    #[tokio::test]
    async fn test_tool_arguments_pass_through() {
        let echo_tool = Arc::new(MockTool::new("echo").with_response(ToolResult::text("done")));

        let backend = MockBackend::new(vec![
            mock_tool_use_response(
                "call_1",
                "echo",
                serde_json::json!({"message": "test input", "count": 3}),
            ),
            mock_text_response("Done."),
        ]);

        let mut tools = ToolRegistry::new();
        tools.register_arc(echo_tool.clone());

        let agent = Agent::builder()
            .with_backend(backend)
            .with_tools(tools)
            .build()
            .unwrap();

        let mut session = Session::new();
        agent.turn(&mut session, "Echo it", None).await.unwrap();

        assert_eq!(echo_tool.call_count(), 1);
        let args = &echo_tool.calls()[0];
        assert_eq!(args["message"], "test input");
        assert_eq!(args["count"], 3);
    }

    /// Multi-turn conversation with tools: first turn uses a tool, second turn follows up.
    #[tokio::test]
    async fn test_multi_turn_with_tool_then_followup() {
        let backend = Arc::new(MockBackend::new(vec![
            // Turn 1: tool call
            mock_tool_use_response("call_1", "lookup", serde_json::json!({"key": "user_name"})),
            // Turn 1: final response after tool
            mock_text_response("Your name is Alice."),
            // Turn 2: text-only response referencing previous context
            mock_text_response("Yes, I already told you — your name is Alice."),
        ]));

        let mut tools = ToolRegistry::new();
        tools.register(MockTool::new("lookup").with_response(ToolResult::text("Alice")));

        let agent = Agent::builder()
            .with_shared_backend(backend.clone())
            .with_tools(tools)
            .build()
            .unwrap();

        let mut session = Session::new();

        // Turn 1: uses tool
        let r1 = agent
            .turn(&mut session, "What is my name?", None)
            .await
            .unwrap();
        assert_eq!(r1.text, "Your name is Alice.");
        assert_eq!(r1.tool_calls.len(), 1);
        assert_eq!(r1.iterations, 2);

        // Turn 2: follow-up without tools
        let r2 = agent
            .turn(&mut session, "Can you repeat that?", None)
            .await
            .unwrap();
        assert_eq!(r2.text, "Yes, I already told you — your name is Alice.");
        assert!(r2.tool_calls.is_empty());
        assert_eq!(r2.iterations, 1);

        // Verify the third LLM request includes the full history:
        // user msg 1, assistant (tool_use), tool result, assistant text, user msg 2
        let requests = backend.requests();
        assert_eq!(requests.len(), 3);
        let third_req = &requests[2];
        // Should have: user("What is my name?"), assistant(tool_use), user(tool_result),
        //              assistant("Your name is Alice."), user("Can you repeat that?") -- but
        //              build_messages reconstructs from session turns, so check message count
        assert!(
            third_req.messages.len() >= 3,
            "Third request should have full conversation history"
        );
    }

    /// Session state records tool calls and results correctly after a turn.
    #[tokio::test]
    async fn test_session_records_tool_state() {
        let backend = MockBackend::new(vec![
            mock_tool_use_response("call_abc", "greet", serde_json::json!({"name": "Bob"})),
            mock_text_response("Hello, Bob!"),
        ]);

        let mut tools = ToolRegistry::new();
        tools.register(
            MockTool::new("greet").with_response(ToolResult::text("Greeting sent to Bob")),
        );

        let agent = Agent::builder()
            .with_backend(backend)
            .with_tools(tools)
            .build()
            .unwrap();

        let mut session = Session::new();
        let response = agent.turn(&mut session, "Greet Bob", None).await.unwrap();

        // Verify session turn state
        assert_eq!(session.turn_count(), 1);
        let turn = &session.all_turns()[0];
        assert_eq!(turn.user_message, "Greet Bob");
        assert_eq!(turn.assistant_response.as_deref(), Some("Hello, Bob!"));

        // Turn should record tool calls and results
        assert_eq!(turn.tool_calls.len(), 1);
        assert_eq!(turn.tool_calls[0].id, "call_abc");
        assert_eq!(turn.tool_calls[0].name, "greet");
        assert_eq!(turn.tool_calls[0].arguments["name"], "Bob");

        assert_eq!(turn.tool_results.len(), 1);
        assert!(turn.tool_results[0].success);
        assert!(
            turn.tool_results[0]
                .content
                .contains("Greeting sent to Bob")
        );

        // AgentResponse should match
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_results.len(), 1);
    }

    /// Tool error result flows back to the LLM and the agent produces a graceful response.
    #[tokio::test]
    async fn test_tool_error_flows_back_to_llm() {
        let backend = Arc::new(MockBackend::new(vec![
            mock_tool_use_response("call_1", "risky_op", serde_json::json!({})),
            mock_text_response("The operation failed, but I can help you recover."),
        ]));

        let mut tools = ToolRegistry::new();
        tools.register(
            MockTool::new("risky_op").with_response(ToolResult::error("Permission denied")),
        );

        let agent = Agent::builder()
            .with_shared_backend(backend.clone())
            .with_tools(tools)
            .build()
            .unwrap();

        let mut session = Session::new();
        let response = agent
            .turn(&mut session, "Do the risky thing", None)
            .await
            .unwrap();

        assert_eq!(
            response.text,
            "The operation failed, but I can help you recover."
        );
        assert!(!response.tool_results[0].success);

        // Verify the error was sent back to the LLM in the second request
        let requests = backend.requests();
        let second_messages = &requests[1].messages;
        let has_error_result = second_messages.iter().any(|msg| {
            msg.content
                .blocks()
                .iter()
                .any(|block| matches!(block, ContentBlock::ToolResult { is_error: true, .. }))
        });
        assert!(
            has_error_result,
            "Error tool result should be sent back to LLM"
        );
    }

    /// Multiple sequential tool calls within a single turn.
    #[tokio::test]
    async fn test_multiple_sequential_tool_calls() {
        let backend = MockBackend::new(vec![
            // First iteration: call tool A
            mock_tool_use_response("call_1", "step_one", serde_json::json!({})),
            // Second iteration: call tool B
            mock_tool_use_response("call_2", "step_two", serde_json::json!({})),
            // Third iteration: final text
            mock_text_response("Both steps completed."),
        ]);

        let mut tools = ToolRegistry::new();
        tools.register(MockTool::new("step_one").with_response(ToolResult::text("step 1 done")));
        tools.register(MockTool::new("step_two").with_response(ToolResult::text("step 2 done")));

        let agent = Agent::builder()
            .with_backend(backend)
            .with_tools(tools)
            .build()
            .unwrap();

        let mut session = Session::new();
        let response = agent
            .turn(&mut session, "Do both steps", None)
            .await
            .unwrap();

        assert_eq!(response.text, "Both steps completed.");
        assert_eq!(response.tool_calls.len(), 2);
        assert_eq!(response.tool_calls[0].name, "step_one");
        assert_eq!(response.tool_calls[1].name, "step_two");
        assert_eq!(response.iterations, 3);

        // Both tool results should be successful
        assert!(response.tool_results.iter().all(|r| r.success));
    }

    /// Usage tokens accumulate correctly across tool-call iterations.
    #[tokio::test]
    async fn test_usage_accumulates_across_iterations() {
        let backend = MockBackend::new(vec![
            mock_tool_use_response("call_1", "echo", serde_json::json!({})),
            mock_text_response("Done"),
        ]);

        let mut tools = ToolRegistry::new();
        tools.register(MockTool::new("echo"));

        let agent = Agent::builder()
            .with_backend(backend)
            .with_tools(tools)
            .build()
            .unwrap();

        let mut session = Session::new();
        let response = agent.turn(&mut session, "Go", None).await.unwrap();

        // Each mock response uses Usage::new(10, 20), so 2 iterations = 20 input, 40 output
        assert_eq!(response.usage.input_tokens, 20);
        assert_eq!(response.usage.output_tokens, 40);
        assert_eq!(response.usage.total(), 60);
    }
}

// ── Active Recall Tests ──────────────────────────────────────────

mod recall_tests {
    use super::*;
    use arawn_llm::Embedder;
    use arawn_memory::store::MemoryStore;
    use arawn_memory::types::{ContentType, Memory};
    use serial_test::serial;

    /// Simple mock embedder that returns a fixed vector.
    struct FixedEmbedder {
        dims: usize,
    }

    impl FixedEmbedder {
        fn new(dims: usize) -> Self {
            Self { dims }
        }
    }

    #[async_trait::async_trait]
    impl Embedder for FixedEmbedder {
        async fn embed(&self, _text: &str) -> arawn_llm::Result<Vec<f32>> {
            Ok(vec![0.5; self.dims])
        }

        fn dimensions(&self) -> usize {
            self.dims
        }

        fn name(&self) -> &str {
            "fixed"
        }
    }

    fn create_recall_store(dims: usize) -> Arc<MemoryStore> {
        arawn_memory::vector::init_vector_extension();
        let store = MemoryStore::open_in_memory().unwrap();
        store.init_vectors(dims, "mock").unwrap();
        Arc::new(store)
    }

    #[tokio::test]
    #[serial]
    async fn test_recall_injects_context() {
        let store = create_recall_store(4);

        // Insert a memory with embedding
        let mem = Memory::new(ContentType::Note, "Rust has great memory safety");
        store
            .insert_memory_with_embedding(&mem, &[0.5, 0.5, 0.5, 0.5])
            .unwrap();

        let embedder: SharedEmbedder = Arc::new(FixedEmbedder::new(4));

        let backend = MockBackend::with_text("I recall that Rust has great memory safety.");
        let agent = Agent::builder()
            .with_backend(backend)
            .with_memory_store(store)
            .with_embedder(embedder)
            .with_recall_config(RecallConfig {
                enabled: true,
                threshold: 0.0, // low threshold to ensure match
                limit: 5,
            })
            .build()
            .unwrap();

        let mut session = Session::new();
        let response = agent
            .turn(&mut session, "Tell me about Rust", None)
            .await
            .unwrap();

        // The agent should respond (recall happens silently)
        assert_eq!(response.text, "I recall that Rust has great memory safety.");
    }

    #[tokio::test]
    #[serial]
    async fn test_recall_no_results() {
        let store = create_recall_store(4);
        // Empty store — no memories to recall

        let embedder: SharedEmbedder = Arc::new(FixedEmbedder::new(4));

        let backend = MockBackend::with_text("No memories found.");
        let agent = Agent::builder()
            .with_backend(backend)
            .with_memory_store(store)
            .with_embedder(embedder)
            .with_recall_config(RecallConfig {
                enabled: true,
                threshold: 0.0,
                limit: 5,
            })
            .build()
            .unwrap();

        let mut session = Session::new();
        let response = agent.turn(&mut session, "Hello", None).await.unwrap();
        assert_eq!(response.text, "No memories found.");
    }

    #[tokio::test]
    async fn test_recall_disabled_config() {
        let backend = MockBackend::with_text("Recall disabled.");
        let agent = Agent::builder()
            .with_backend(backend)
            .with_recall_config(RecallConfig {
                enabled: false,
                threshold: 0.6,
                limit: 5,
            })
            .build()
            .unwrap();

        let mut session = Session::new();
        let response = agent.turn(&mut session, "Hello", None).await.unwrap();
        assert_eq!(response.text, "Recall disabled.");
    }

    #[tokio::test]
    async fn test_recall_no_embedder() {
        // No embedder configured — recall should be silently skipped
        let backend = MockBackend::with_text("No embedder.");
        let agent = Agent::builder()
            .with_backend(backend)
            .with_recall_config(RecallConfig {
                enabled: true,
                threshold: 0.6,
                limit: 5,
            })
            .build()
            .unwrap();

        let mut session = Session::new();
        let response = agent.turn(&mut session, "Hello", None).await.unwrap();
        assert_eq!(response.text, "No embedder.");
    }
}

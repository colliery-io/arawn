use std::sync::Arc;

use arawn_core::{Message, Session, Workstream};
use arawn_llm::{MockLlmClient, MockResponse};
use tempfile::TempDir;

use crate::context::ToolContext;
use crate::hooks::HookRunner;
use crate::permissions::PermissionChecker;
use crate::query_engine::{QueryEngine, QueryEngineConfig};
use crate::skills::SkillRegistry;
use crate::tool::{Tool, ToolRegistry};

/// Result from running the test harness.
pub struct HarnessResult {
    pub final_text: String,
    pub session: Session,
}

impl HarnessResult {
    pub fn final_text(&self) -> &str {
        &self.final_text
    }

    pub fn tool_calls(&self) -> Vec<(&str, &serde_json::Value)> {
        self.session
            .messages()
            .iter()
            .filter_map(|msg| match msg {
                Message::Assistant { tool_uses, .. } => {
                    Some(tool_uses.iter().map(|tu| (tu.name.as_str(), &tu.input)))
                }
                _ => None,
            })
            .flatten()
            .collect()
    }

    pub fn session_messages(&self) -> &[Message] {
        self.session.messages()
    }

    pub fn message_count(&self) -> usize {
        self.session.messages().len()
    }
}

/// Builder for assembling a full engine test fixture.
pub struct TestHarness {
    _temp_dir: TempDir,
    workstream: Workstream,
    registry: Arc<ToolRegistry>,
    mock_llm: Arc<MockLlmClient>,
    config: QueryEngineConfig,
    permission_checker: Option<Arc<PermissionChecker>>,
    hook_runner: Option<Arc<HookRunner>>,
    skill_registry: Option<Arc<SkillRegistry>>,
}

/// Builder for constructing a TestHarness.
pub struct TestHarnessBuilder {
    temp_dir: TempDir,
    files: Vec<(String, String)>,
    tools: Vec<Box<dyn Tool>>,
    script: Vec<MockResponse>,
    max_iterations: usize,
    permission_checker: Option<Arc<PermissionChecker>>,
    hook_runner: Option<Arc<HookRunner>>,
    skill_registry: Option<Arc<SkillRegistry>>,
}

impl TestHarnessBuilder {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("failed to create temp dir"),
            files: vec![],
            tools: vec![],
            script: vec![],
            max_iterations: 20,
            permission_checker: None,
            hook_runner: None,
            skill_registry: None,
        }
    }

    /// Pre-populate a file in the workstream directory.
    pub fn with_workstream_file(
        mut self,
        path: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        self.files.push((path.into(), content.into()));
        self
    }

    /// Register a tool in the registry.
    pub fn with_tool(mut self, tool: Box<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Register multiple tools.
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = Box<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Set the scripted LLM responses.
    pub fn with_script(mut self, script: Vec<MockResponse>) -> Self {
        self.script = script;
        self
    }

    /// Set max iterations for the engine.
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Wire a permission checker into the engine.
    pub fn with_permission_checker(mut self, checker: Arc<PermissionChecker>) -> Self {
        self.permission_checker = Some(checker);
        self
    }

    /// Wire a hook runner into the engine.
    pub fn with_hook_runner(mut self, runner: Arc<HookRunner>) -> Self {
        self.hook_runner = Some(runner);
        self
    }

    /// Wire a skill registry into the engine.
    pub fn with_skill_registry(mut self, registry: Arc<SkillRegistry>) -> Self {
        self.skill_registry = Some(registry);
        self
    }

    /// Build the harness.
    pub fn build(self) -> TestHarness {
        // Create files in temp dir
        for (path, content) in &self.files {
            let full_path = self.temp_dir.path().join(path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).expect("failed to create parent dirs");
            }
            std::fs::write(&full_path, content).expect("failed to write file");
        }

        let workstream = Workstream::new("test", self.temp_dir.path());
        let registry = Arc::new(ToolRegistry::new());
        for tool in self.tools {
            registry.register(tool);
        }

        // Auto-register SkillTool when a skill registry is provided
        if let Some(ref skill_reg) = self.skill_registry {
            registry.register(Box::new(crate::tools::SkillTool::new(skill_reg.clone())));
        }

        let mock_llm = Arc::new(MockLlmClient::new(self.script));

        let config = QueryEngineConfig {
            max_iterations: self.max_iterations,
            system_prompt: "Test system prompt".into(),
            ..Default::default()
        };

        TestHarness {
            _temp_dir: self.temp_dir,
            workstream,
            registry,
            mock_llm,
            config,
            permission_checker: self.permission_checker,
            hook_runner: self.hook_runner,
            skill_registry: self.skill_registry,
        }
    }
}

impl Default for TestHarnessBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestHarness {
    pub fn builder() -> TestHarnessBuilder {
        TestHarnessBuilder::new()
    }

    /// Run the engine with the given user input and return results.
    pub async fn run(&self, user_input: impl Into<String>) -> HarnessResult {
        let mut session = Session::new(self.workstream.id);
        let ctx = ToolContext::new(&self.workstream, session.id);

        session.add_message(Message::User {
            content: user_input.into(),
        });

        let mut engine = self.build_engine();

        let final_text = engine
            .run(&mut session, &ctx)
            .await
            .unwrap_or_else(|e| format!("ENGINE_ERROR: {e}"));

        HarnessResult {
            final_text,
            session,
        }
    }

    /// Run expecting an error (e.g., max iterations).
    pub async fn run_expect_error(
        &self,
        user_input: impl Into<String>,
    ) -> crate::error::EngineError {
        let mut session = Session::new(self.workstream.id);
        let ctx = ToolContext::new(&self.workstream, session.id);

        session.add_message(Message::User {
            content: user_input.into(),
        });

        let mut engine = self.build_engine();

        engine
            .run(&mut session, &ctx)
            .await
            .expect_err("expected engine error")
    }

    /// Build a QueryEngine with all configured subsystems wired in.
    fn build_engine(&self) -> QueryEngine {
        let mut engine = QueryEngine::with_config(
            self.mock_llm.clone(),
            self.registry.clone(),
            QueryEngineConfig {
                max_iterations: self.config.max_iterations,
                system_prompt: self.config.system_prompt.clone(),
                ..Default::default()
            },
        );
        if let Some(ref checker) = self.permission_checker {
            engine = engine.with_permission_checker(checker.clone());
        }
        if let Some(ref runner) = self.hook_runner {
            engine = engine.with_hook_runner(runner.clone());
        }
        if let Some(ref skills) = self.skill_registry {
            engine = engine.with_skill_registry(skills.clone());
        }
        engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{FileReadTool, ShellTool, ThinkTool};
    use arawn_llm::MockResponse;

    #[tokio::test]
    async fn harness_text_only() {
        let harness = TestHarness::builder()
            .with_script(vec![MockResponse::text("Hello!")])
            .build();

        let result = harness.run("Hi").await;
        assert_eq!(result.final_text(), "Hello!");
        assert_eq!(result.message_count(), 2); // user + assistant
        assert!(result.tool_calls().is_empty());
    }

    #[tokio::test]
    async fn harness_single_tool_call() {
        let harness = TestHarness::builder()
            .with_workstream_file("notes.txt", "hello world")
            .with_tool(Box::new(FileReadTool))
            .with_script(vec![
                MockResponse::tool_call("call_1", "file_read", r#"{"path":"notes.txt"}"#),
                MockResponse::text("The file contains: hello world"),
            ])
            .build();

        let result = harness.run("What's in notes.txt?").await;
        assert_eq!(result.final_text(), "The file contains: hello world");

        let calls = result.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "file_read");
    }

    #[tokio::test]
    async fn harness_multi_step_tool_chain() {
        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_tool(Box::new(ShellTool::default()))
            .with_script(vec![
                MockResponse::tool_call("c1", "think", r#"{"thought":"planning..."}"#),
                MockResponse::tool_call("c2", "shell", r#"{"command":"echo done"}"#),
                MockResponse::text("All done."),
            ])
            .build();

        let result = harness.run("Do the thing").await;
        assert_eq!(result.final_text(), "All done.");

        let calls = result.tool_calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "think");
        assert_eq!(calls[1].0, "shell");
    }

    #[tokio::test]
    async fn harness_tool_not_found() {
        let harness = TestHarness::builder()
            .with_script(vec![
                MockResponse::tool_call("c1", "nonexistent", "{}"),
                MockResponse::text("Tool wasn't available."),
            ])
            .build();

        let result = harness.run("Use missing tool").await;
        assert_eq!(result.final_text(), "Tool wasn't available.");

        // The tool_result should be an error
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                is_error, content, ..
            } => {
                assert!(is_error);
                assert!(content.contains("not available") || content.contains("not found"));
            }
            _ => panic!("expected ToolResult at index 2"),
        }
    }

    #[tokio::test]
    async fn harness_max_iterations() {
        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_max_iterations(2)
            .with_script(vec![
                MockResponse::tool_call("c1", "think", r#"{"thought":"1"}"#),
                MockResponse::tool_call("c2", "think", r#"{"thought":"2"}"#),
                MockResponse::tool_call("c3", "think", r#"{"thought":"3"}"#),
            ])
            .build();

        let err = harness.run_expect_error("Loop").await;
        match err {
            crate::error::EngineError::MaxIterations(2) => {}
            other => panic!("expected MaxIterations(2), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn harness_shell_tool_receives_arguments() {
        // Verify the shell tool actually receives the "command" parameter
        let harness = TestHarness::builder()
            .with_tool(Box::new(ShellTool::default()))
            .with_script(vec![
                MockResponse::tool_call("c1", "shell", r#"{"command":"echo hello"}"#),
                MockResponse::text("Done"),
            ])
            .build();

        let result = harness.run("Run echo").await;
        assert_eq!(result.final_text(), "Done");

        // The tool_result should contain "hello" (the echo output)
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(!is_error, "tool returned error: {content}");
                assert!(
                    content.contains("hello"),
                    "expected 'hello' in output, got: {content}"
                );
            }
            _ => panic!("expected ToolResult at index 2"),
        }
    }

    #[tokio::test]
    async fn harness_raw_chunks_split_arguments() {
        // Simulate how a real LLM streams tool call arguments in multiple deltas
        use arawn_llm::ChatChunk;

        let harness = TestHarness::builder()
            .with_tool(Box::new(ShellTool::default()))
            .with_script(vec![
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "call_1".into(),
                        name: "shell".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"com"#.into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"mand":"#.into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#""echo split"}"#.into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                MockResponse::text("Got it"),
            ])
            .build();

        let result = harness.run("Split args test").await;
        assert_eq!(result.final_text(), "Got it");

        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(!is_error, "tool returned error: {content}");
                assert!(
                    content.contains("split"),
                    "expected 'split' in output, got: {content}"
                );
            }
            _ => panic!("expected ToolResult at index 2"),
        }
    }

    #[tokio::test]
    async fn harness_tool_arguments_passed_correctly() {
        // Verify the exact arguments object reaches the tool
        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                MockResponse::tool_call("c1", "think", r#"{"thought":"the arguments work"}"#),
                MockResponse::text("Confirmed"),
            ])
            .build();

        let result = harness.run("Test args").await;

        // ThinkTool returns the thought as its output
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(!is_error);
                assert_eq!(content, "the arguments work");
            }
            _ => panic!("expected ToolResult at index 2"),
        }
    }

    #[tokio::test]
    async fn harness_permission_checker_blocks_tool() {
        use crate::permissions::{
            MockModalPrompt, PermissionChecker, PermissionRule, RuleKind,
        };

        let checker = Arc::new(
            PermissionChecker::new(vec![PermissionRule::new(RuleKind::Deny, "shell")])
                .with_prompter(Box::new(MockModalPrompt::always(None))),
        );

        let harness = TestHarness::builder()
            .with_tool(Box::new(ShellTool::default()))
            .with_permission_checker(checker)
            .with_script(vec![
                MockResponse::tool_call("c1", "shell", r#"{"command":"echo hi"}"#),
                MockResponse::text("Tool was denied"),
            ])
            .build();

        let result = harness.run("run echo").await;
        assert_eq!(result.final_text(), "Tool was denied");

        // The tool_result should indicate permission denied
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                is_error, content, ..
            } => {
                assert!(is_error);
                assert!(
                    content.contains("denied") || content.contains("permission"),
                    "expected permission error, got: {content}"
                );
            }
            _ => panic!("expected ToolResult at index 2"),
        }
    }

    #[tokio::test]
    async fn harness_permission_checker_allows_tool() {
        use crate::permissions::{
            MockModalPrompt, PermissionChecker, PermissionRule, RuleKind,
        };

        let checker = Arc::new(
            PermissionChecker::new(vec![PermissionRule::new(RuleKind::Allow, "think")])
                .with_prompter(Box::new(MockModalPrompt::always(None))),
        );

        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_permission_checker(checker)
            .with_script(vec![
                MockResponse::tool_call("c1", "think", r#"{"thought":"planning"}"#),
                MockResponse::text("Done thinking"),
            ])
            .build();

        let result = harness.run("think about it").await;
        assert_eq!(result.final_text(), "Done thinking");

        // Tool should have succeeded (not an error)
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult { is_error, .. } => {
                assert!(!is_error, "tool should have been allowed");
            }
            _ => panic!("expected ToolResult at index 2"),
        }
    }

    #[tokio::test]
    async fn harness_file_read_with_real_filesystem() {
        let harness = TestHarness::builder()
            .with_workstream_file("data/config.toml", "[server]\nport = 8080\n")
            .with_tool(Box::new(FileReadTool))
            .with_script(vec![
                MockResponse::tool_call("c1", "file_read", r#"{"path":"data/config.toml"}"#),
                MockResponse::text("Port is 8080"),
            ])
            .build();

        let result = harness.run("What port?").await;
        assert_eq!(result.final_text(), "Port is 8080");

        // Verify the tool actually read the file
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(!is_error);
                assert!(content.contains("port = 8080"));
            }
            _ => panic!("expected ToolResult"),
        }
    }
}

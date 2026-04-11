use std::sync::Arc;

use arawn_core::{Message, Session, Workstream};
use arawn_llm::{MockLlmClient, MockResponse};
use tempfile::TempDir;

use crate::context::EngineToolContext;
use crate::hooks::HookRunner;
use crate::permissions::PermissionChecker;
use crate::plan::PlanModeState;
use crate::query_engine::{ProgressEvent, QueryEngine, QueryEngineConfig};
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
    plan_state: Option<Arc<PlanModeState>>,
    progress_tx: Option<tokio::sync::mpsc::Sender<ProgressEvent>>,
    progress_rx: std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<ProgressEvent>>>,
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
    plan_active: bool,
    with_progress: bool,
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
            plan_active: false,
            with_progress: false,
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

    /// Enable plan mode on the engine (blocks write tools, allows read-only).
    pub fn with_plan_active(mut self) -> Self {
        self.plan_active = true;
        self
    }

    /// Enable progress event capture. Call `progress_rx()` on the built harness
    /// to get the receiver.
    pub fn with_progress_channel(mut self) -> Self {
        self.with_progress = true;
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

        let plan_state = if self.plan_active {
            let ps = Arc::new(PlanModeState::new());
            ps.enter(
                crate::permissions::PermissionMode::Default,
                "test-plan",
                self.temp_dir.path(),
            )
            .expect("failed to enter plan mode");
            Some(ps)
        } else {
            None
        };

        let (progress_tx, progress_rx) = if self.with_progress {
            let (tx, rx) = tokio::sync::mpsc::channel(256);
            (Some(tx), Some(rx))
        } else {
            (None, None)
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
            plan_state,
            progress_tx,
            progress_rx: std::sync::Mutex::new(progress_rx),
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

    /// Access the underlying mock LLM client for assertions (call_count, captured_requests).
    pub fn mock_llm(&self) -> &Arc<MockLlmClient> {
        &self.mock_llm
    }

    /// Take the progress event receiver. Can only be called once.
    pub fn take_progress_rx(&self) -> Option<tokio::sync::mpsc::Receiver<ProgressEvent>> {
        self.progress_rx.lock().unwrap().take()
    }

    /// Run the engine with the given user input and return results.
    pub async fn run(&self, user_input: impl Into<String>) -> HarnessResult {
        let mut session = Session::new(self.workstream.id);
        let ctx = EngineToolContext::new(&self.workstream, session.id);

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
        let ctx = EngineToolContext::new(&self.workstream, session.id);

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
        if let Some(ref plan_state) = self.plan_state {
            engine = engine.with_plan_state(plan_state.clone());
        }
        if let Some(ref tx) = self.progress_tx {
            engine = engine.with_progress_sender(tx.clone());
        }
        engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{FileReadTool, ShellTool, ThinkTool};
    use arawn_llm::{ChatChunk, MockResponse};

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
            crate::error::EngineError::MaxIterations { iterations: 2, .. } => {}
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

    #[tokio::test]
    async fn harness_parallel_tool_calls_in_single_turn() {
        use arawn_llm::ChatChunk;

        let harness = TestHarness::builder()
            .with_workstream_file("a.txt", "content A")
            .with_workstream_file("b.txt", "content B")
            .with_tool(Box::new(FileReadTool))
            .with_script(vec![
                // Two tool calls in a single LLM response
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "file_read".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"path":"a.txt"}"#.into(),
                    },
                    ChatChunk::ToolUseStart {
                        id: "c2".into(),
                        name: "file_read".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"path":"b.txt"}"#.into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                MockResponse::text("Both files read."),
            ])
            .build();

        let result = harness.run("Read both files").await;
        assert_eq!(result.final_text(), "Both files read.");

        let calls = result.tool_calls();
        assert_eq!(calls.len(), 2, "expected 2 tool calls, got {}", calls.len());
        assert_eq!(calls[0].0, "file_read");
        assert_eq!(calls[1].0, "file_read");

        // Verify both tool results are present and contain correct content
        let msgs = result.session_messages();
        // Messages: User, Assistant(2 tool_uses), ToolResult(a), ToolResult(b), Assistant(text)
        match &msgs[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(!is_error);
                assert!(
                    content.contains("content A"),
                    "first result should contain 'content A', got: {content}"
                );
            }
            other => panic!("expected ToolResult at index 2, got: {other:?}"),
        }
        match &msgs[3] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(!is_error);
                assert!(
                    content.contains("content B"),
                    "second result should contain 'content B', got: {content}"
                );
            }
            other => panic!("expected ToolResult at index 3, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn harness_mixed_text_and_tool_call_in_same_turn() {
        use arawn_llm::ChatChunk;

        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                MockResponse::raw(vec![
                    ChatChunk::TextDelta {
                        text: "Let me think about this.".into(),
                    },
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"thought":"planning next step"}"#.into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                MockResponse::text("Done."),
            ])
            .build();

        let result = harness.run("Think about it").await;
        assert_eq!(result.final_text(), "Done.");

        // The assistant message at index 1 should have both text content and tool_uses
        let msgs = result.session_messages();
        match &msgs[1] {
            Message::Assistant {
                content,
                tool_uses,
            } => {
                assert_eq!(content, "Let me think about this.");
                assert_eq!(tool_uses.len(), 1);
                assert_eq!(tool_uses[0].name, "think");
            }
            other => panic!("expected Assistant at index 1, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn harness_stream_without_done_chunk() {
        use arawn_llm::ChatChunk;

        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                // No Done chunk — relies on flush path
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"thought":"no done"}"#.into(),
                    },
                ]),
                MockResponse::text("Flushed ok."),
            ])
            .build();

        let result = harness.run("Flush test").await;
        assert_eq!(result.final_text(), "Flushed ok.");

        let calls = result.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "think");

        // Verify tool actually ran and produced a result
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(!is_error);
                assert_eq!(content, "no done");
            }
            other => panic!("expected ToolResult at index 2, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn harness_empty_stream_done_only() {
        use arawn_llm::ChatChunk;

        let harness = TestHarness::builder()
            .with_script(vec![MockResponse::raw(vec![ChatChunk::Done {
                usage: None,
            }])])
            .build();

        let result = harness.run("Empty stream").await;
        // No tool calls, empty text — engine returns empty string
        assert_eq!(result.final_text(), "");
        assert!(result.tool_calls().is_empty());
    }

    #[tokio::test]
    async fn harness_empty_text_deltas_assembled_correctly() {
        use arawn_llm::ChatChunk;

        let harness = TestHarness::builder()
            .with_script(vec![MockResponse::raw(vec![
                ChatChunk::TextDelta { text: "".into() },
                ChatChunk::TextDelta {
                    text: "hello".into(),
                },
                ChatChunk::TextDelta { text: "".into() },
                ChatChunk::TextDelta {
                    text: " world".into(),
                },
                ChatChunk::Done { usage: None },
            ])])
            .build();

        let result = harness.run("Empty deltas").await;
        assert_eq!(result.final_text(), "hello world");
    }

    #[tokio::test]
    async fn harness_text_after_tool_start_both_captured() {
        use arawn_llm::ChatChunk;

        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"thought":"deep thought"}"#.into(),
                    },
                    // Text appearing after tool start
                    ChatChunk::TextDelta {
                        text: "narration after tool".into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                MockResponse::text("Final."),
            ])
            .build();

        let result = harness.run("Interleaved").await;
        assert_eq!(result.final_text(), "Final.");

        // Tool should have been captured
        let calls = result.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "think");

        // The assistant message should have both narration text and tool uses
        let msgs = result.session_messages();
        match &msgs[1] {
            Message::Assistant {
                content,
                tool_uses,
            } => {
                assert!(
                    content.contains("narration after tool"),
                    "expected narration text, got: {content}"
                );
                assert_eq!(tool_uses.len(), 1);
            }
            other => panic!("expected Assistant at index 1, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn harness_malformed_json_args_falls_back_to_empty_object() {
        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: "{{not valid json!!!".into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                MockResponse::text("Handled."),
            ])
            .build();

        let result = harness.run("Malformed args").await;
        assert_eq!(result.final_text(), "Handled.");

        // parse_arguments falls back to {} — the tool gets empty args
        // ThinkTool requires "thought" field, so it may error or return empty
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult { .. } => {
                // Tool was invoked (not rejected at validation) — that's the key behavior
            }
            other => panic!("expected ToolResult at index 2, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn harness_non_object_json_args_rejected() {
        // Array args — valid JSON but not an object
        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"[1,2,3]"#.into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                MockResponse::text("Rejected."),
            ])
            .build();

        let result = harness.run("Array args").await;
        assert_eq!(result.final_text(), "Rejected.");

        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(is_error, "expected error for non-object args");
                assert!(
                    content.contains("expected a JSON object"),
                    "expected 'expected a JSON object', got: {content}"
                );
            }
            other => panic!("expected ToolResult at index 2, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn harness_string_json_args_rejected() {
        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#""just a string""#.into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                MockResponse::text("Rejected."),
            ])
            .build();

        let result = harness.run("String args").await;
        assert_eq!(result.final_text(), "Rejected.");

        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(is_error, "expected error for string args");
                assert!(content.contains("expected a JSON object"));
            }
            other => panic!("expected ToolResult at index 2, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn harness_empty_tool_args_no_delta() {
        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    // No ToolUseInputDelta — args will be empty string -> {}
                    ChatChunk::Done { usage: None },
                ]),
                MockResponse::text("Empty args handled."),
            ])
            .build();

        let result = harness.run("No delta").await;
        assert_eq!(result.final_text(), "Empty args handled.");

        // Tool should have been invoked with {} (parse_arguments("") returns {})
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult { .. } => {
                // Tool was invoked — key behavior confirmed
            }
            other => panic!("expected ToolResult at index 2, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn harness_repeated_failure_circuit_breaker() {
        // file_read with nonexistent file will fail with is_error=true
        let harness = TestHarness::builder()
            .with_tool(Box::new(FileReadTool))
            .with_max_iterations(10)
            .with_script(vec![
                // 1st call — will execute and fail (file not found)
                MockResponse::tool_call(
                    "c1",
                    "file_read",
                    r#"{"path":"nonexistent.txt"}"#,
                ),
                // 2nd identical call — will execute and fail again
                MockResponse::tool_call(
                    "c2",
                    "file_read",
                    r#"{"path":"nonexistent.txt"}"#,
                ),
                // 3rd identical call — should be blocked by circuit breaker
                MockResponse::tool_call(
                    "c3",
                    "file_read",
                    r#"{"path":"nonexistent.txt"}"#,
                ),
                MockResponse::text("Gave up."),
            ])
            .build();

        let result = harness.run("Repeated failure").await;
        assert_eq!(result.final_text(), "Gave up.");

        // Find all ToolResult messages
        let tool_results: Vec<&Message> = result
            .session_messages()
            .iter()
            .filter(|m| matches!(m, Message::ToolResult { .. }))
            .collect();

        assert_eq!(
            tool_results.len(),
            3,
            "expected 3 tool results (2 real failures + 1 circuit breaker)"
        );

        // Third result should be the circuit breaker message
        match tool_results[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(is_error);
                assert!(
                    content.contains("already failed") || content.contains("failed 2 times"),
                    "expected circuit breaker message, got: {content}"
                );
            }
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn harness_empty_text_response_returns_cleanly() {
        let harness = TestHarness::builder()
            .with_script(vec![MockResponse::raw(vec![
                ChatChunk::TextDelta { text: "".into() },
                ChatChunk::Done { usage: None },
            ])])
            .build();

        let result = harness.run("Empty text").await;
        assert_eq!(result.final_text(), "");
        assert!(result.tool_calls().is_empty());
        assert_eq!(result.message_count(), 2); // user + assistant
    }

    #[tokio::test]
    async fn harness_token_usage_accumulation() {
        use arawn_llm::Usage;

        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"thought":"planning"}"#.into(),
                    },
                    ChatChunk::Done {
                        usage: Some(Usage {
                            input_tokens: 100,
                            output_tokens: 50,
                        }),
                    },
                ]),
                MockResponse::raw(vec![
                    ChatChunk::TextDelta {
                        text: "Done.".into(),
                    },
                    ChatChunk::Done {
                        usage: Some(Usage {
                            input_tokens: 200,
                            output_tokens: 80,
                        }),
                    },
                ]),
            ])
            .build();

        let result = harness.run("Token test").await;
        assert_eq!(result.final_text(), "Done.");

        // Verify accumulated stats across 2 turns
        assert_eq!(result.session.stats.input_tokens, 300); // 100 + 200
        assert_eq!(result.session.stats.output_tokens, 130); // 50 + 80
        assert_eq!(result.session.stats.turns, 2);
        assert_eq!(result.session.stats.tool_calls, 1); // 1 tool call in first turn
    }

    #[tokio::test]
    async fn harness_fatal_llm_error_no_retry() {
        use arawn_llm::LlmError;

        let harness = TestHarness::builder()
            .with_script(vec![MockResponse::error(LlmError::Auth(
                "invalid API key".into(),
            ))])
            .build();

        let err = harness.run_expect_error("Fatal error").await;
        match err {
            crate::error::EngineError::Llm(ref e) => {
                assert!(
                    e.to_string().contains("invalid API key"),
                    "expected auth error, got: {e}"
                );
            }
            other => panic!("expected EngineError::Llm, got: {other:?}"),
        }

        // Should have been called exactly once — no retries for auth errors
        assert_eq!(harness.mock_llm().call_count(), 1);
    }

    #[tokio::test]
    async fn harness_transient_error_then_success() {
        use arawn_llm::LlmError;

        let harness = TestHarness::builder()
            .with_script(vec![
                // First call: transient error (contains "rate")
                MockResponse::error(LlmError::RateLimited("slow down".into())),
                // Second call: success
                MockResponse::text("recovered after retry"),
            ])
            .build();

        let result = harness.run("Retry test").await;
        assert_eq!(result.final_text(), "recovered after retry");

        // Should have been called twice: 1 failure + 1 success
        assert_eq!(harness.mock_llm().call_count(), 2);
    }

    #[tokio::test]
    async fn harness_transient_error_exhausts_retries() {
        use arawn_llm::LlmError;

        let harness = TestHarness::builder()
            .with_script(vec![
                // 3 transient errors (MAX_RETRIES=2, so attempts 0,1,2)
                MockResponse::error(LlmError::RateLimited("rate limit 1".into())),
                MockResponse::error(LlmError::RateLimited("rate limit 2".into())),
                MockResponse::error(LlmError::RateLimited("rate limit 3".into())),
            ])
            .build();

        let err = harness.run_expect_error("Exhaust retries").await;
        match err {
            crate::error::EngineError::Llm(ref e) => {
                assert!(e.to_string().contains("rate limit"));
            }
            other => panic!("expected EngineError::Llm, got: {other:?}"),
        }

        // Should have tried 3 times total (0, 1, 2)
        assert_eq!(harness.mock_llm().call_count(), 3);
    }

    #[tokio::test]
    async fn harness_mid_stream_error_during_text() {
        use arawn_llm::LlmError;

        // Note: "connection" is a transient keyword, so stream errors with that word
        // will be retried. Use a non-transient error message to test mid-stream behavior.
        let harness = TestHarness::builder()
            .with_script(vec![MockResponse::stream_error(
                vec![ChatChunk::TextDelta {
                    text: "partial output".into(),
                }],
                LlmError::Api("unexpected end of response".into()),
            )])
            .build();

        let err = harness.run_expect_error("Mid-stream error").await;
        match err {
            crate::error::EngineError::Llm(ref e) => {
                assert!(
                    e.to_string().contains("unexpected end"),
                    "expected API error, got: {e}"
                );
            }
            other => panic!("expected EngineError::Llm, got: {other:?}"),
        }

        // Non-transient error — should not retry
        assert_eq!(harness.mock_llm().call_count(), 1);
    }

    #[tokio::test]
    async fn harness_mid_stream_error_during_tool_call() {
        use arawn_llm::LlmError;

        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![MockResponse::stream_error(
                vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"thought":"partial"}"#.into(),
                    },
                ],
                LlmError::Api("truncated response".into()),
            )])
            .build();

        let err = harness.run_expect_error("Tool mid-stream error").await;
        match err {
            crate::error::EngineError::Llm(ref e) => {
                assert!(e.to_string().contains("truncated response"));
            }
            other => panic!("expected EngineError::Llm, got: {other:?}"),
        }

        // No partial tool should have been executed
        assert_eq!(harness.mock_llm().call_count(), 1);
    }

    #[tokio::test]
    async fn harness_server_error_is_transient() {
        use arawn_llm::LlmError;

        let harness = TestHarness::builder()
            .with_script(vec![
                // Server 500 error — should be transient (contains "500")
                MockResponse::error(LlmError::ServerError("HTTP 500: internal error".into())),
                MockResponse::text("recovered from 500"),
            ])
            .build();

        let result = harness.run("Server error test").await;
        assert_eq!(result.final_text(), "recovered from 500");
        assert_eq!(harness.mock_llm().call_count(), 2);
    }

    #[tokio::test]
    async fn harness_model_not_found_is_not_transient() {
        use arawn_llm::LlmError;

        let harness = TestHarness::builder()
            .with_script(vec![MockResponse::error(LlmError::ModelNotFound(
                "gpt-99 does not exist".into(),
            ))])
            .build();

        let err = harness.run_expect_error("Model not found").await;
        match err {
            crate::error::EngineError::Llm(_) => {}
            other => panic!("expected EngineError::Llm, got: {other:?}"),
        }

        // No retry for model not found
        assert_eq!(harness.mock_llm().call_count(), 1);
    }

    #[tokio::test]
    async fn harness_permission_denial_then_llm_recovery() {
        use crate::permissions::{
            MockModalPrompt, PermissionChecker, PermissionRule, RuleKind,
        };

        // Deny shell, allow think
        let checker = Arc::new(
            PermissionChecker::new(vec![
                PermissionRule::new(RuleKind::Deny, "shell"),
                PermissionRule::new(RuleKind::Allow, "think"),
            ])
            .with_prompter(Box::new(MockModalPrompt::always(None))),
        );

        let harness = TestHarness::builder()
            .with_tool(Box::new(ShellTool::default()))
            .with_tool(Box::new(ThinkTool))
            .with_permission_checker(checker)
            .with_script(vec![
                // LLM tries shell first — will be denied
                MockResponse::tool_call("c1", "shell", r#"{"command":"echo hi"}"#),
                // LLM sees the denial and pivots to think
                MockResponse::tool_call("c2", "think", r#"{"thought":"shell denied, using think"}"#),
                // LLM produces final answer
                MockResponse::text("Recovered using alternative approach."),
            ])
            .build();

        let result = harness.run("Try shell then recover").await;
        assert_eq!(result.final_text(), "Recovered using alternative approach.");

        // Verify: shell denied, think succeeded
        let tool_results: Vec<(&str, bool)> = result
            .session_messages()
            .iter()
            .filter_map(|m| match m {
                Message::ToolResult {
                    content, is_error, ..
                } => Some((content.as_str(), *is_error)),
                _ => None,
            })
            .collect();

        assert_eq!(tool_results.len(), 2);
        assert!(tool_results[0].1, "first tool result should be error (denied)");
        assert!(
            tool_results[0].0.contains("denied") || tool_results[0].0.contains("Permission"),
            "first result should mention denial"
        );
        assert!(!tool_results[1].1, "second tool result should be success");
    }

    #[tokio::test]
    async fn harness_plan_mode_blocks_write_tool() {
        let harness = TestHarness::builder()
            .with_tool(Box::new(ShellTool::default()))
            .with_tool(Box::new(ThinkTool))
            .with_plan_active()
            .with_script(vec![
                // LLM tries shell (write tool) — blocked by plan mode
                MockResponse::tool_call("c1", "shell", r#"{"command":"echo hi"}"#),
                MockResponse::text("Plan mode blocked it."),
            ])
            .build();

        let result = harness.run("Shell in plan mode").await;
        assert_eq!(result.final_text(), "Plan mode blocked it.");

        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(is_error, "shell should be blocked in plan mode");
                assert!(
                    content.contains("Plan mode is active"),
                    "expected plan mode message, got: {content}"
                );
            }
            other => panic!("expected ToolResult at index 2, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn harness_plan_mode_allows_read_only_tool() {
        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_plan_active()
            .with_script(vec![
                // ThinkTool is read-only — should be allowed
                MockResponse::tool_call("c1", "think", r#"{"thought":"planning step"}"#),
                MockResponse::text("Plan step recorded."),
            ])
            .build();

        let result = harness.run("Think in plan mode").await;
        assert_eq!(result.final_text(), "Plan step recorded.");

        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(!is_error, "think should be allowed in plan mode");
                assert_eq!(content, "planning step");
            }
            other => panic!("expected ToolResult at index 2, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn harness_hook_and_permission_both_wired() {
        use crate::hooks::{HookConfig, HookRunner};
        use crate::permissions::{
            MockModalPrompt, PermissionChecker, PermissionRule, RuleKind,
        };

        let tmp_dir = TempDir::new().unwrap();

        // Permission: allow think
        let checker = Arc::new(
            PermissionChecker::new(vec![PermissionRule::new(RuleKind::Allow, "think")])
                .with_prompter(Box::new(MockModalPrompt::always(None))),
        );

        // Hook: PreToolUse blocking hook on think (exit 2 = block)
        let config: HookConfig = serde_json::from_value(serde_json::json!({
            "PreToolUse": [{
                "matcher": "think",
                "hooks": [{"type": "command", "command": "exit 2"}]
            }]
        }))
        .unwrap();
        let runner = Arc::new(HookRunner::new(config, tmp_dir.path().to_path_buf()));

        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_permission_checker(checker)
            .with_hook_runner(runner)
            .with_script(vec![
                // Permission allows, but hook blocks
                MockResponse::tool_call("c1", "think", r#"{"thought":"test"}"#),
                MockResponse::text("Hook blocked it."),
            ])
            .build();

        let result = harness.run("Hook vs permission").await;
        assert_eq!(result.final_text(), "Hook blocked it.");

        // The tool result should be an error from the hook blocking
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                is_error, content, ..
            } => {
                assert!(
                    is_error,
                    "expected hook to block even though permission allows: {content}"
                );
                assert!(
                    content.contains("Hook blocked") || content.contains("blocked"),
                    "expected hook block message, got: {content}"
                );
            }
            other => panic!("expected ToolResult at index 2, got: {other:?}"),
        }
    }

    // ── Complex multi-turn scenarios ─────────────────────────────────────

    #[tokio::test]
    async fn harness_long_tool_chain_five_steps() {
        let harness = TestHarness::builder()
            .with_workstream_file("config.toml", "port = 8080")
            .with_workstream_file("data.json", r#"{"key": "value"}"#)
            .with_tool(Box::new(ThinkTool))
            .with_tool(Box::new(FileReadTool))
            .with_script(vec![
                MockResponse::tool_call("c1", "think", r#"{"thought":"step 1: plan"}"#),
                MockResponse::tool_call("c2", "file_read", r#"{"path":"config.toml"}"#),
                MockResponse::tool_call("c3", "think", r#"{"thought":"step 3: analyze config"}"#),
                MockResponse::tool_call("c4", "file_read", r#"{"path":"data.json"}"#),
                MockResponse::tool_call("c5", "think", r#"{"thought":"step 5: summarize"}"#),
                MockResponse::text("All 5 steps complete."),
            ])
            .build();

        let result = harness.run("Do a 5-step analysis").await;
        assert_eq!(result.final_text(), "All 5 steps complete.");

        let calls = result.tool_calls();
        assert_eq!(calls.len(), 5, "expected 5 tool calls, got {}", calls.len());
        assert_eq!(calls[0].0, "think");
        assert_eq!(calls[1].0, "file_read");
        assert_eq!(calls[2].0, "think");
        assert_eq!(calls[3].0, "file_read");
        assert_eq!(calls[4].0, "think");

        // Messages: user + 5*(assistant + tool_result) + final_assistant = 12
        assert_eq!(result.message_count(), 12);

        // Verify file reads returned real content
        let msgs = result.session_messages();
        match &msgs[4] {
            Message::ToolResult { content, .. } => {
                assert!(content.contains("port = 8080"));
            }
            _ => panic!("expected ToolResult for config.toml read"),
        }
        match &msgs[8] {
            Message::ToolResult { content, .. } => {
                assert!(content.contains(r#""key": "value""#));
            }
            _ => panic!("expected ToolResult for data.json read"),
        }
    }

    #[tokio::test]
    async fn harness_tool_error_recovery_mid_chain() {
        let harness = TestHarness::builder()
            .with_workstream_file("real.txt", "real content here")
            .with_tool(Box::new(FileReadTool))
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                // 1. Try to read nonexistent file — will fail
                MockResponse::tool_call("c1", "file_read", r#"{"path":"missing.txt"}"#),
                // 2. LLM analyzes the error
                MockResponse::tool_call("c2", "think", r#"{"thought":"file not found, try real.txt"}"#),
                // 3. Read the correct file
                MockResponse::tool_call("c3", "file_read", r#"{"path":"real.txt"}"#),
                // 4. Final response
                MockResponse::text("Found it: real content here"),
            ])
            .build();

        let result = harness.run("Read the file").await;
        assert_eq!(result.final_text(), "Found it: real content here");

        let msgs = result.session_messages();

        // First tool result: error (file not found)
        match &msgs[2] {
            Message::ToolResult { is_error, .. } => assert!(is_error, "first read should fail"),
            _ => panic!("expected ToolResult at 2"),
        }

        // Second tool result: think succeeds
        match &msgs[4] {
            Message::ToolResult {
                is_error, content, ..
            } => {
                assert!(!is_error);
                assert!(content.contains("file not found"));
            }
            _ => panic!("expected ToolResult at 4"),
        }

        // Third tool result: real file read succeeds
        match &msgs[6] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(!is_error);
                assert!(content.contains("real content here"));
            }
            _ => panic!("expected ToolResult at 6"),
        }
    }

    #[tokio::test]
    async fn harness_parallel_reads_then_sequential_think() {
        let harness = TestHarness::builder()
            .with_workstream_file("a.txt", "alpha")
            .with_workstream_file("b.txt", "bravo")
            .with_tool(Box::new(FileReadTool))
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                // Turn 1: 2 parallel file reads
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "file_read".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"path":"a.txt"}"#.into(),
                    },
                    ChatChunk::ToolUseStart {
                        id: "c2".into(),
                        name: "file_read".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"path":"b.txt"}"#.into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                // Turn 2: think about results
                MockResponse::tool_call("c3", "think", r#"{"thought":"a=alpha, b=bravo"}"#),
                // Turn 3: final
                MockResponse::text("Files analyzed: alpha and bravo."),
            ])
            .build();

        let result = harness.run("Read and analyze").await;
        assert_eq!(result.final_text(), "Files analyzed: alpha and bravo.");

        let calls = result.tool_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "file_read");
        assert_eq!(calls[1].0, "file_read");
        assert_eq!(calls[2].0, "think");

        // Verify parallel reads got correct content
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult { content, .. } => assert!(content.contains("alpha")),
            _ => panic!("expected ToolResult at 2"),
        }
        match &msgs[3] {
            Message::ToolResult { content, .. } => assert!(content.contains("bravo")),
            _ => panic!("expected ToolResult at 3"),
        }
    }

    #[tokio::test]
    async fn harness_narration_text_across_multiple_tool_turns() {
        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                // Turn 1: narration + tool
                MockResponse::raw(vec![
                    ChatChunk::TextDelta {
                        text: "First, let me plan.".into(),
                    },
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"thought":"step 1"}"#.into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                // Turn 2: narration + tool
                MockResponse::raw(vec![
                    ChatChunk::TextDelta {
                        text: "Now analyzing.".into(),
                    },
                    ChatChunk::ToolUseStart {
                        id: "c2".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"thought":"step 2"}"#.into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                // Turn 3: narration + tool
                MockResponse::raw(vec![
                    ChatChunk::TextDelta {
                        text: "Almost done.".into(),
                    },
                    ChatChunk::ToolUseStart {
                        id: "c3".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"thought":"step 3"}"#.into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                // Final text
                MockResponse::text("All done with narration."),
            ])
            .build();

        let result = harness.run("Narrated chain").await;
        assert_eq!(result.final_text(), "All done with narration.");

        // Verify all 3 assistant messages have both text and tool_uses
        let msgs = result.session_messages();
        let narrations = ["First, let me plan.", "Now analyzing.", "Almost done."];
        for (i, expected_text) in narrations.iter().enumerate() {
            let msg_idx = 1 + i * 2; // indices 1, 3, 5
            match &msgs[msg_idx] {
                Message::Assistant {
                    content,
                    tool_uses,
                } => {
                    assert_eq!(
                        content, expected_text,
                        "turn {} narration mismatch",
                        i + 1
                    );
                    assert_eq!(
                        tool_uses.len(),
                        1,
                        "turn {} should have 1 tool use",
                        i + 1
                    );
                }
                other => panic!("expected Assistant at index {msg_idx}, got: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn harness_retry_recovery_mid_conversation() {
        use arawn_llm::LlmError;

        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                // Turn 1: think succeeds
                MockResponse::tool_call("c1", "think", r#"{"thought":"initial analysis"}"#),
                // Turn 2: LLM call fails with rate limit (transient)
                MockResponse::error(LlmError::RateLimited("slow down".into())),
                // Turn 2 retry: succeeds with final text
                MockResponse::text("Recovered after rate limit."),
            ])
            .build();

        let result = harness.run("Retry mid-conversation").await;
        assert_eq!(result.final_text(), "Recovered after rate limit.");

        // Verify the think tool ran
        let calls = result.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "think");

        // 3 LLM calls: 1 for think, 1 failed, 1 retry success
        assert_eq!(harness.mock_llm().call_count(), 3);

        // Session should have: user, assistant(think), tool_result, assistant(text)
        assert_eq!(result.message_count(), 4);
    }

    #[tokio::test]
    async fn harness_large_argument_reassembly_many_deltas() {
        let harness = TestHarness::builder()
            .with_tool(Box::new(ThinkTool))
            .with_script(vec![
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    // Split the argument JSON across 6 deltas
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"th"#.into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"ou"#.into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"gh"#.into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"t":"#.into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#""reassembled"#.into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#" correctly"}"#.into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                MockResponse::text("Args reassembled."),
            ])
            .build();

        let result = harness.run("Split args").await;
        assert_eq!(result.final_text(), "Args reassembled.");

        // ThinkTool returns the thought as content
        let msgs = result.session_messages();
        match &msgs[2] {
            Message::ToolResult {
                content, is_error, ..
            } => {
                assert!(!is_error);
                assert_eq!(content, "reassembled correctly");
            }
            _ => panic!("expected ToolResult at 2"),
        }
    }

    #[tokio::test]
    async fn harness_alternating_success_and_failure_chain() {
        let harness = TestHarness::builder()
            .with_workstream_file("exists.txt", "hello")
            .with_tool(Box::new(ThinkTool))
            .with_tool(Box::new(FileReadTool))
            .with_script(vec![
                // 1. think — success
                MockResponse::tool_call("c1", "think", r#"{"thought":"planning"}"#),
                // 2. file_read nonexistent — error
                MockResponse::tool_call("c2", "file_read", r#"{"path":"nope.txt"}"#),
                // 3. think — success
                MockResponse::tool_call("c3", "think", r#"{"thought":"error noted"}"#),
                // 4. file_read existing — success
                MockResponse::tool_call("c4", "file_read", r#"{"path":"exists.txt"}"#),
                MockResponse::text("Alternating pattern complete."),
            ])
            .build();

        let result = harness.run("Alternating").await;
        assert_eq!(result.final_text(), "Alternating pattern complete.");

        let msgs = result.session_messages();
        let tool_results: Vec<bool> = msgs
            .iter()
            .filter_map(|m| match m {
                Message::ToolResult { is_error, .. } => Some(*is_error),
                _ => None,
            })
            .collect();

        assert_eq!(tool_results, vec![false, true, false, false]);
    }

    #[tokio::test]
    async fn harness_permission_denial_cascade_then_success() {
        use crate::permissions::{
            MockModalPrompt, PermissionChecker, PermissionRule, RuleKind,
        };

        let checker = Arc::new(
            PermissionChecker::new(vec![
                PermissionRule::new(RuleKind::Deny, "shell"),
                PermissionRule::new(RuleKind::Allow, "think"),
            ])
            .with_prompter(Box::new(MockModalPrompt::always(None))),
        );

        let harness = TestHarness::builder()
            .with_tool(Box::new(ShellTool::default()))
            .with_tool(Box::new(ThinkTool))
            .with_permission_checker(checker)
            .with_script(vec![
                // LLM tries shell 3 times with different args
                MockResponse::tool_call("c1", "shell", r#"{"command":"ls"}"#),
                MockResponse::tool_call("c2", "shell", r#"{"command":"pwd"}"#),
                MockResponse::tool_call("c3", "shell", r#"{"command":"whoami"}"#),
                // Gives up on shell, uses think
                MockResponse::tool_call("c4", "think", r#"{"thought":"shell keeps failing"}"#),
                MockResponse::text("Used think instead after 3 denials."),
            ])
            .build();

        let result = harness.run("Permission cascade").await;
        assert_eq!(result.final_text(), "Used think instead after 3 denials.");

        let msgs = result.session_messages();
        let tool_results: Vec<(bool, bool)> = msgs
            .iter()
            .filter_map(|m| match m {
                Message::ToolResult {
                    is_error, content, ..
                } => Some((*is_error, content.contains("denied") || content.contains("Permission"))),
                _ => None,
            })
            .collect();

        // 3 denials (error=true, contains denial msg), then 1 success
        assert_eq!(tool_results.len(), 4);
        assert!(tool_results[0].0 && tool_results[0].1, "1st should be denied");
        assert!(tool_results[1].0 && tool_results[1].1, "2nd should be denied");
        assert!(tool_results[2].0 && tool_results[2].1, "3rd should be denied");
        assert!(!tool_results[3].0, "4th (think) should succeed");
    }

    #[tokio::test]
    async fn harness_plan_mode_parallel_mixed_tools() {
        let harness = TestHarness::builder()
            .with_workstream_file("info.txt", "plan info")
            .with_tool(Box::new(ThinkTool))
            .with_tool(Box::new(FileReadTool))
            .with_tool(Box::new(ShellTool::default()))
            .with_plan_active()
            .with_script(vec![
                // Parallel turn: think (read-only, allowed) + shell (write, blocked)
                MockResponse::raw(vec![
                    ChatChunk::ToolUseStart {
                        id: "c1".into(),
                        name: "think".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"thought":"planning"}"#.into(),
                    },
                    ChatChunk::ToolUseStart {
                        id: "c2".into(),
                        name: "shell".into(),
                    },
                    ChatChunk::ToolUseInputDelta {
                        json: r#"{"command":"echo hi"}"#.into(),
                    },
                    ChatChunk::Done { usage: None },
                ]),
                // LLM acknowledges shell was blocked, reads a file instead
                MockResponse::tool_call("c3", "file_read", r#"{"path":"info.txt"}"#),
                MockResponse::text("Plan complete."),
            ])
            .build();

        let result = harness.run("Plan mode parallel").await;
        assert_eq!(result.final_text(), "Plan complete.");

        let msgs = result.session_messages();
        // First turn has 2 tool results
        let first_results: Vec<(bool, &str)> = msgs[2..=3]
            .iter()
            .filter_map(|m| match m {
                Message::ToolResult {
                    is_error, content, ..
                } => Some((*is_error, content.as_str())),
                _ => None,
            })
            .collect();

        assert_eq!(first_results.len(), 2);
        // think should succeed
        assert!(
            !first_results[0].0,
            "think should succeed in plan mode, got error: {}",
            first_results[0].1
        );
        // shell should be blocked by plan mode
        assert!(
            first_results[1].0,
            "shell should be blocked in plan mode"
        );
        assert!(
            first_results[1].1.contains("Plan mode"),
            "error should mention plan mode, got: {}",
            first_results[1].1
        );
    }
}

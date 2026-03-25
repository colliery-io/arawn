//! Facade safety-net tests for arawn-agent.
//!
//! These tests verify that every publicly re-exported type is accessible
//! through the crate's public API. After the arawn-agent crate split,
//! these tests run against the facade and catch any missing re-exports.
//!
//! If a test here fails after a refactoring, it means a downstream consumer
//! would also fail to compile.

// ─────────────────────────────────────────────────────────────────────────────
// Re-export surface tests — verify every public type is accessible
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn reexport_core_types() {
    // Verify core type re-exports compile
    let _: fn() -> arawn_agent::SessionId = arawn_agent::SessionId::new;
    let _: fn() -> arawn_agent::TurnId = arawn_agent::TurnId::new;
    let _: fn() -> arawn_agent::Session = arawn_agent::Session::new;

    // AgentConfig, AgentResponse, ResponseUsage are constructible
    let _ = std::mem::size_of::<arawn_agent::AgentConfig>();
    let _ = std::mem::size_of::<arawn_agent::AgentResponse>();
    let _ = std::mem::size_of::<arawn_agent::ResponseUsage>();
    let _ = std::mem::size_of::<arawn_agent::ToolCall>();
    let _ = std::mem::size_of::<arawn_agent::ToolResultRecord>();
    let _ = std::mem::size_of::<arawn_agent::Turn>();
}

#[test]
fn reexport_error_types() {
    let err = arawn_agent::AgentError::tool("test error");
    assert!(err.to_string().contains("test error"));

    let err = arawn_agent::AgentError::session("session error");
    assert!(err.to_string().contains("session error"));

    // Result type alias
    let _: arawn_agent::Result<()> = Ok(());
}

#[test]
fn reexport_tool_framework() {
    // Tool trait is accessible
    let _ = std::mem::size_of::<&dyn arawn_agent::Tool>();

    // ToolContext is constructible
    let ctx = arawn_agent::ToolContext::default();
    assert!(!ctx.is_cancelled());

    // ToolResult variants
    let result = arawn_agent::ToolResult::text("hello");
    assert!(result.is_success());
    assert!(!result.is_error());

    let err_result = arawn_agent::ToolResult::error("failed");
    assert!(err_result.is_error());

    // ToolRegistry
    let registry = arawn_agent::ToolRegistry::new();
    assert_eq!(registry.len(), 0);
}

#[test]
fn reexport_output_config() {
    let config = arawn_agent::OutputConfig::default();
    // Verify config is constructible (field names may vary)
    let _ = format!("{:?}", config);

    // Constants
    assert!(arawn_agent::DEFAULT_MAX_OUTPUT_SIZE > 0);

    // Functions
    let sanitized = arawn_agent::sanitize_output("hello", &config);
    assert!(sanitized.is_ok());
}

#[test]
fn reexport_command_validator() {
    let validator = arawn_agent::CommandValidator::default();
    let result = validator.validate("ls -la");
    assert!(matches!(result, arawn_agent::CommandValidation::Allowed));

    let result = validator.validate("rm -rf /");
    assert!(matches!(result, arawn_agent::CommandValidation::Blocked(_)));
}

#[test]
fn reexport_param_types() {
    // Verify param type sizes (ensures they're importable)
    let _ = std::mem::size_of::<arawn_agent::ShellParams>();
    let _ = std::mem::size_of::<arawn_agent::FileReadParams>();
    let _ = std::mem::size_of::<arawn_agent::FileWriteParams>();
    let _ = std::mem::size_of::<arawn_agent::ThinkParams>();
    let _ = std::mem::size_of::<arawn_agent::WebSearchParams>();
    let _ = std::mem::size_of::<arawn_agent::DelegateParams>();

    // ParamExt trait
    fn _check_param_ext<T: arawn_agent::ParamExt>() {}
}

#[test]
fn reexport_compaction_types() {
    let _ = std::mem::size_of::<arawn_agent::CompactorConfig>();
    let _ = std::mem::size_of::<arawn_agent::CompactionResult>();
    let _ = std::mem::size_of::<arawn_agent::CompactionProgress>();

    let config = arawn_agent::CompactorConfig::default();
    assert!(config.summary_prompt.is_some() || config.summary_prompt.is_none()); // just verify accessible
}

#[test]
fn reexport_orchestrator_types() {
    let _ = std::mem::size_of::<arawn_agent::OrchestratorConfig>();
    let _ = std::mem::size_of::<arawn_agent::OrchestrationResult>();
    let _ = std::mem::size_of::<arawn_agent::OrchestrationMetadata>();
}

#[test]
fn reexport_context_types() {
    let _ = std::mem::size_of::<arawn_agent::ContextStatus>();
    let _ = std::mem::size_of::<arawn_agent::ContextTracker>();
}

#[test]
fn reexport_prompt_types() {
    let builder = arawn_agent::SystemPromptBuilder::new();
    let prompt = builder.build();
    assert!(!prompt.is_empty());

    let _ = std::mem::size_of::<arawn_agent::PromptMode>();
    let _ = std::mem::size_of::<arawn_agent::BootstrapContext>();
}

#[test]
fn reexport_stream_types() {
    let _ = std::mem::size_of::<arawn_agent::StreamChunk>();
}

#[test]
fn indexing_types_accessible() {
    // Indexing types now live in arawn_agent_indexing directly
    let _ = std::mem::size_of::<arawn_agent_indexing::IndexerConfig>();
    let _ = std::mem::size_of::<arawn_agent_indexing::IndexReport>();
    let _ = std::mem::size_of::<arawn_agent_indexing::NerConfig>();
}

#[test]
fn reexport_rlm_types() {
    let _ = std::mem::size_of::<arawn_agent::RlmConfig>();
    assert!(!arawn_agent::RLM_SYSTEM_PROMPT.is_empty());
    assert!(!arawn_agent::DEFAULT_READ_ONLY_TOOLS.is_empty());
}

#[test]
fn reexport_mcp_types() {
    assert!(!arawn_agent::MCP_PREFIX.is_empty());
    assert!(!arawn_agent::NAMESPACE_DELIMITER.is_empty());

    assert!(!arawn_agent::is_mcp_tool("shell"));
    assert!(arawn_agent::is_mcp_tool(&format!(
        "{}server{}tool",
        arawn_agent::MCP_PREFIX,
        arawn_agent::NAMESPACE_DELIMITER
    )));
}

#[test]
fn reexport_fs_gate_types() {
    assert!(!arawn_agent::GATED_TOOLS.is_empty());
    assert!(arawn_agent::is_gated_tool("shell"));
    assert!(arawn_agent::is_gated_tool("file_read"));
    assert!(arawn_agent::is_gated_tool("web_fetch"));
    assert!(!arawn_agent::is_gated_tool("think"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Built-in tool smoke tests — verify each tool is constructible
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn smoke_shell_tool() {
    let tool = arawn_agent_tools::ShellTool::new();
    assert_eq!(arawn_agent::Tool::name(&tool), "shell");
}

#[test]
fn smoke_shell_tool_with_config() {
    let config = arawn_agent_tools::ShellConfig::default();
    let tool = arawn_agent_tools::ShellTool::with_config(config);
    assert_eq!(arawn_agent::Tool::name(&tool), "shell");
}

#[test]
fn smoke_file_read_tool() {
    let tool = arawn_agent_tools::FileReadTool::new();
    assert_eq!(arawn_agent::Tool::name(&tool), "file_read");
}

#[test]
fn smoke_file_write_tool() {
    let tool = arawn_agent_tools::FileWriteTool::new();
    assert_eq!(arawn_agent::Tool::name(&tool), "file_write");
}

#[test]
fn smoke_glob_tool() {
    let tool = arawn_agent_tools::GlobTool::new();
    assert_eq!(arawn_agent::Tool::name(&tool), "glob");
}

#[test]
fn smoke_grep_tool() {
    let tool = arawn_agent_tools::GrepTool::new();
    assert_eq!(arawn_agent::Tool::name(&tool), "grep");
}

#[test]
fn smoke_web_fetch_tool() {
    let tool = arawn_agent_tools::WebFetchTool::new().unwrap();
    assert_eq!(arawn_agent::Tool::name(&tool), "web_fetch");
}

#[test]
fn smoke_web_search_tool() {
    let tool = arawn_agent_tools::WebSearchTool::new().unwrap();
    assert_eq!(arawn_agent::Tool::name(&tool), "web_search");
}

#[test]
fn smoke_think_tool() {
    let _ = std::mem::size_of::<arawn_agent_tools::ThinkTool>();
}

#[test]
fn smoke_note_tool() {
    let tool = arawn_agent_tools::NoteTool::new();
    assert_eq!(arawn_agent::Tool::name(&tool), "note");
}

#[test]
fn smoke_memory_search_tool() {
    let _ = std::mem::size_of::<arawn_agent_tools::MemorySearchTool>();
}

// ─────────────────────────────────────────────────────────────────────────────
// Module path access tests — verify qualified paths work
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn module_path_tool() {
    // arawn-plugin uses arawn_agent::tool::ToolRegistry
    let _ = arawn_agent::tool::ToolRegistry::new();
    let _ = std::mem::size_of::<&dyn arawn_agent::tool::Tool>();
}

#[test]
fn module_path_types() {
    // arawn-plugin uses arawn_agent::types::AgentConfig
    let _ = std::mem::size_of::<arawn_agent::types::AgentConfig>();
    let _ = std::mem::size_of::<arawn_agent::types::Session>();
}

#[test]
fn module_path_error() {
    // arawn-plugin uses arawn_agent::error::Result
    let _: arawn_agent::error::Result<()> = Ok(());
}

#[test]
fn module_path_tools() {
    // Tools now in arawn_agent_tools crate
    let _ = std::mem::size_of::<arawn_agent_tools::ShellTool>();
}

#[test]
fn module_path_prompt() {
    let _ = arawn_agent::prompt::SystemPromptBuilder::new();
}

#[test]
fn module_path_indexing() {
    let _ = std::mem::size_of::<arawn_agent_indexing::SessionIndexer>();
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent builder smoke test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn smoke_agent_builder() {
    use arawn_agent::{Agent, ToolRegistry};
    use arawn_llm::MockBackend;

    let backend = MockBackend::with_text("Hello");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .expect("should build agent");

    // Verify agent was built successfully (prompt may be empty with no config)
    let _ = agent.system_prompt();
}

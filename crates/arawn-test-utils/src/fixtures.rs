//! Factory functions for common test data.

use std::sync::Arc;

use arawn_domain::{Agent, MemoryStore, Session, ToolRegistry};
use arawn_llm::{CompletionResponse, ContentBlock, MockBackend, StopReason, Usage};
use arawn_server::{AppState, ServerConfig};

/// Factory for common test data.
pub struct TestFixtures;

impl TestFixtures {
    /// Create a ServerConfig with sensible test defaults.
    pub fn server_config() -> ServerConfig {
        ServerConfig::new(Some("test-token".to_string()))
            .with_rate_limiting(false)
            .with_request_logging(false)
    }

    /// Create a ServerConfig with no auth (localhost mode).
    pub fn server_config_no_auth() -> ServerConfig {
        ServerConfig::new(None)
            .with_rate_limiting(false)
            .with_request_logging(false)
    }

    /// Create a MockBackend that returns the given text responses.
    pub fn mock_backend(responses: &[&str]) -> MockBackend {
        let completions: Vec<CompletionResponse> = responses
            .iter()
            .enumerate()
            .map(|(i, text)| {
                CompletionResponse::new(
                    format!("mock_msg_{}", i),
                    "mock-model",
                    vec![ContentBlock::Text {
                        text: text.to_string(),
                        cache_control: None,
                    }],
                    StopReason::EndTurn,
                    Usage::new(10, 20),
                )
            })
            .collect();
        MockBackend::new(completions)
    }

    /// Create a simple Agent with MockBackend and no tools.
    pub fn agent(responses: &[&str]) -> Agent {
        Agent::builder()
            .with_backend(Self::mock_backend(responses))
            .with_tools(ToolRegistry::new())
            .build()
            .expect("Failed to build test agent")
    }

    /// Create an Agent with a ToolRegistry.
    pub fn agent_with_tools(responses: &[&str], tools: ToolRegistry) -> Agent {
        Agent::builder()
            .with_backend(Self::mock_backend(responses))
            .with_tools(tools)
            .build()
            .expect("Failed to build test agent")
    }

    /// Create an in-memory MemoryStore.
    pub fn memory_store() -> Arc<MemoryStore> {
        Arc::new(MemoryStore::open_in_memory().expect("Failed to open in-memory store"))
    }

    /// Create a new empty Session.
    pub fn session() -> Session {
        Session::new()
    }

    /// Create an AppState with sensible defaults for testing.
    pub fn app_state(responses: &[&str]) -> AppState {
        let agent = Self::agent(responses);
        let config = Self::server_config();
        let mut state = AppState::new(agent, config);
        state.services.memory_store = Some(Self::memory_store());
        state
    }

    /// Create an AppState with workstreams enabled.
    pub fn app_state_with_workstreams(responses: &[&str]) -> (AppState, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let agent = Self::agent(responses);
        let config = Self::server_config();
        let mut state = AppState::new(agent, config);
        state.services.memory_store = Some(Self::memory_store());

        let ws_config = arawn_workstream::WorkstreamConfig {
            db_path: temp_dir.path().join("workstreams.db"),
            data_dir: temp_dir.path().join("workstreams"),
            session_timeout_minutes: 30,
        };
        if let Ok(mgr) = arawn_workstream::WorkstreamManager::new(&ws_config) {
            state = state.with_workstreams(mgr);
        }

        (state, temp_dir)
    }

    /// Create a CompletionResponse with text content.
    pub fn completion_response(text: &str) -> CompletionResponse {
        CompletionResponse::new(
            "msg_test",
            "mock-model",
            vec![ContentBlock::Text {
                text: text.to_string(),
                cache_control: None,
            }],
            StopReason::EndTurn,
            Usage::new(10, 20),
        )
    }

    /// Create a CompletionResponse with a tool use.
    pub fn tool_use_response(
        tool_name: &str,
        tool_id: &str,
        input: serde_json::Value,
    ) -> CompletionResponse {
        CompletionResponse::new(
            "msg_test",
            "mock-model",
            vec![ContentBlock::ToolUse {
                id: tool_id.to_string(),
                name: tool_name.to_string(),
                input,
                cache_control: None,
            }],
            StopReason::ToolUse,
            Usage::new(10, 20),
        )
    }

    /// Create an in-memory WorkstreamManager with a temp directory.
    pub fn workstream_manager() -> (arawn_workstream::WorkstreamManager, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let ws_config = arawn_workstream::WorkstreamConfig {
            db_path: temp_dir.path().join("workstreams.db"),
            data_dir: temp_dir.path().join("workstreams"),
            session_timeout_minutes: 30,
        };
        let mgr = arawn_workstream::WorkstreamManager::new(&ws_config)
            .expect("Failed to create WorkstreamManager");
        (mgr, temp_dir)
    }
}

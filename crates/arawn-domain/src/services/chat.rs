//! Chat service for conversation orchestration.
//!
//! The chat service coordinates agent execution with session management
//! and workstream persistence.

use std::sync::Arc;

use arawn_agent::{Agent, AgentResponse, Session, SessionId};
use arawn_agent_indexing::SessionIndexer;
use arawn_workstream::{DirectoryManager, WorkstreamManager};
use tracing::{debug, info, warn};

use crate::error::{DomainError, Result};

/// Response from a chat turn.
///
/// # Examples
///
/// ```rust,ignore
/// let response: ChatResponse = chat.send_message(session_id, "Hello", &opts).await?;
/// println!("Response: {}", response.response);
/// println!("Tokens: {} in / {} out", response.input_tokens, response.output_tokens);
/// ```
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// The session ID.
    pub session_id: SessionId,
    /// The agent's response text.
    pub response: String,
    /// Whether the response was truncated (hit max turns).
    pub truncated: bool,
    /// Input tokens used.
    pub input_tokens: u32,
    /// Output tokens generated.
    pub output_tokens: u32,
    /// Tool calls made during the turn.
    pub tool_calls: Vec<ToolCallSummary>,
}

/// Summary of a tool call.
#[derive(Debug, Clone)]
pub struct ToolCallSummary {
    /// Tool call ID.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Whether the call succeeded (based on tool_results).
    pub success: bool,
}

/// Options for executing a turn.
///
/// # Examples
///
/// ```rust,ignore
/// let opts = TurnOptions {
///     max_message_bytes: Some(100_000),
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct TurnOptions {
    /// Maximum message size in bytes.
    pub max_message_bytes: Option<usize>,
}

/// Chat service for conversation orchestration.
#[derive(Clone)]
pub struct ChatService {
    agent: Arc<Agent>,
    workstreams: Option<Arc<WorkstreamManager>>,
    directory_manager: Option<Arc<DirectoryManager>>,
    indexer: Option<Arc<SessionIndexer>>,
}

impl ChatService {
    /// Create a new chat service.
    pub fn new(
        agent: Arc<Agent>,
        workstreams: Option<Arc<WorkstreamManager>>,
        directory_manager: Option<Arc<DirectoryManager>>,
        indexer: Option<Arc<SessionIndexer>>,
    ) -> Self {
        Self {
            agent,
            workstreams,
            directory_manager,
            indexer,
        }
    }

    /// Get the underlying agent.
    pub fn agent(&self) -> &Arc<Agent> {
        &self.agent
    }

    /// Get the workstream manager.
    pub fn workstreams(&self) -> Option<&Arc<WorkstreamManager>> {
        self.workstreams.as_ref()
    }

    /// Get the directory manager.
    pub fn directory_manager(&self) -> Option<&Arc<DirectoryManager>> {
        self.directory_manager.as_ref()
    }

    /// Get the session indexer.
    pub fn indexer(&self) -> Option<&Arc<SessionIndexer>> {
        self.indexer.as_ref()
    }

    /// Execute a chat turn with an existing session.
    ///
    /// This is the core chat operation that:
    /// 1. Executes the agent turn
    /// 2. Returns the response
    ///
    /// Note: Session persistence is handled separately via the workstream manager.
    pub async fn turn(
        &self,
        session: &mut Session,
        message: &str,
        workstream_id: Option<&str>,
    ) -> Result<ChatResponse> {
        let session_id = session.id;

        debug!(session_id = %session_id, message_len = message.len(), "Executing chat turn");

        // Execute the agent turn
        let response = self.agent.turn(session, message, workstream_id).await?;

        // Build response
        let chat_response = self.build_response(session_id, &response);

        debug!(
            session_id = %session_id,
            response_len = chat_response.response.len(),
            tool_calls = chat_response.tool_calls.len(),
            "Chat turn completed"
        );

        Ok(chat_response)
    }

    /// Create a scratch session directory.
    pub fn create_scratch_session(&self, session_id: &str) -> Result<()> {
        if let Some(ref dm) = self.directory_manager {
            dm.create_scratch_session(session_id)
                .map_err(|e| DomainError::Internal(e.to_string()))?;
        }
        Ok(())
    }

    /// Get allowed paths for a session.
    pub fn allowed_paths(
        &self,
        workstream_id: &str,
        session_id: &str,
    ) -> Option<Vec<std::path::PathBuf>> {
        self.directory_manager
            .as_ref()
            .map(|dm| dm.allowed_paths(workstream_id, session_id))
    }

    /// Index a closed session for memory search.
    pub async fn index_session(&self, session_id: &str, session: &Session) {
        if let Some(ref indexer) = self.indexer
            && !session.is_empty()
        {
            let messages = session_to_messages(session);
            let refs = messages_as_refs(&messages);

            let report = indexer.index_session(session_id, &refs).await;
            info!(
                session_id = session_id,
                report = %report,
                "Session indexed"
            );
            if report.has_errors() {
                warn!(
                    session_id = session_id,
                    errors = ?report.errors,
                    "Session indexing completed with errors"
                );
            }
        }
    }

    /// Build a ChatResponse from an AgentResponse.
    fn build_response(&self, session_id: SessionId, response: &AgentResponse) -> ChatResponse {
        // Build tool call summaries from tool_calls and tool_results
        let tool_results_success: std::collections::HashMap<String, bool> = response
            .tool_results
            .iter()
            .map(|tr| (tr.tool_call_id.clone(), tr.success))
            .collect();

        let tool_calls: Vec<ToolCallSummary> = response
            .tool_calls
            .iter()
            .map(|tc| ToolCallSummary {
                id: tc.id.clone(),
                name: tc.name.clone(),
                success: tool_results_success.get(&tc.id).copied().unwrap_or(false),
            })
            .collect();

        ChatResponse {
            session_id,
            response: response.text.clone(),
            truncated: response.truncated,
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
            tool_calls,
        }
    }
}

/// Convert a session's turns into owned `(role, content)` pairs.
fn session_to_messages(session: &Session) -> Vec<(String, String)> {
    let mut messages = Vec::new();
    for turn in session.all_turns() {
        messages.push(("user".to_string(), turn.user_message.clone()));
        if let Some(ref response) = turn.assistant_response {
            messages.push(("assistant".to_string(), response.clone()));
        }
    }
    messages
}

/// Convert owned message pairs to borrowed slices for the indexer API.
fn messages_as_refs(messages: &[(String, String)]) -> Vec<(&str, &str)> {
    messages
        .iter()
        .map(|(r, c)| (r.as_str(), c.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_agent::ToolRegistry;
    use arawn_llm::MockBackend;

    fn create_test_agent() -> Arc<Agent> {
        let backend = MockBackend::with_text("Hello!");
        Arc::new(
            Agent::builder()
                .with_backend(backend)
                .with_tools(ToolRegistry::new())
                .build()
                .expect("failed to create test agent"),
        )
    }

    #[tokio::test]
    async fn test_chat_turn() {
        let agent = create_test_agent();
        let chat = ChatService::new(agent, None, None, None);

        let mut session = Session::new();
        let response = chat.turn(&mut session, "Hello", None).await.unwrap();

        assert_eq!(response.session_id, session.id);
        assert!(!response.response.is_empty());
    }

    #[tokio::test]
    async fn test_chat_turn_token_counts() {
        let agent = create_test_agent();
        let chat = ChatService::new(agent, None, None, None);

        let mut session = Session::new();
        let response = chat.turn(&mut session, "Hi", None).await.unwrap();

        // MockBackend returns deterministic token counts
        assert!(!response.truncated);
    }

    #[tokio::test]
    async fn test_chat_multiple_turns() {
        use arawn_llm::{CompletionResponse, ContentBlock, MockResponse, StopReason, Usage};
        let responses = vec![
            MockResponse::Success(CompletionResponse::new(
                "msg1",
                "mock",
                vec![ContentBlock::Text {
                    text: "First reply".to_string(),
                    cache_control: None,
                }],
                StopReason::EndTurn,
                Usage::new(10, 20),
            )),
            MockResponse::Success(CompletionResponse::new(
                "msg2",
                "mock",
                vec![ContentBlock::Text {
                    text: "Second reply".to_string(),
                    cache_control: None,
                }],
                StopReason::EndTurn,
                Usage::new(10, 20),
            )),
        ];
        let backend = MockBackend::with_results(responses);
        let agent = Arc::new(
            Agent::builder()
                .with_backend(backend)
                .with_tools(ToolRegistry::new())
                .build()
                .unwrap(),
        );
        let chat = ChatService::new(agent, None, None, None);

        let mut session = Session::new();
        let r1 = chat.turn(&mut session, "First", None).await.unwrap();
        let r2 = chat.turn(&mut session, "Second", None).await.unwrap();

        assert_eq!(r1.session_id, r2.session_id);
    }

    #[test]
    fn test_session_to_messages() {
        let mut session = Session::new();
        let turn = session.start_turn("Hello");
        turn.complete("Hi there!");

        let messages = session_to_messages(&session);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], ("user".to_string(), "Hello".to_string()));
        assert_eq!(
            messages[1],
            ("assistant".to_string(), "Hi there!".to_string())
        );
    }

    #[test]
    fn test_session_to_messages_empty() {
        let session = Session::new();
        let messages = session_to_messages(&session);
        assert!(messages.is_empty());
    }

    #[test]
    fn test_session_to_messages_multiple_turns() {
        let mut session = Session::new();

        let t1 = session.start_turn("First");
        t1.complete("Response 1");

        let t2 = session.start_turn("Second");
        t2.complete("Response 2");

        let messages = session_to_messages(&session);
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].0, "user");
        assert_eq!(messages[0].1, "First");
        assert_eq!(messages[1].0, "assistant");
        assert_eq!(messages[1].1, "Response 1");
        assert_eq!(messages[2].0, "user");
        assert_eq!(messages[2].1, "Second");
        assert_eq!(messages[3].0, "assistant");
        assert_eq!(messages[3].1, "Response 2");
    }

    #[test]
    fn test_session_to_messages_incomplete_turn() {
        let mut session = Session::new();
        let _turn = session.start_turn("No response yet");
        // Turn is not completed — no assistant_response

        let messages = session_to_messages(&session);
        // Should have user message but no assistant
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "user");
    }

    #[test]
    fn test_messages_as_refs() {
        let messages = vec![
            ("user".to_string(), "Hello".to_string()),
            ("assistant".to_string(), "Hi".to_string()),
        ];
        let refs = messages_as_refs(&messages);
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0], ("user", "Hello"));
        assert_eq!(refs[1], ("assistant", "Hi"));
    }

    #[test]
    fn test_messages_as_refs_empty() {
        let messages: Vec<(String, String)> = vec![];
        let refs = messages_as_refs(&messages);
        assert!(refs.is_empty());
    }

    #[test]
    fn test_chat_response_fields() {
        let response = ChatResponse {
            session_id: SessionId::new(),
            response: "Hello world".to_string(),
            truncated: false,
            input_tokens: 10,
            output_tokens: 5,
            tool_calls: vec![],
        };
        assert!(!response.truncated);
        assert_eq!(response.input_tokens, 10);
        assert_eq!(response.output_tokens, 5);
        assert!(response.tool_calls.is_empty());
    }

    #[test]
    fn test_chat_response_truncated() {
        let response = ChatResponse {
            session_id: SessionId::new(),
            response: "Partial...".to_string(),
            truncated: true,
            input_tokens: 100,
            output_tokens: 50,
            tool_calls: vec![],
        };
        assert!(response.truncated);
    }

    #[test]
    fn test_chat_response_with_tool_calls() {
        let response = ChatResponse {
            session_id: SessionId::new(),
            response: "Done".to_string(),
            truncated: false,
            input_tokens: 20,
            output_tokens: 10,
            tool_calls: vec![
                ToolCallSummary {
                    id: "tc-1".to_string(),
                    name: "read_file".to_string(),
                    success: true,
                },
                ToolCallSummary {
                    id: "tc-2".to_string(),
                    name: "write_file".to_string(),
                    success: false,
                },
            ],
        };
        assert_eq!(response.tool_calls.len(), 2);
        assert!(response.tool_calls[0].success);
        assert!(!response.tool_calls[1].success);
    }

    #[test]
    fn test_chat_response_clone() {
        let response = ChatResponse {
            session_id: SessionId::new(),
            response: "test".to_string(),
            truncated: false,
            input_tokens: 1,
            output_tokens: 2,
            tool_calls: vec![ToolCallSummary {
                id: "id".to_string(),
                name: "tool".to_string(),
                success: true,
            }],
        };
        let cloned = response.clone();
        assert_eq!(response.response, cloned.response);
        assert_eq!(response.tool_calls.len(), cloned.tool_calls.len());
    }

    #[test]
    fn test_chat_response_debug() {
        let response = ChatResponse {
            session_id: SessionId::new(),
            response: "test".to_string(),
            truncated: false,
            input_tokens: 0,
            output_tokens: 0,
            tool_calls: vec![],
        };
        let debug = format!("{:?}", response);
        assert!(debug.contains("ChatResponse"));
    }

    #[test]
    fn test_tool_call_summary_fields() {
        let summary = ToolCallSummary {
            id: "call-123".to_string(),
            name: "bash".to_string(),
            success: true,
        };
        assert_eq!(summary.id, "call-123");
        assert_eq!(summary.name, "bash");
        assert!(summary.success);
    }

    #[test]
    fn test_tool_call_summary_clone() {
        let summary = ToolCallSummary {
            id: "id".to_string(),
            name: "tool".to_string(),
            success: false,
        };
        let cloned = summary.clone();
        assert_eq!(summary.id, cloned.id);
        assert_eq!(summary.success, cloned.success);
    }

    #[test]
    fn test_tool_call_summary_debug() {
        let summary = ToolCallSummary {
            id: "id".to_string(),
            name: "tool".to_string(),
            success: true,
        };
        let debug = format!("{:?}", summary);
        assert!(debug.contains("ToolCallSummary"));
    }

    #[test]
    fn test_turn_options_default() {
        let opts = TurnOptions::default();
        assert!(opts.max_message_bytes.is_none());
    }

    #[test]
    fn test_turn_options_with_max_bytes() {
        let opts = TurnOptions {
            max_message_bytes: Some(100_000),
        };
        assert_eq!(opts.max_message_bytes, Some(100_000));
    }

    #[test]
    fn test_turn_options_clone() {
        let opts = TurnOptions {
            max_message_bytes: Some(50),
        };
        let cloned = opts.clone();
        assert_eq!(opts.max_message_bytes, cloned.max_message_bytes);
    }

    #[test]
    fn test_turn_options_debug() {
        let opts = TurnOptions::default();
        let debug = format!("{:?}", opts);
        assert!(debug.contains("TurnOptions"));
    }

    #[test]
    fn test_chat_service_accessors_none() {
        let agent = create_test_agent();
        let chat = ChatService::new(agent, None, None, None);

        assert!(chat.workstreams().is_none());
        assert!(chat.directory_manager().is_none());
        assert!(chat.indexer().is_none());
    }

    #[test]
    fn test_chat_service_agent_accessor() {
        let agent = create_test_agent();
        let agent_ptr = Arc::as_ptr(&agent);
        let chat = ChatService::new(agent, None, None, None);

        assert_eq!(Arc::as_ptr(chat.agent()), agent_ptr);
    }

    #[test]
    fn test_chat_service_clone() {
        let agent = create_test_agent();
        let chat = ChatService::new(agent, None, None, None);
        let cloned = chat.clone();

        // Both should point to the same agent
        assert!(Arc::ptr_eq(chat.agent(), cloned.agent()));
    }
}

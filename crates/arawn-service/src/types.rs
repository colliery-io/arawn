use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use arawn_core::Message;

/// Lightweight view of a workstream for API transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkstreamInfo {
    pub id: Uuid,
    pub name: String,
    pub root_dir: PathBuf,
    pub created_at: DateTime<Utc>,
}

/// Lightweight view of a session (metadata only, no messages).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: Uuid,
    pub workstream_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Session with full message history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    pub id: Uuid,
    pub workstream_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub messages: Vec<Message>,
}

/// An option in a modal prompt sent to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalPromptOption {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Streaming event emitted during a conversation turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum EngineEvent {
    /// A chunk of assistant text.
    StreamingText { text: String },

    /// A tool call has started.
    ToolCallStart {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    /// A tool call completed with a result.
    ToolCallResult {
        id: String,
        content: String,
        is_error: bool,
    },

    /// The assistant's turn is complete.
    Complete { final_text: String },

    /// An error occurred during the turn.
    Error { message: String },

    /// Context compaction was triggered.
    CompactionOccurred { messages_summarized: usize },

    /// Token usage update from the API response.
    Usage { input_tokens: u64, output_tokens: u64 },

    /// Tool needs user input. Client should render a modal/dialog,
    /// capture the user's selection, and send back a `user_input_response`
    /// WS message with the request_id and selected index.
    /// The engine is paused until the response arrives.
    UserInputRequest {
        request_id: String,
        title: String,
        subtitle: Option<String>,
        options: Vec<ModalPromptOption>,
    },

    /// Client should render now. Sent after each logical boundary:
    /// streaming text burst, tool call, tool result, etc.
    Flush,
}

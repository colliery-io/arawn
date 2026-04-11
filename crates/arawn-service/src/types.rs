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

    /// Non-fatal warning the user should see (e.g., persistence failure, sandbox unavailable).
    Warning { message: String },

    /// Client should render now. Sent after each logical boundary:
    /// streaming text burst, tool call, tool result, etc.
    Flush,
}

/// Result of storing a fact in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum MemoryStoreResult {
    #[serde(rename = "inserted")]
    Inserted {
        entity_id: String,
        title: String,
        entity_type: String,
    },
    #[serde(rename = "reinforced")]
    Reinforced {
        entity_id: String,
        title: String,
        count: u64,
    },
    #[serde(rename = "superseded")]
    Superseded {
        old_id: String,
        new_id: String,
        title: String,
    },
}

/// Summary of the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySummary {
    pub global: MemoryStoreSummary,
    pub workstream: MemoryStoreSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStoreSummary {
    pub total: u64,
    pub by_type: Vec<MemoryTypeCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryTypeCount {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub count: u64,
}

/// Result of forgetting an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ForgetResult {
    #[serde(rename = "deleted")]
    Deleted {
        title: String,
        entity_type: String,
        scope: String,
    },
    #[serde(rename = "ambiguous")]
    Ambiguous { candidates: Vec<ForgetCandidate> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgetCandidate {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub scope: String,
}

/// A single item in an inventory query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_invocable: Option<bool>,
}

/// A command available for autocomplete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    pub kind: String,
}

/// Result of promoting a scratch session to a workstream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionResult {
    pub workstream_id: String,
    pub workstream_name: String,
}

/// Info about a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron: Option<String>,
}

/// Result of getting or setting the permission mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionModeInfo {
    pub mode: String,
}

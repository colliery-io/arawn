use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Request to create a new session.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateSessionRequest {
    /// Optional title for the session.
    #[serde(default)]
    pub title: Option<String>,
    /// Optional metadata to attach to the session.
    #[serde(default)]
    #[schema(value_type = Object)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Request to update a session.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateSessionRequest {
    /// New title for the session.
    #[serde(default)]
    pub title: Option<String>,
    /// Metadata to merge into the session (existing keys will be overwritten).
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Move session to a different workstream.
    #[serde(default)]
    pub workstream_id: Option<String>,
}

/// Message info for conversation history.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MessageInfo {
    /// Role of the message sender (user, assistant, tool_use, tool_result).
    pub role: String,
    /// Content of the message.
    pub content: String,
    /// Timestamp of the message.
    pub timestamp: String,
    /// Optional metadata (tool name, arguments, success, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<serde_json::Value>,
}

/// Response containing session messages.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionMessagesResponse {
    /// Session ID.
    pub session_id: String,
    /// List of messages in the session.
    pub messages: Vec<MessageInfo>,
    /// Total message count.
    pub count: usize,
}

/// Summary info for a session.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionSummary {
    /// Session ID.
    pub id: String,
    /// Session title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Number of turns in the session.
    pub turn_count: usize,
    /// Creation time (ISO 8601).
    pub created_at: String,
    /// Last update time (ISO 8601).
    pub updated_at: String,
}

/// Full session details.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionDetail {
    /// Session ID.
    pub id: String,
    /// All turns in the session.
    pub turns: Vec<TurnInfo>,
    /// Creation time.
    pub created_at: String,
    /// Last update time.
    pub updated_at: String,
    /// Session metadata.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    #[schema(value_type = Object)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    /// Current workstream ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workstream_id: Option<String>,
    /// Number of files migrated (when moving from scratch).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_migrated: Option<usize>,
    /// Allowed file paths for this session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_paths: Option<Vec<String>>,
}

/// Turn info for API responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TurnInfo {
    /// Turn ID.
    pub id: String,
    /// User message.
    pub user_message: String,
    /// Assistant response (if complete).
    pub assistant_response: Option<String>,
    /// Number of tool calls.
    pub tool_call_count: usize,
    /// When the turn started.
    pub started_at: String,
    /// When the turn completed.
    pub completed_at: Option<String>,
}

/// Response for list sessions.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListSessionsResponse {
    /// List of sessions.
    pub sessions: Vec<SessionSummary>,
    /// Total number of sessions across all pages.
    pub total: usize,
    /// Maximum items per page (as requested).
    pub limit: usize,
    /// Offset from the start of the collection.
    pub offset: usize,
}

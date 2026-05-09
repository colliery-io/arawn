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

/// Runtime capabilities advertised to clients on connect — what optional
/// subsystems are actually available for this server instance. Lets the
/// client surface degraded-functionality warnings (e.g. embeddings missing,
/// memory falls back to keyword search) before the user runs into them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Server version (Cargo package version of the arawn binary).
    pub server_version: String,
    /// True if the embedding model loaded successfully and semantic memory
    /// search is available. False means memory falls back to FTS-only.
    pub embeddings_available: bool,
}

/// Read-only snapshot of the active permission configuration plus a
/// rolling audit of recent decisions. Returned by `get_permissions_status`
/// and rendered by the TUI's `/permissions` command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsStatus {
    pub mode: String,
    pub allow_rules: Vec<String>,
    pub deny_rules: Vec<String>,
    pub ask_rules: Vec<String>,
    pub recent_decisions: Vec<PermissionAuditEntry>,
}

/// One row of the permission audit — what the agent tried to do and how
/// the checker decided.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionAuditEntry {
    /// RFC3339 timestamp.
    pub timestamp: String,
    pub tool_name: String,
    pub tool_input_summary: String,
    /// One of: "allowed", "denied", "ask", "no_match".
    pub decision: String,
    /// Human-readable reason — e.g. "rule 'deny shell(rm -rf *)'" or
    /// "mode default 'default'".
    pub reason: String,
}

/// Server-wide event broadcast to every connected client. Used for things
/// that aren't per-session — hot-reload outcomes, config changes,
/// background-task notifications. Distinct from `EngineEvent`, which is
/// per-conversation-turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerNotice {
    /// Severity: "info" | "warn" | "error".
    pub level: String,
    /// What kind of notice this is — lets the TUI route to the right UI
    /// affordance (banner vs chat history vs status line). Examples:
    /// "plugin_reload", "config_reload", "integration".
    pub category: String,
    /// One-line human-readable message. Already includes any counts or
    /// error details; the TUI just renders verbatim.
    pub message: String,
    /// RFC3339 timestamp the notice was emitted.
    pub timestamp: String,
}

/// One row of the integration registry as seen by clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStatus {
    pub name: String,
    pub connected: bool,
}

/// Returned by `start_oauth_flow` so the TUI knows what URL to open.
/// The actual flow continues asynchronously on the server; the TUI watches
/// for a `ServerNotice` with category="integration" to know when it lands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFlowStarted {
    pub service: String,
    /// URL the user must visit to authorize. The TUI tries to `open` this
    /// and also prints it for copy/paste.
    pub auth_url: String,
}

// ─── Feeds ──────────────────────────────────────────────────────────

/// Args for `ArawnService::feed_register`. Mirrors the `/watch`
/// command surface: template name + free-form params + optional
/// caller-chosen feed id and cadence override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedRegisterSpec {
    /// Template name like `"slack/channel-archive"`.
    pub template: String,
    /// Caller-chosen feed id. Must be unique within the template.
    /// E.g. `"design-channel"` for `slack/channel-archive`.
    pub feed_id: String,
    /// Template-specific params (e.g. `{"channel": "C0123"}`). Schema
    /// validation happens template-side.
    #[serde(default)]
    pub params: serde_json::Value,
    /// Optional cron override. When `None`, the template's default
    /// cadence is used. Must clear the 15-minute floor either way.
    pub cadence: Option<String>,
}

/// User-facing snapshot of one feed for the `/feeds` list.
/// Re-shape of `arawn_feeds::FeedSummary` so the service layer doesn't
/// re-export the feeds crate's type directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSummaryDto {
    pub id: String,
    pub template: String,
    pub cadence: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_run_at: Option<String>,
    pub last_status: Option<String>,
    pub run_count: u64,
    pub data_size_bytes: u64,
    pub data_dir: String,
}

/// Returned by `feed_remove` so the TUI can confirm the wipe with a
/// "deleted N bytes" message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedRemoveDto {
    pub id: String,
    pub template: String,
    pub bytes_wiped: u64,
}

/// One pickable row from `feed_discover`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedDiscoverRow {
    /// What the picker shows in the main column.
    pub label: String,
    /// Optional second-line context (id, member count, privacy tag).
    #[serde(default)]
    pub hint: Option<String>,
    /// Params object the user's selection resolves to. Submitted
    /// straight to `feed_register` without further shaping.
    pub params: serde_json::Value,
}

/// Response from `feed_discover`. `picker_supported = false` means
/// the template's params are free-form — the caller should print a
/// usage message rather than open an empty picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedDiscoverDto {
    pub template: String,
    pub picker_supported: bool,
    pub rows: Vec<FeedDiscoverRow>,
}

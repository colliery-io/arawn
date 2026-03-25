//! Types used by the TUI application.
//!
//! Extracted from app.rs to reduce file size and improve organization.

/// Pending async actions to be executed in the main loop.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PendingAction {
    /// Create a new workstream with the given title.
    CreateWorkstream(String),
    /// Rename a workstream (id, new_title).
    RenameWorkstream(String, String),
    /// Delete a session by ID.
    DeleteSession(String),
    /// Delete a workstream by ID.
    DeleteWorkstream(String),
    /// Refresh sidebar data.
    RefreshSidebar,
    /// Fetch sessions for a workstream by ID.
    FetchWorkstreamSessions(String),
    /// Fetch message history for a session by ID.
    FetchSessionMessages(String),
    /// Move a session to a different workstream (session_id, new_workstream_id).
    MoveSessionToWorkstream(String, String),
}

/// Input mode determines what the input field is being used for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal chat input.
    #[default]
    Chat,
    /// Creating a new workstream - input is the name.
    NewWorkstream,
    /// Renaming a workstream - stores the workstream ID.
    RenameWorkstream(String),
}

/// A chat message for display.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Whether this is from the user (true) or assistant (false).
    pub is_user: bool,
    /// Message content.
    pub content: String,
    /// Whether the message is still streaming.
    pub streaming: bool,
}

/// A tool execution for display.
#[derive(Debug, Clone)]
pub struct ToolExecution {
    /// Tool call ID.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Tool arguments (truncated for display).
    pub args: String,
    /// Accumulated output.
    pub output: String,
    /// Whether the tool is still running.
    pub running: bool,
    /// Whether the tool succeeded (None if still running).
    pub success: Option<bool>,
    /// When the tool started (for duration calculation).
    pub started_at: std::time::Instant,
    /// Duration in milliseconds (calculated when tool ends).
    pub duration_ms: Option<u64>,
}

/// Panel areas for mouse click routing.
#[derive(Debug, Clone, Default)]
pub struct PanelAreas {
    /// Chat message area.
    pub chat: Option<ratatui::layout::Rect>,
    /// Tool pane area.
    pub tool_pane: Option<ratatui::layout::Rect>,
    /// Logs panel area.
    pub logs: Option<ratatui::layout::Rect>,
    /// Sidebar area.
    pub sidebar: Option<ratatui::layout::Rect>,
}

/// Context usage state for display in status bar.
#[derive(Debug, Clone)]
pub struct ContextState {
    /// Current token count.
    pub current_tokens: usize,
    /// Maximum tokens.
    pub max_tokens: usize,
    /// Usage percentage (0-100).
    pub percent: u8,
    /// Status: "ok", "warning", or "critical".
    pub status: String,
}

/// Disk usage statistics for a workstream.
#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    /// Workstream ID.
    pub workstream_id: String,
    /// Workstream name.
    pub workstream_name: String,
    /// Whether this is a scratch workstream.
    pub is_scratch: bool,
    /// Size of production directory in bytes.
    pub production_bytes: u64,
    /// Size of work directory in bytes.
    pub work_bytes: u64,
    /// Total size in bytes.
    pub total_bytes: u64,
    /// Configured limit in bytes (0 = no limit).
    pub limit_bytes: u64,
    /// Usage percentage (0-100).
    pub percent: u8,
}

impl UsageStats {
    /// Format size as human-readable string.
    pub fn format_size(bytes: u64) -> String {
        if bytes >= 1024 * 1024 * 1024 {
            format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            format!("{:.0} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            format!("{:.0} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Get formatted production size.
    pub fn production_size(&self) -> String {
        Self::format_size(self.production_bytes)
    }

    /// Get formatted work size.
    pub fn work_size(&self) -> String {
        Self::format_size(self.work_bytes)
    }

    /// Get formatted total size.
    pub fn total_size(&self) -> String {
        Self::format_size(self.total_bytes)
    }

    /// Get formatted limit.
    pub fn limit_size(&self) -> String {
        if self.limit_bytes == 0 {
            "∞".to_string()
        } else {
            Self::format_size(self.limit_bytes)
        }
    }
}

/// A disk usage warning.
#[derive(Debug, Clone)]
pub struct DiskWarning {
    /// Workstream ID for deduplication.
    pub workstream_id: String,
    /// Workstream display name.
    pub workstream: String,
    /// Warning level: "warning" or "critical".
    pub level: String,
    /// Current usage in bytes.
    pub usage_bytes: u64,
    /// Limit in bytes.
    pub limit_bytes: u64,
    /// Usage percentage.
    pub percent: u8,
    /// When the warning was received.
    pub timestamp: std::time::Instant,
}

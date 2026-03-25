use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── Request/Response types ──────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkstreamRequest {
    /// Workstream title.
    pub title: String,
    /// Default model for this workstream.
    #[serde(default)]
    pub default_model: Option<String>,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkstreamResponse {
    /// Unique workstream ID.
    pub id: String,
    /// Workstream title.
    pub title: String,
    /// Workstream summary.
    pub summary: Option<String>,
    /// Workstream state (active, archived).
    pub state: String,
    /// Default model for this workstream.
    pub default_model: Option<String>,
    /// Whether this is the scratch workstream.
    pub is_scratch: bool,
    /// Creation timestamp (RFC 3339).
    pub created_at: String,
    /// Last update timestamp (RFC 3339).
    pub updated_at: String,
    /// Tags for categorization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkstreamListResponse {
    /// List of workstreams.
    pub workstreams: Vec<WorkstreamResponse>,
    /// Total number of workstreams across all pages.
    pub total: usize,
    /// Maximum items per page (as requested).
    pub limit: usize,
    /// Offset from the start of the collection.
    pub offset: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendMessageRequest {
    /// Message role. Defaults to `"user"` if omitted.
    ///
    /// Valid values:
    /// - `"user"` — a human message
    /// - `"assistant"` — an agent response
    /// - `"system"` — a system-level instruction
    /// - `"agent_push"` — an agent-initiated notification (not in response to a user message)
    #[schema(example = "user")]
    pub role: Option<String>,
    /// Message content (plain text or markdown).
    pub content: String,
    /// Optional metadata as a JSON string. Stored verbatim and returned on read.
    #[serde(default)]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MessageResponse {
    /// Unique message ID.
    pub id: String,
    /// Workstream this message belongs to.
    pub workstream_id: String,
    /// Session this message belongs to, if any.
    pub session_id: Option<String>,
    /// Message role.
    pub role: String,
    /// Message content.
    pub content: String,
    /// Message timestamp (RFC 3339).
    pub timestamp: String,
    /// Optional metadata JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageListResponse {
    /// List of messages.
    pub messages: Vec<MessageResponse>,
    /// Total number of messages across all pages.
    pub total: usize,
    /// Maximum items per page (as requested).
    pub limit: usize,
    /// Offset from the start of the collection.
    pub offset: usize,
}

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    pub since: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListWorkstreamsQuery {
    /// Include archived workstreams in the response.
    #[serde(default)]
    pub include_archived: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PromoteRequest {
    /// Title for the promoted workstream.
    pub title: String,
    /// Tags for the promoted workstream.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Default model for the promoted workstream.
    #[serde(default)]
    pub default_model: Option<String>,
}

/// Request to promote a file from work/ to production/.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PromoteFileRequest {
    /// Source path relative to work/.
    pub source: String,
    /// Destination path relative to production/.
    pub destination: String,
}

/// Response from file promotion.
#[derive(Debug, Serialize, ToSchema)]
pub struct PromoteFileResponse {
    /// Final path of the promoted file (relative to production/).
    pub path: String,
    /// File size in bytes.
    pub bytes: u64,
    /// Whether the file was renamed due to a conflict.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub renamed: bool,
}

/// Request to export a file from production/ to external path.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExportFileRequest {
    /// Source path relative to production/.
    pub source: String,
    /// Absolute destination path (directory or file).
    pub destination: String,
}

/// Response from file export.
#[derive(Debug, Serialize, ToSchema)]
pub struct ExportFileResponse {
    /// Final path of the exported file.
    pub exported_to: String,
    /// File size in bytes.
    pub bytes: u64,
}

/// Request to clone a git repository into production/.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CloneRepoRequest {
    /// Git repository URL (HTTPS or SSH).
    pub url: String,
    /// Optional custom directory name.
    #[serde(default)]
    pub name: Option<String>,
}

/// Response from git clone operation.
#[derive(Debug, Serialize, ToSchema)]
pub struct CloneRepoResponse {
    /// Path where the repository was cloned (relative to production/).
    pub path: String,
    /// HEAD commit hash.
    pub commit: String,
}

/// Per-session disk usage info.
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionUsageResponse {
    /// Session ID.
    pub id: String,
    /// Disk usage in megabytes.
    pub mb: f64,
}

/// Response from usage stats endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct UsageResponse {
    /// Production directory size in megabytes.
    pub production_mb: f64,
    /// Work directory size in megabytes.
    pub work_mb: f64,
    /// Per-session breakdown (only for scratch workstream).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sessions: Vec<SessionUsageResponse>,
    /// Total disk usage in megabytes.
    pub total_mb: f64,
    /// Warnings based on configured thresholds.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Request to clean up work directory files.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CleanupRequest {
    /// Only delete files older than this many days.
    #[serde(default)]
    pub older_than_days: Option<u32>,
    /// Confirm deletion of more than 100 files.
    #[serde(default)]
    pub confirm: bool,
}

/// Response from cleanup operation.
#[derive(Debug, Serialize, ToSchema)]
pub struct CleanupResponse {
    /// Number of files deleted.
    pub deleted_files: usize,
    /// Total megabytes freed.
    pub freed_mb: f64,
    /// Number of files pending deletion (if confirmation required).
    #[serde(skip_serializing_if = "is_zero")]
    pub pending_files: usize,
    /// Whether confirmation is required for this operation.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub requires_confirmation: bool,
}

pub(crate) fn is_zero(v: &usize) -> bool {
    *v == 0
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateWorkstreamRequest {
    /// New title.
    #[serde(default)]
    pub title: Option<String>,
    /// New summary.
    #[serde(default)]
    pub summary: Option<String>,
    /// New default model.
    #[serde(default)]
    pub default_model: Option<String>,
    /// New tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionResponse {
    /// Session ID.
    pub id: String,
    /// Workstream this session belongs to.
    pub workstream_id: String,
    /// Session start timestamp (RFC 3339).
    pub started_at: String,
    /// Session end timestamp, if ended (RFC 3339).
    pub ended_at: Option<String>,
    /// Whether the session is currently active.
    pub is_active: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SessionListResponse {
    /// List of sessions.
    pub sessions: Vec<SessionResponse>,
    /// Total number of sessions across all pages.
    pub total: usize,
    /// Maximum items per page (as requested).
    pub limit: usize,
    /// Offset from the start of the collection.
    pub offset: usize,
}

/// Response from compression operation.
#[derive(Debug, Serialize, ToSchema)]
pub struct CompressResponse {
    /// Workstream summary after compression.
    pub summary: String,
    /// Number of sessions that were compressed.
    pub sessions_compressed: usize,
}

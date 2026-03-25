use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// A note (API representation).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Note {
    /// Note ID.
    pub id: String,
    /// Optional title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Note content.
    pub content: String,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Creation time (RFC 3339).
    pub created_at: String,
    /// Last update time (RFC 3339).
    pub updated_at: String,
}

/// Request to create a note.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateNoteRequest {
    /// Note content.
    pub content: String,
    /// Optional title.
    #[serde(default)]
    pub title: Option<String>,
    /// Optional tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Query params for listing notes.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ListNotesQuery {
    /// Filter by tag.
    pub tag: Option<String>,
}

/// Request to update a note.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateNoteRequest {
    /// New title for the note.
    #[serde(default)]
    pub title: Option<String>,
    /// New content for the note.
    #[serde(default)]
    pub content: Option<String>,
    /// New tags for the note (replaces existing tags).
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Response for listing notes.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListNotesResponse {
    /// List of notes.
    pub notes: Vec<Note>,
    /// Total number of notes across all pages.
    pub total: usize,
    /// Maximum items per page (as requested).
    pub limit: usize,
    /// Offset from the start of the collection.
    pub offset: usize,
}

/// Query params for memory search.
#[derive(Debug, Clone, Deserialize)]
pub struct MemorySearchQuery {
    /// Search query text.
    pub q: String,
    /// Maximum results.
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Optional session ID to scope the search.
    pub session_id: Option<String>,
}

fn default_limit() -> usize {
    10
}

/// Memory search result item.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MemorySearchResult {
    /// Result ID.
    pub id: String,
    /// Content type.
    pub content_type: String,
    /// Content text.
    pub content: String,
    /// Session the memory belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Relevance score (0.0 - 1.0).
    pub score: f32,
    /// Where the result came from (e.g., "text" or "note").
    pub source: String,
    /// Citation metadata for provenance tracking.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub citation: Option<serde_json::Value>,
}

/// Response for memory search.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MemorySearchResponse {
    /// Search results ordered by relevance (highest score first).
    pub results: Vec<MemorySearchResult>,
    /// The query that was executed.
    pub query: String,
    /// Number of results returned (equal to `results.len()`).
    pub count: usize,
    /// When `true`, the search fell back to text-only matching because the
    /// vector/embedding search failed. Results may be less relevant.
    /// Only present when `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub degraded: bool,
}

/// Request to store a memory directly.
///
/// Requires the memory/indexing feature to be enabled on the server
/// (returns 503 otherwise). Memories are persisted in the vector store
/// and become searchable via `GET /api/v1/memory/search`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct StoreMemoryRequest {
    /// The memory content (plain text).
    pub content: String,
    /// Content type. Defaults to `"fact"`.
    ///
    /// Valid values: `"fact"`, `"summary"`, `"insight"`, `"preference"`,
    /// `"procedure"`, `"entity"`.
    #[serde(default = "default_content_type")]
    #[schema(example = "fact")]
    pub content_type: String,
    /// Optional session ID to associate with this memory.
    /// When set, the memory can be filtered by session in search results.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Optional metadata as key-value pairs. Stored alongside the memory.
    #[serde(default)]
    #[schema(value_type = Object)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Confidence score (0.0 to 1.0). Defaults to 0.8.
    /// Higher scores rank higher in search results.
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

fn default_content_type() -> String {
    "fact".to_string()
}

fn default_confidence() -> f32 {
    0.8
}

/// Response after storing a memory.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StoreMemoryResponse {
    /// The stored memory ID.
    pub id: String,
    /// Content type.
    pub content_type: String,
    /// Confirmation message.
    pub message: String,
}

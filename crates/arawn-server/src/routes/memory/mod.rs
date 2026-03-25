//! Memory and notes endpoints.
//!
//! These endpoints provide access to the memory search and notes functionality,
//! backed by `arawn-memory::MemoryStore` for persistent storage.

mod types;
pub use types::*;

#[cfg(test)]
mod tests;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use std::sync::Arc;

use arawn_domain::{ContentType, Memory, MemoryId, MemoryNote, MemoryStore, NoteId};

use super::pagination::PaginationParams;
use crate::auth::Identity;
use crate::error::ServerError;
use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Get the memory store from app state, returning 503 if not configured.
fn require_memory_store(state: &AppState) -> Result<&Arc<MemoryStore>, ServerError> {
    state
        .memory_store()
        .ok_or_else(|| ServerError::ServiceUnavailable("Memory storage not configured".to_string()))
}

/// Convert an `arawn_memory::Note` to the API `Note` type.
fn to_api_note(note: MemoryNote) -> Note {
    Note {
        id: note.id.to_string(),
        title: note.title,
        content: note.content,
        tags: note.tags,
        created_at: note.created_at.to_rfc3339(),
        updated_at: note.updated_at.to_rfc3339(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// POST /api/v1/notes - Create a new note.
#[utoipa::path(
    post,
    path = "/api/v1/notes",
    request_body = CreateNoteRequest,
    responses(
        (status = 201, description = "Note created", body = Note),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Memory storage not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "memory"
)]
pub async fn create_note_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Json(request): Json<CreateNoteRequest>,
) -> Result<(StatusCode, Json<Note>), ServerError> {
    let store = require_memory_store(&state)?;

    let mut note = MemoryNote::new(request.content);
    if let Some(title) = request.title {
        note = note.with_title(title);
    }
    for tag in request.tags {
        note = note.with_tag(tag);
    }

    store
        .insert_note(&note)
        .map_err(|e| ServerError::Internal(format!("Failed to create note: {}", e)))?;

    Ok((StatusCode::CREATED, Json(to_api_note(note))))
}

/// GET /api/v1/notes - List notes.
#[utoipa::path(
    get,
    path = "/api/v1/notes",
    params(
        ("tag" = Option<String>, Query, description = "Filter by tag"),
        PaginationParams,
    ),
    responses(
        (status = 200, description = "List of notes", body = ListNotesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Memory storage not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "memory"
)]
pub async fn list_notes_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Query(query): Query<ListNotesQuery>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ListNotesResponse>, ServerError> {
    let store = require_memory_store(&state)?;
    let limit = pagination.effective_limit();

    let (notes, total) = if let Some(ref tag) = query.tag {
        // Tag-filtered: fetch all matching, paginate in memory
        let all = store
            .list_notes_by_tag(tag, 10_000)
            .map_err(|e| ServerError::Internal(format!("Failed to list notes: {}", e)))?;
        let total = all.len();
        let offset = pagination.offset.min(total);
        let end = (offset + limit).min(total);
        (all[offset..end].to_vec(), total)
    } else {
        // Unfiltered: use store pagination directly
        let notes = store
            .list_notes(limit, pagination.offset)
            .map_err(|e| ServerError::Internal(format!("Failed to list notes: {}", e)))?;
        let total = store.stats().map(|s| s.note_count).unwrap_or(notes.len());
        (notes, total)
    };

    let api_notes: Vec<Note> = notes.into_iter().map(to_api_note).collect();

    Ok(Json(ListNotesResponse {
        notes: api_notes,
        total,
        limit,
        offset: pagination.offset,
    }))
}

/// GET /api/v1/notes/:id - Get a single note.
#[utoipa::path(
    get,
    path = "/api/v1/notes/{id}",
    params(
        ("id" = String, Path, description = "Note ID"),
    ),
    responses(
        (status = 200, description = "Note found", body = Note),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Note not found"),
        (status = 503, description = "Memory storage not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "memory"
)]
pub async fn get_note_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(note_id): Path<String>,
) -> Result<Json<Note>, ServerError> {
    let store = require_memory_store(&state)?;

    let id = NoteId::parse(&note_id)
        .map_err(|_| ServerError::BadRequest(format!("Invalid note ID: {}", note_id)))?;

    let note = store
        .get_note(id)
        .map_err(|e| ServerError::Internal(format!("Failed to get note: {}", e)))?
        .ok_or_else(|| ServerError::NotFound(format!("Note {} not found", note_id)))?;

    Ok(Json(to_api_note(note)))
}

/// PUT /api/v1/notes/:id - Update a note.
#[utoipa::path(
    put,
    path = "/api/v1/notes/{id}",
    params(
        ("id" = String, Path, description = "Note ID"),
    ),
    request_body = UpdateNoteRequest,
    responses(
        (status = 200, description = "Note updated", body = Note),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Note not found"),
        (status = 503, description = "Memory storage not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "memory"
)]
pub async fn update_note_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(note_id): Path<String>,
    Json(request): Json<UpdateNoteRequest>,
) -> Result<Json<Note>, ServerError> {
    let store = require_memory_store(&state)?;

    let id = NoteId::parse(&note_id)
        .map_err(|_| ServerError::BadRequest(format!("Invalid note ID: {}", note_id)))?;

    let mut note = store
        .get_note(id)
        .map_err(|e| ServerError::Internal(format!("Failed to get note: {}", e)))?
        .ok_or_else(|| ServerError::NotFound(format!("Note {} not found", note_id)))?;

    if let Some(title) = request.title {
        note.title = Some(title);
    }
    if let Some(content) = request.content {
        note.content = content;
    }
    if let Some(tags) = request.tags {
        note.tags = tags;
    }
    note.updated_at = chrono::Utc::now();

    store
        .update_note(&note)
        .map_err(|e| ServerError::Internal(format!("Failed to update note: {}", e)))?;

    Ok(Json(to_api_note(note)))
}

/// DELETE /api/v1/notes/:id - Delete a note.
#[utoipa::path(
    delete,
    path = "/api/v1/notes/{id}",
    params(
        ("id" = String, Path, description = "Note ID"),
    ),
    responses(
        (status = 204, description = "Note deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Note not found"),
        (status = 503, description = "Memory storage not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "memory"
)]
pub async fn delete_note_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(note_id): Path<String>,
) -> Result<StatusCode, ServerError> {
    let store = require_memory_store(&state)?;

    let id = NoteId::parse(&note_id)
        .map_err(|_| ServerError::BadRequest(format!("Invalid note ID: {}", note_id)))?;

    let deleted = store
        .delete_note(id)
        .map_err(|e| ServerError::Internal(format!("Failed to delete note: {}", e)))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ServerError::NotFound(format!("Note {} not found", note_id)))
    }
}

/// GET /api/v1/memory/search - Search memories.
///
/// Searches the MemoryStore (text match on indexed facts, summaries, etc.)
/// and supplements with matching notes. Results are sorted by relevance
/// score (highest first).
///
/// When the memory store search fails, sets `degraded: true` and returns
/// note-only results.
#[utoipa::path(
    get,
    path = "/api/v1/memory/search",
    params(
        ("q" = String, Query, description = "Search query text (case-insensitive for note matching)"),
        ("limit" = Option<usize>, Query, description = "Maximum results to return (default: 10)"),
        ("session_id" = Option<String>, Query, description = "Filter results to a specific session. Only applies to memory results, not notes."),
    ),
    responses(
        (status = 200, description = "Search results ordered by relevance. Check `degraded` to detect fallback mode.", body = MemorySearchResponse),
        (status = 401, description = "Unauthorized — missing or invalid bearer token"),
        (status = 503, description = "Memory storage not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "memory"
)]
pub async fn memory_search_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Query(query): Query<MemorySearchQuery>,
) -> Result<Json<MemorySearchResponse>, ServerError> {
    let store = require_memory_store(&state)?;

    let mut results: Vec<MemorySearchResult> = Vec::new();
    let mut degraded = false;

    // Search memories (facts, summaries, etc.)
    match store.search_memories(&query.q, query.limit) {
        Ok(memories) => {
            for memory in memories {
                // Apply session filter if provided
                if let Some(ref sid) = query.session_id
                    && memory.session_id.as_deref() != Some(sid.as_str())
                {
                    continue;
                }
                let citation = memory
                    .citation
                    .as_ref()
                    .and_then(|c| serde_json::to_value(c).ok());
                results.push(MemorySearchResult {
                    id: memory.id.to_string(),
                    content_type: memory.content_type.as_str().to_string(),
                    content: memory.content,
                    session_id: memory.session_id,
                    score: memory.confidence.score,
                    source: "memory_store".to_string(),
                    citation,
                });
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "MemoryStore search failed, falling back to notes");
            degraded = true;
        }
    }

    // Supplement with matching notes
    let remaining = query.limit.saturating_sub(results.len());
    if remaining > 0
        && let Ok(notes) = store.search_notes(&query.q, remaining)
    {
        for note in notes {
            results.push(MemorySearchResult {
                id: note.id.to_string(),
                content_type: "note".to_string(),
                content: note.content,
                session_id: None,
                score: 1.0,
                source: "notes".to_string(),
                citation: None,
            });
        }
    }

    // Sort by score descending
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(query.limit);

    let count = results.len();
    Ok(Json(MemorySearchResponse {
        results,
        query: query.q,
        count,
        degraded,
    }))
}

/// POST /api/v1/memory - Store a memory directly.
#[utoipa::path(
    post,
    path = "/api/v1/memory",
    request_body = StoreMemoryRequest,
    responses(
        (status = 201, description = "Memory stored", body = StoreMemoryResponse),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Memory storage not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "memory"
)]
pub async fn store_memory_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Json(request): Json<StoreMemoryRequest>,
) -> Result<(StatusCode, Json<StoreMemoryResponse>), ServerError> {
    let store = require_memory_store(&state)?;

    // Parse content type (default to Fact if invalid)
    let content_type = ContentType::parse(&request.content_type).unwrap_or(ContentType::Fact);
    let mut memory = Memory::new(content_type, &request.content);

    // Set session ID if provided
    if let Some(ref session_id) = request.session_id {
        memory = memory.with_session(session_id);
    }

    // Set confidence
    memory.confidence.score = request.confidence;

    // Store the memory
    store
        .insert_memory(&memory)
        .map_err(|e| ServerError::Internal(format!("Failed to store memory: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(StoreMemoryResponse {
            id: memory.id.to_string(),
            content_type: request.content_type,
            message: "Memory stored successfully".to_string(),
        }),
    ))
}

/// DELETE /api/v1/memory/:id - Delete a memory.
#[utoipa::path(
    delete,
    path = "/api/v1/memory/{id}",
    params(
        ("id" = String, Path, description = "Memory ID (UUID)"),
    ),
    responses(
        (status = 204, description = "Memory deleted"),
        (status = 400, description = "Invalid memory ID"),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Memory storage not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "memory"
)]
pub async fn delete_memory_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(memory_id): Path<String>,
) -> Result<StatusCode, ServerError> {
    let store = require_memory_store(&state)?;

    // Parse UUID and wrap in MemoryId
    let uuid = uuid::Uuid::parse_str(&memory_id)
        .map_err(|_| ServerError::BadRequest(format!("Invalid memory ID: {}", memory_id)))?;
    let id = MemoryId(uuid);

    // Delete the memory
    store
        .delete_memory(id)
        .map_err(|e| ServerError::Internal(format!("Failed to delete memory: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

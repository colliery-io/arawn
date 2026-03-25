//! Session management endpoints.

mod types;
pub use types::*;

#[cfg(test)]
mod tests;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use arawn_domain::{Session, SessionId};

use super::pagination::PaginationParams;
use crate::auth::Identity;
use crate::error::ServerError;
use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// POST /api/v1/sessions - Create a new session.
#[utoipa::path(
    post,
    path = "/api/v1/sessions",
    request_body = CreateSessionRequest,
    responses(
        (status = 201, description = "Session created", body = SessionDetail),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "sessions"
)]
pub async fn create_session_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<SessionDetail>), ServerError> {
    // Create a new session with optional metadata
    let session_id = state.get_or_create_session(None).await;

    // Update metadata if provided
    if !request.metadata.is_empty() || request.title.is_some() {
        state
            .session_cache()
            .with_session_mut(&session_id, |session| {
                // Set title as metadata if provided
                if let Some(title) = &request.title {
                    session.metadata.insert(
                        "title".to_string(),
                        serde_json::Value::String(title.clone()),
                    );
                }
                // Merge additional metadata
                for (key, value) in &request.metadata {
                    session.metadata.insert(key.clone(), value.clone());
                }
            })
            .await;
    }

    // Return the created session
    let session = state
        .session_cache()
        .get(&session_id)
        .await
        .ok_or_else(|| ServerError::Internal("Failed to retrieve created session".to_string()))?;

    // Get workstream ID and allowed paths
    let workstream_id = state
        .session_cache()
        .get_workstream_id(&session_id)
        .await
        .unwrap_or_else(|| "scratch".to_string());
    let allowed_paths = state
        .allowed_paths(&workstream_id, &session_id.to_string())
        .map(|paths| paths.iter().map(|p| p.display().to_string()).collect());

    Ok((
        StatusCode::CREATED,
        Json(session_to_detail_with_migration(
            &session,
            Some(workstream_id),
            None,
            allowed_paths,
        )),
    ))
}

/// GET /api/v1/sessions - List all sessions.
#[utoipa::path(
    get,
    path = "/api/v1/sessions",
    params(PaginationParams),
    responses(
        (status = 200, description = "List of sessions", body = ListSessionsResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "sessions"
)]
pub async fn list_sessions_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ListSessionsResponse>, ServerError> {
    let mut summaries: Vec<SessionSummary> = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // Get sessions from the cache (active sessions)
    let cached_sessions = state.session_cache().all_sessions().await;
    for (_, session) in cached_sessions {
        let title = session
            .metadata
            .get("title")
            .and_then(|v| v.as_str())
            .map(String::from);
        seen_ids.insert(session.id.to_string());
        summaries.push(SessionSummary {
            id: session.id.to_string(),
            title,
            turn_count: session.turn_count(),
            created_at: session.created_at.to_rfc3339(),
            updated_at: session.updated_at.to_rfc3339(),
        });
    }

    // Also include sessions from workstream storage (for historical sessions)
    if let Some(workstreams) = state.workstreams()
        && let Ok(ws_list) = workstreams.list_workstreams()
    {
        for ws in ws_list {
            if let Ok(ws_sessions) = workstreams.list_sessions(&ws.id) {
                for ws_session in ws_sessions {
                    // Skip if we already have this session from cache
                    if seen_ids.contains(&ws_session.id) {
                        continue;
                    }
                    seen_ids.insert(ws_session.id.clone());

                    summaries.push(SessionSummary {
                        id: ws_session.id.clone(),
                        title: ws_session.summary.clone(),
                        turn_count: ws_session.turn_count.unwrap_or(0) as usize,
                        created_at: ws_session.started_at.to_rfc3339(),
                        updated_at: ws_session
                            .ended_at
                            .unwrap_or(ws_session.started_at)
                            .to_rfc3339(),
                    });
                }
            }
        }
    }

    // Sort by updated_at descending (most recent first)
    summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let (paginated, total) = pagination.paginate(&summaries);

    Ok(Json(ListSessionsResponse {
        sessions: paginated,
        total,
        limit: pagination.effective_limit(),
        offset: pagination.offset,
    }))
}

/// GET /api/v1/sessions/:id - Get session details.
#[utoipa::path(
    get,
    path = "/api/v1/sessions/{id}",
    params(("id" = String, Path, description = "Session ID")),
    responses(
        (status = 200, description = "Session details", body = SessionDetail),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Session not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "sessions"
)]
pub async fn get_session_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionDetail>, ServerError> {
    let id = parse_session_id(&session_id)?;

    // Helper to get allowed paths for a session
    let get_allowed_paths = |ws_id: &str, sess_id: &str| {
        state
            .allowed_paths(ws_id, sess_id)
            .map(|paths| paths.iter().map(|p| p.display().to_string()).collect())
    };

    // Try session cache first
    if let Some(session) = state.session_cache().get(&id).await {
        let workstream_id = state
            .session_cache()
            .get_workstream_id(&id)
            .await
            .unwrap_or_else(|| "scratch".to_string());
        let allowed_paths = get_allowed_paths(&workstream_id, &session_id);
        return Ok(Json(session_to_detail_with_migration(
            &session,
            Some(workstream_id),
            None,
            allowed_paths,
        )));
    }

    // Try to load from workstream if workstreams are configured
    if let Some(workstreams) = state.workstreams() {
        // First, find which workstream this session belongs to
        if let Ok(ws_list) = workstreams.list_workstreams() {
            for ws in ws_list {
                if let Ok(ws_sessions) = workstreams.list_sessions(&ws.id)
                    && ws_sessions.iter().any(|s| s.id == session_id)
                {
                    // Found the workstream, try to load the session
                    if let Ok((session, _)) = state.session_cache().get_or_load(id, &ws.id).await {
                        let allowed_paths = get_allowed_paths(&ws.id, &session_id);
                        return Ok(Json(session_to_detail_with_migration(
                            &session,
                            Some(ws.id),
                            None,
                            allowed_paths,
                        )));
                    }
                }
            }
        }
    }

    Err(ServerError::NotFound(format!(
        "Session {} not found",
        session_id
    )))
}

/// DELETE /api/v1/sessions/:id - Delete a session.
///
/// Removes the session and triggers background indexing (if enabled).
#[utoipa::path(
    delete,
    path = "/api/v1/sessions/{id}",
    params(("id" = String, Path, description = "Session ID")),
    responses(
        (status = 204, description = "Session deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Session not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "sessions"
)]
pub async fn delete_session_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(session_id): Path<String>,
) -> Result<StatusCode, ServerError> {
    let id = parse_session_id(&session_id)?;

    // Try removing from the in-memory cache first (handles active sessions)
    let was_in_cache = state.close_session(id).await;

    // Also delete from persistent workstream storage (whether or not it was in cache)
    if let Some(workstreams) = state.workstreams() {
        match workstreams.delete_session(&session_id) {
            Ok(()) => {
                tracing::info!(session_id = %session_id, was_in_cache, "Session deleted from store");
                return Ok(StatusCode::NO_CONTENT);
            }
            Err(e) => {
                tracing::debug!(
                    session_id = %session_id,
                    was_in_cache,
                    error = %e,
                    "Session not found in store"
                );
                // If it was at least in cache, that's still a success
                if was_in_cache {
                    return Ok(StatusCode::NO_CONTENT);
                }
            }
        }
    } else if was_in_cache {
        return Ok(StatusCode::NO_CONTENT);
    }

    Err(ServerError::NotFound(format!(
        "Session {} not found",
        session_id
    )))
}

/// PATCH /api/v1/sessions/:id - Update session metadata.
#[utoipa::path(
    patch,
    path = "/api/v1/sessions/{id}",
    params(("id" = String, Path, description = "Session ID")),
    request_body = UpdateSessionRequest,
    responses(
        (status = 200, description = "Session updated", body = SessionDetail),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Session not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "sessions"
)]
pub async fn update_session_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(session_id): Path<String>,
    Json(request): Json<UpdateSessionRequest>,
) -> Result<Json<SessionDetail>, ServerError> {
    tracing::info!(
        session_id = %session_id,
        workstream_id = ?request.workstream_id,
        title = ?request.title,
        "PATCH /sessions/:id - update_session_handler called"
    );

    let id = parse_session_id(&session_id)?;

    // Track the workstream ID for potential reload after reassignment
    let mut target_workstream_id: Option<String> = None;
    // Track file migration info when moving from scratch
    let mut migration_result: Option<arawn_domain::AttachResult> = None;

    // Handle workstream reassignment if requested
    if let Some(ref new_workstream_id) = request.workstream_id {
        // Validate workstream ID to prevent path traversal
        if !arawn_domain::DirectoryManager::is_valid_name(new_workstream_id) {
            return Err(ServerError::BadRequest(format!(
                "Invalid workstream ID: '{}'. Must contain only alphanumeric characters, hyphens, and underscores.",
                new_workstream_id
            )));
        }
        tracing::info!(
            session_id = %session_id,
            new_workstream_id = %new_workstream_id,
            "Attempting to reassign session to new workstream"
        );
        if let Some(workstreams) = state.workstreams() {
            // Get the current workstream ID before reassignment to detect scratch→named migration
            let current_workstream_id = workstreams
                .store()
                .get_session(&session_id)
                .map(|s| s.workstream_id)
                .ok();

            match workstreams.reassign_session(&session_id, new_workstream_id) {
                Ok(session) => {
                    tracing::info!(
                        session_id = %session_id,
                        new_workstream_id = %new_workstream_id,
                        result_workstream_id = %session.workstream_id,
                        "Session reassignment successful"
                    );

                    // Check if we're moving from scratch to a named workstream
                    // and if directory manager is configured
                    if let Some(ref old_ws) = current_workstream_id
                        && old_ws == "scratch"
                        && new_workstream_id != "scratch"
                        && let Some(dir_mgr) = workstreams.directory_manager()
                    {
                        match dir_mgr.attach_session(&session_id, new_workstream_id) {
                            Ok(result) => {
                                tracing::info!(
                                    session_id = %session_id,
                                    files_migrated = result.files_migrated,
                                    new_work_path = %result.new_work_path.display(),
                                    "Migrated session files from scratch"
                                );
                                migration_result = Some(result);
                            }
                            Err(arawn_domain::DirectoryError::SessionWorkNotFound(_)) => {
                                // No files to migrate, that's fine
                                tracing::debug!(
                                    session_id = %session_id,
                                    "No scratch session work directory to migrate"
                                );
                            }
                            Err(e) => {
                                // Log but don't fail - file migration is best-effort
                                tracing::warn!(
                                    session_id = %session_id,
                                    error = %e,
                                    "Failed to migrate session files from scratch (non-fatal)"
                                );
                            }
                        }
                    }

                    // Invalidate the session cache so it reloads from the new workstream
                    state.invalidate_session(id).await;
                    target_workstream_id = Some(new_workstream_id.clone());
                }
                Err(e) => {
                    tracing::error!(
                        session_id = %session_id,
                        new_workstream_id = %new_workstream_id,
                        error = %e,
                        "Session reassignment failed"
                    );
                    return Err(ServerError::BadRequest(format!(
                        "Failed to reassign session: {}",
                        e
                    )));
                }
            }
        } else {
            tracing::error!("Workstreams not configured");
            return Err(ServerError::BadRequest(
                "Workstreams not configured".to_string(),
            ));
        }
    }

    // If we reassigned, we need to reload the session from the new workstream
    // before applying any metadata updates
    if let Some(ref workstream_id) = target_workstream_id {
        // Reload session from the new workstream
        let (mut session, _) = state
            .session_cache()
            .get_or_load(id, workstream_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    session_id = %session_id,
                    workstream_id = %workstream_id,
                    error = %e,
                    "Failed to reload session after reassignment"
                );
                ServerError::NotFound(format!(
                    "Session {} not found after reassignment",
                    session_id
                ))
            })?;

        // Apply title/metadata updates if provided
        if request.title.is_some() || request.metadata.is_some() {
            if let Some(ref title) = request.title {
                session.metadata.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.clone()),
                );
            }
            if let Some(ref metadata) = request.metadata {
                for (key, value) in metadata {
                    session.metadata.insert(key.clone(), value.clone());
                }
            }
            session.updated_at = chrono::Utc::now();

            // Update the cache with the modified session
            let _ = state.session_cache().update(id, session.clone()).await;
        }

        // Build response with migration info if available
        let (files_migrated, allowed_paths) = match migration_result {
            Some(ref result) => (
                Some(result.files_migrated),
                Some(
                    result
                        .allowed_paths
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect(),
                ),
            ),
            None => (None, None),
        };

        return Ok(Json(session_to_detail_with_migration(
            &session,
            Some(workstream_id.clone()),
            files_migrated,
            allowed_paths,
        )));
    }

    // No reassignment - update session via cache directly
    let updated = state
        .session_cache()
        .with_session_mut(&id, |session| {
            // Update title if provided
            if let Some(ref title) = request.title {
                session.metadata.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.clone()),
                );
            }

            // Merge metadata if provided
            if let Some(ref metadata) = request.metadata {
                for (key, value) in metadata {
                    session.metadata.insert(key.clone(), value.clone());
                }
            }

            // Update timestamp
            session.updated_at = chrono::Utc::now();
            session_to_detail(session)
        })
        .await;

    match updated {
        Some(detail) => Ok(Json(detail)),
        None => Err(ServerError::NotFound(format!(
            "Session {} not found",
            session_id
        ))),
    }
}

/// GET /api/v1/sessions/:id/messages - Get session conversation history.
#[utoipa::path(
    get,
    path = "/api/v1/sessions/{id}/messages",
    params(("id" = String, Path, description = "Session ID")),
    responses(
        (status = 200, description = "Session messages", body = SessionMessagesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Session not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "sessions"
)]
pub async fn get_session_messages_handler(
    State(state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionMessagesResponse>, ServerError> {
    let id = parse_session_id(&session_id)?;

    // Try session cache first
    let session = if let Some(session) = state.session_cache().get(&id).await {
        session
    } else if let Some(workstreams) = state.workstreams() {
        // Try to load from workstream
        let mut found_session = None;
        if let Ok(ws_list) = workstreams.list_workstreams() {
            for ws in ws_list {
                if let Ok(ws_sessions) = workstreams.list_sessions(&ws.id)
                    && ws_sessions.iter().any(|s| s.id == session_id)
                {
                    // Found the workstream, try to load the session
                    if let Ok((session, _)) = state.session_cache().get_or_load(id, &ws.id).await {
                        found_session = Some(session);
                        break;
                    }
                }
            }
        }
        found_session
            .ok_or_else(|| ServerError::NotFound(format!("Session {} not found", session_id)))?
    } else {
        return Err(ServerError::NotFound(format!(
            "Session {} not found",
            session_id
        )));
    };

    let mut messages = Vec::new();
    for turn in session.all_turns() {
        messages.push(MessageInfo {
            role: "user".to_string(),
            content: turn.user_message.clone(),
            timestamp: turn.started_at.to_rfc3339(),
            metadata: None,
        });
        // Emit tool calls and their results
        for tool_call in &turn.tool_calls {
            messages.push(MessageInfo {
                role: "tool_use".to_string(),
                content: String::new(),
                timestamp: turn.started_at.to_rfc3339(),
                metadata: Some(serde_json::json!({
                    "tool_id": tool_call.id,
                    "name": tool_call.name,
                    "arguments": tool_call.arguments,
                })),
            });
            // Find matching result
            if let Some(result) = turn
                .tool_results
                .iter()
                .find(|r| r.tool_call_id == tool_call.id)
            {
                messages.push(MessageInfo {
                    role: "tool_result".to_string(),
                    content: result.content.clone(),
                    timestamp: turn.started_at.to_rfc3339(),
                    metadata: Some(serde_json::json!({
                        "tool_call_id": result.tool_call_id,
                        "success": result.success,
                    })),
                });
            }
        }
        if let Some(ref response) = turn.assistant_response {
            messages.push(MessageInfo {
                role: "assistant".to_string(),
                content: response.clone(),
                timestamp: turn
                    .completed_at
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| turn.started_at.to_rfc3339()),
                metadata: None,
            });
        }
    }

    let count = messages.len();

    Ok(Json(SessionMessagesResponse {
        session_id: session_id.clone(),
        messages,
        count,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn parse_session_id(s: &str) -> Result<SessionId, ServerError> {
    uuid::Uuid::parse_str(s)
        .map(SessionId::from_uuid)
        .map_err(|_| ServerError::BadRequest(format!("Invalid session ID: {}", s)))
}

fn session_to_detail(session: &Session) -> SessionDetail {
    session_to_detail_with_migration(session, None, None, None)
}

fn session_to_detail_with_migration(
    session: &Session,
    workstream_id: Option<String>,
    files_migrated: Option<usize>,
    allowed_paths: Option<Vec<String>>,
) -> SessionDetail {
    SessionDetail {
        id: session.id.to_string(),
        turns: session
            .all_turns()
            .iter()
            .map(|t| TurnInfo {
                id: t.id.to_string(),
                user_message: t.user_message.clone(),
                assistant_response: t.assistant_response.clone(),
                tool_call_count: t.tool_calls.len(),
                started_at: t.started_at.to_rfc3339(),
                completed_at: t.completed_at.map(|dt| dt.to_rfc3339()),
            })
            .collect(),
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
        metadata: session.metadata.clone(),
        workstream_id,
        files_migrated,
        allowed_paths,
    }
}

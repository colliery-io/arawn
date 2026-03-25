use std::path::Path as StdPath;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};

use arawn_domain::{
    DirectoryError, DirectoryManager, MessageRole, SCRATCH_ID, WorkstreamManager, WorkstreamMessage,
};

mod types;
pub use types::*;

#[cfg(test)]
mod tests;

use super::pagination::PaginationParams;
use crate::error::ServerError;
use crate::state::AppState;

/// Validate a workstream ID from a URL path parameter.
///
/// Rejects IDs containing path traversal sequences, path separators,
/// or other invalid characters. Valid IDs match `^[a-zA-Z0-9_-]+$`.
fn validate_id(id: &str) -> Result<(), ServerError> {
    DirectoryManager::validate_workstream_id(id).map_err(|_| {
        ServerError::BadRequest(format!(
            "Invalid workstream ID: '{}'. IDs must contain only alphanumeric characters, hyphens, and underscores.",
            id
        ))
    })
}

// ── Helpers ─────────────────────────────────────────────────────────

fn get_manager(state: &AppState) -> Result<&Arc<WorkstreamManager>, ServerError> {
    state
        .workstreams()
        .ok_or_else(|| ServerError::ServiceUnavailable("Workstreams not configured".to_string()))
}

fn to_workstream_response(
    ws: &arawn_domain::Workstream,
    tags: Option<Vec<String>>,
) -> WorkstreamResponse {
    WorkstreamResponse {
        id: ws.id.clone(),
        title: ws.title.clone(),
        summary: ws.summary.clone(),
        state: ws.state.clone(),
        default_model: ws.default_model.clone(),
        is_scratch: ws.is_scratch,
        created_at: ws.created_at.to_rfc3339(),
        updated_at: ws.updated_at.to_rfc3339(),
        tags,
    }
}

fn to_message_response(msg: &WorkstreamMessage) -> MessageResponse {
    MessageResponse {
        id: msg.id.clone(),
        workstream_id: msg.workstream_id.clone(),
        session_id: msg.session_id.clone(),
        role: msg.role.to_string(),
        content: msg.content.clone(),
        timestamp: msg.timestamp.to_rfc3339(),
        metadata: msg.metadata.clone(),
    }
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /api/v1/workstreams - Create a new workstream.
#[utoipa::path(
    post,
    path = "/api/v1/workstreams",
    request_body = CreateWorkstreamRequest,
    responses(
        (status = 201, description = "Workstream created", body = WorkstreamResponse),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Workstreams not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn create_workstream_handler(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkstreamRequest>,
) -> Result<(StatusCode, Json<WorkstreamResponse>), ServerError> {
    let mgr = get_manager(&state)?;

    let ws = mgr.create_workstream(&req.title, req.default_model.as_deref(), &req.tags)?;

    // Create directory structure for the new workstream
    if let Some(dm) = state.directory_manager() {
        dm.create_workstream(&ws.id).map_err(|e| {
            tracing::warn!(workstream = %ws.id, error = %e, "Failed to create workstream directories");
            ServerError::Internal(format!("Failed to create workstream directories: {}", e))
        })?;
    }

    let tags = mgr.get_tags(&ws.id).ok();
    Ok((StatusCode::CREATED, Json(to_workstream_response(&ws, tags))))
}

/// GET /api/v1/workstreams - List all workstreams.
#[utoipa::path(
    get,
    path = "/api/v1/workstreams",
    params(
        PaginationParams,
        ("include_archived" = Option<bool>, Query, description = "Include archived workstreams"),
    ),
    responses(
        (status = 200, description = "List of workstreams", body = WorkstreamListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Workstreams not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn list_workstreams_handler(
    State(state): State<AppState>,
    Query(query): Query<ListWorkstreamsQuery>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<WorkstreamListResponse>, ServerError> {
    let mgr = get_manager(&state)?;

    let list = if query.include_archived {
        mgr.list_all_workstreams()?
    } else {
        mgr.list_workstreams()?
    };
    let all_workstreams: Vec<_> = list
        .iter()
        .map(|ws| to_workstream_response(ws, None))
        .collect();

    let (paginated, total) = pagination.paginate(&all_workstreams);

    Ok(Json(WorkstreamListResponse {
        workstreams: paginated,
        total,
        limit: pagination.effective_limit(),
        offset: pagination.offset,
    }))
}

/// GET /api/v1/workstreams/:id - Get a workstream by ID.
#[utoipa::path(
    get,
    path = "/api/v1/workstreams/{id}",
    params(
        ("id" = String, Path, description = "Workstream ID"),
    ),
    responses(
        (status = 200, description = "Workstream details", body = WorkstreamResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream not found"),
        (status = 503, description = "Workstreams not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn get_workstream_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<WorkstreamResponse>, ServerError> {
    validate_id(&id)?;
    let mgr = get_manager(&state)?;

    let ws = mgr.get_workstream(&id)?;
    let tags = mgr.get_tags(&ws.id).ok();

    Ok(Json(to_workstream_response(&ws, tags)))
}

/// DELETE /api/v1/workstreams/:id - Archive a workstream.
#[utoipa::path(
    delete,
    path = "/api/v1/workstreams/{id}",
    params(
        ("id" = String, Path, description = "Workstream ID"),
    ),
    responses(
        (status = 204, description = "Workstream archived"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream not found"),
        (status = 503, description = "Workstreams not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn delete_workstream_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ServerError> {
    validate_id(&id)?;
    let mgr = get_manager(&state)?;

    mgr.archive_workstream(&id)?;

    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /api/v1/workstreams/:id - Update a workstream.
#[utoipa::path(
    patch,
    path = "/api/v1/workstreams/{id}",
    params(
        ("id" = String, Path, description = "Workstream ID"),
    ),
    request_body = UpdateWorkstreamRequest,
    responses(
        (status = 200, description = "Workstream updated", body = WorkstreamResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream not found"),
        (status = 503, description = "Workstreams not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn update_workstream_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorkstreamRequest>,
) -> Result<Json<WorkstreamResponse>, ServerError> {
    validate_id(&id)?;
    let mgr = get_manager(&state)?;

    // Update workstream fields
    let ws = mgr.update_workstream(
        &id,
        req.title.as_deref(),
        req.summary.as_deref(),
        req.default_model.as_deref(),
    )?;

    // Update tags if provided
    if let Some(ref tags) = req.tags {
        mgr.set_tags(&id, tags)?;
    }

    let tags = mgr.get_tags(&ws.id).ok();
    Ok(Json(to_workstream_response(&ws, tags)))
}

/// GET /api/v1/workstreams/:id/sessions - List sessions for a workstream.
#[utoipa::path(
    get,
    path = "/api/v1/workstreams/{id}/sessions",
    params(
        ("id" = String, Path, description = "Workstream ID"),
        PaginationParams,
    ),
    responses(
        (status = 200, description = "List of sessions", body = SessionListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream not found"),
        (status = 503, description = "Workstreams not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn list_workstream_sessions_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<SessionListResponse>, ServerError> {
    validate_id(&id)?;
    let mgr = get_manager(&state)?;

    let ws_sessions = mgr.list_sessions(&id)?;
    let all_sessions: Vec<_> = ws_sessions
        .iter()
        .map(|s| SessionResponse {
            id: s.id.clone(),
            workstream_id: s.workstream_id.clone(),
            started_at: s.started_at.to_rfc3339(),
            ended_at: s.ended_at.map(|dt| dt.to_rfc3339()),
            is_active: s.ended_at.is_none(),
        })
        .collect();

    let (paginated, total) = pagination.paginate(&all_sessions);

    Ok(Json(SessionListResponse {
        sessions: paginated,
        total,
        limit: pagination.effective_limit(),
        offset: pagination.offset,
    }))
}

/// POST /api/v1/workstreams/:id/messages - Send a message to a workstream.
#[utoipa::path(
    post,
    path = "/api/v1/workstreams/{id}/messages",
    params(
        ("id" = String, Path, description = "Workstream ID"),
    ),
    request_body = SendMessageRequest,
    responses(
        (status = 201, description = "Message sent", body = MessageResponse),
        (status = 400, description = "Invalid role"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream not found"),
        (status = 503, description = "Workstreams not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn send_message_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), ServerError> {
    validate_id(&id)?;
    let mgr = get_manager(&state)?;

    let role = match req.role.as_deref().unwrap_or("user") {
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        "agent_push" => MessageRole::AgentPush,
        other => {
            return Err(ServerError::BadRequest(format!("Invalid role: {other}")));
        }
    };

    let msg = mgr.send_message(Some(&id), None, role, &req.content, req.metadata.as_deref())?;

    Ok((StatusCode::CREATED, Json(to_message_response(&msg))))
}

/// GET /api/v1/workstreams/:id/messages - List messages for a workstream.
#[utoipa::path(
    get,
    path = "/api/v1/workstreams/{id}/messages",
    params(
        ("id" = String, Path, description = "Workstream ID"),
        ("since" = Option<String>, Query, description = "Only return messages after this timestamp (RFC 3339)"),
        PaginationParams,
    ),
    responses(
        (status = 200, description = "List of messages", body = MessageListResponse),
        (status = 400, description = "Invalid timestamp format"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream not found"),
        (status = 503, description = "Workstreams not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn list_messages_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<MessageQuery>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<MessageListResponse>, ServerError> {
    validate_id(&id)?;
    let mgr = get_manager(&state)?;

    let result = if let Some(since_str) = &query.since {
        let since = since_str.parse::<DateTime<Utc>>().map_err(|_| {
            ServerError::BadRequest("Invalid 'since' timestamp. Use RFC 3339 format.".to_string())
        })?;
        mgr.get_messages_since(&id, since)
    } else {
        mgr.get_messages(&id)
    };

    let msgs = result?;
    let all_messages: Vec<_> = msgs.iter().map(to_message_response).collect();

    let (paginated, total) = pagination.paginate(&all_messages);

    Ok(Json(MessageListResponse {
        messages: paginated,
        total,
        limit: pagination.effective_limit(),
        offset: pagination.offset,
    }))
}

/// POST /api/v1/workstreams/:id/promote - Promote scratch workstream.
#[utoipa::path(
    post,
    path = "/api/v1/workstreams/{id}/promote",
    params(
        ("id" = String, Path, description = "Workstream ID (must be scratch)"),
    ),
    request_body = PromoteRequest,
    responses(
        (status = 201, description = "Workstream promoted", body = WorkstreamResponse),
        (status = 400, description = "Only scratch workstream can be promoted"),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Workstreams not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn promote_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<PromoteRequest>,
) -> Result<(StatusCode, Json<WorkstreamResponse>), ServerError> {
    validate_id(&id)?;
    let mgr = get_manager(&state)?;

    if id != SCRATCH_ID {
        return Err(ServerError::BadRequest(
            "Only the scratch workstream can be promoted".to_string(),
        ));
    }

    let ws = mgr.promote_scratch(&req.title, &req.tags, req.default_model.as_deref())?;

    let tags = mgr.get_tags(&ws.id).ok();
    Ok((StatusCode::CREATED, Json(to_workstream_response(&ws, tags))))
}

/// POST /api/v1/workstreams/:id/files/promote - Promote a file to production.
#[utoipa::path(
    post,
    path = "/api/v1/workstreams/{id}/files/promote",
    params(
        ("id" = String, Path, description = "Workstream ID"),
    ),
    request_body = PromoteFileRequest,
    responses(
        (status = 201, description = "File promoted", body = PromoteFileResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream or file not found"),
        (status = 503, description = "Workstreams or directory management not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn promote_file_handler(
    State(state): State<AppState>,
    Path(workstream_id): Path<String>,
    Json(req): Json<PromoteFileRequest>,
) -> Result<(StatusCode, Json<PromoteFileResponse>), ServerError> {
    validate_id(&workstream_id)?;
    let mgr = get_manager(&state)?;

    // Get the directory manager
    let dir_mgr = mgr.directory_manager().ok_or_else(|| {
        ServerError::ServiceUnavailable("Directory management not configured".to_string())
    })?;

    // Promote the file
    let result = dir_mgr
        .promote(
            &workstream_id,
            StdPath::new(&req.source),
            StdPath::new(&req.destination),
        )
        .map_err(|e| match e {
            DirectoryError::WorkstreamNotFound(ws) => {
                ServerError::NotFound(format!("Workstream not found: {ws}"))
            }
            DirectoryError::SourceNotFound(path) => {
                ServerError::NotFound(format!("Source file not found: {}", path.display()))
            }
            DirectoryError::NotAFile(path) => {
                ServerError::BadRequest(format!("Source is not a file: {}", path.display()))
            }
            DirectoryError::InvalidName(name) => {
                ServerError::BadRequest(format!("Invalid workstream name: {name}"))
            }
            other => ServerError::Internal(format!("File promotion failed: {other}")),
        })?;

    // Calculate relative path for response
    let prod_path = dir_mgr.production_path(&workstream_id);
    let relative_path = result
        .path
        .strip_prefix(&prod_path)
        .unwrap_or(&result.path)
        .to_string_lossy()
        .to_string();

    // Notify connected clients about the file promotion
    if result.renamed {
        tracing::info!(
            workstream_id = %workstream_id,
            path = %relative_path,
            "File promoted with rename"
        );
    }

    Ok((
        StatusCode::CREATED,
        Json(PromoteFileResponse {
            path: relative_path,
            bytes: result.bytes,
            renamed: result.renamed,
        }),
    ))
}

/// POST /api/v1/workstreams/:id/files/export - Export a file to external path.
#[utoipa::path(
    post,
    path = "/api/v1/workstreams/{id}/files/export",
    params(
        ("id" = String, Path, description = "Workstream ID"),
    ),
    request_body = ExportFileRequest,
    responses(
        (status = 201, description = "File exported", body = ExportFileResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream or file not found"),
        (status = 503, description = "Workstreams or directory management not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn export_file_handler(
    State(state): State<AppState>,
    Path(workstream_id): Path<String>,
    Json(req): Json<ExportFileRequest>,
) -> Result<(StatusCode, Json<ExportFileResponse>), ServerError> {
    validate_id(&workstream_id)?;
    let mgr = get_manager(&state)?;

    // Get the directory manager
    let dir_mgr = mgr.directory_manager().ok_or_else(|| {
        ServerError::ServiceUnavailable("Directory management not configured".to_string())
    })?;

    // Export the file
    let result = dir_mgr
        .export(
            &workstream_id,
            StdPath::new(&req.source),
            StdPath::new(&req.destination),
        )
        .map_err(|e| match e {
            DirectoryError::WorkstreamNotFound(ws) => {
                ServerError::NotFound(format!("Workstream not found: {ws}"))
            }
            DirectoryError::SourceNotFound(path) => {
                ServerError::NotFound(format!("Source file not found: {}", path.display()))
            }
            DirectoryError::NotAFile(path) => {
                ServerError::BadRequest(format!("Source is not a file: {}", path.display()))
            }
            DirectoryError::InvalidName(name) => {
                ServerError::BadRequest(format!("Invalid workstream name: {name}"))
            }
            other => ServerError::Internal(format!("File export failed: {other}")),
        })?;

    Ok((
        StatusCode::CREATED,
        Json(ExportFileResponse {
            exported_to: result.path.to_string_lossy().to_string(),
            bytes: result.bytes,
        }),
    ))
}

/// POST /api/v1/workstreams/:id/clone - Clone a git repository.
#[utoipa::path(
    post,
    path = "/api/v1/workstreams/{id}/clone",
    params(
        ("id" = String, Path, description = "Workstream ID"),
    ),
    request_body = CloneRepoRequest,
    responses(
        (status = 201, description = "Repository cloned", body = CloneRepoResponse),
        (status = 400, description = "Clone failed"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream not found"),
        (status = 409, description = "Destination already exists"),
        (status = 503, description = "Git not available or workstreams not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn clone_repo_handler(
    State(state): State<AppState>,
    Path(workstream_id): Path<String>,
    Json(req): Json<CloneRepoRequest>,
) -> Result<(StatusCode, Json<CloneRepoResponse>), ServerError> {
    validate_id(&workstream_id)?;
    let mgr = get_manager(&state)?;

    // Get the directory manager
    let dir_mgr = mgr.directory_manager().ok_or_else(|| {
        ServerError::ServiceUnavailable("Directory management not configured".to_string())
    })?;

    // Clone the repository
    let result = dir_mgr
        .clone_repo(&workstream_id, &req.url, req.name.as_deref())
        .map_err(|e| match e {
            DirectoryError::WorkstreamNotFound(ws) => {
                ServerError::NotFound(format!("Workstream not found: {ws}"))
            }
            DirectoryError::AlreadyExists(path) => {
                ServerError::Conflict(format!("Destination already exists: {}", path.display()))
            }
            DirectoryError::GitNotFound => {
                ServerError::ServiceUnavailable("Git is not installed or not in PATH".to_string())
            }
            DirectoryError::CloneFailed { url, stderr } => {
                ServerError::BadRequest(format!("Clone failed for {url}: {stderr}"))
            }
            DirectoryError::InvalidName(name) => {
                ServerError::BadRequest(format!("Invalid workstream name: {name}"))
            }
            other => ServerError::Internal(format!("Clone failed: {other}")),
        })?;

    // Calculate relative path for response
    let prod_path = dir_mgr.production_path(&workstream_id);
    let relative_path = result
        .path
        .strip_prefix(&prod_path)
        .unwrap_or(&result.path)
        .to_string_lossy()
        .to_string();

    Ok((
        StatusCode::CREATED,
        Json(CloneRepoResponse {
            path: relative_path,
            commit: result.commit,
        }),
    ))
}

/// GET /api/v1/workstreams/:id/usage - Get disk usage statistics.
#[utoipa::path(
    get,
    path = "/api/v1/workstreams/{id}/usage",
    params(
        ("id" = String, Path, description = "Workstream ID"),
    ),
    responses(
        (status = 200, description = "Usage statistics", body = UsageResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream not found"),
        (status = 503, description = "Workstreams or directory management not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn get_usage_handler(
    State(state): State<AppState>,
    Path(workstream_id): Path<String>,
) -> Result<Json<UsageResponse>, ServerError> {
    validate_id(&workstream_id)?;
    let mgr = get_manager(&state)?;

    // Get the directory manager
    let dir_mgr = mgr.directory_manager().ok_or_else(|| {
        ServerError::ServiceUnavailable("Directory management not configured".to_string())
    })?;

    // Get usage statistics
    let stats = dir_mgr.get_usage(&workstream_id).map_err(|e| match e {
        DirectoryError::WorkstreamNotFound(ws) => {
            ServerError::NotFound(format!("Workstream not found: {ws}"))
        }
        DirectoryError::InvalidName(name) => {
            ServerError::BadRequest(format!("Invalid workstream name: {name}"))
        }
        other => ServerError::Internal(format!("Failed to get usage stats: {other}")),
    })?;

    // Convert session usages to response format
    let sessions: Vec<SessionUsageResponse> = stats
        .sessions
        .iter()
        .map(|s| SessionUsageResponse {
            id: s.id.clone(),
            mb: s.bytes as f64 / 1_048_576.0,
        })
        .collect();

    Ok(Json(UsageResponse {
        production_mb: stats.production_mb(),
        work_mb: stats.work_mb(),
        sessions,
        total_mb: stats.total_mb(),
        warnings: stats.warnings,
    }))
}

/// POST /api/v1/workstreams/:id/cleanup - Clean up work directory.
///
/// Does NOT delete from production/ (safety feature).
/// If more than 100 files would be deleted, requires `confirm: true` in the request.
#[utoipa::path(
    post,
    path = "/api/v1/workstreams/{id}/cleanup",
    params(
        ("id" = String, Path, description = "Workstream ID"),
    ),
    request_body = CleanupRequest,
    responses(
        (status = 200, description = "Cleanup result", body = CleanupResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream not found"),
        (status = 503, description = "Workstreams or directory management not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn cleanup_handler(
    State(state): State<AppState>,
    Path(workstream_id): Path<String>,
    Json(req): Json<CleanupRequest>,
) -> Result<Json<CleanupResponse>, ServerError> {
    validate_id(&workstream_id)?;
    let mgr = get_manager(&state)?;

    // Get the directory manager
    let dir_mgr = mgr.directory_manager().ok_or_else(|| {
        ServerError::ServiceUnavailable("Directory management not configured".to_string())
    })?;

    // Perform cleanup
    let result = dir_mgr
        .cleanup_work(&workstream_id, req.older_than_days, req.confirm)
        .map_err(|e| match e {
            DirectoryError::WorkstreamNotFound(ws) => {
                ServerError::NotFound(format!("Workstream not found: {ws}"))
            }
            DirectoryError::InvalidName(name) => {
                ServerError::BadRequest(format!("Invalid workstream name: {name}"))
            }
            other => ServerError::Internal(format!("Cleanup failed: {other}")),
        })?;

    Ok(Json(CleanupResponse {
        deleted_files: result.deleted_files,
        freed_mb: result.freed_mb(),
        pending_files: result.pending_files,
        requires_confirmation: result.requires_confirmation,
    }))
}

/// POST /api/v1/workstreams/:id/compress - Trigger manual compression.
///
/// Compresses all ended sessions in the workstream and produces a unified summary.
#[utoipa::path(
    post,
    path = "/api/v1/workstreams/{id}/compress",
    params(
        ("id" = String, Path, description = "Workstream ID"),
    ),
    responses(
        (status = 200, description = "Compression complete", body = CompressResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workstream not found"),
        (status = 503, description = "Compression or workstreams not configured"),
    ),
    security(("bearer_auth" = [])),
    tag = "workstreams"
)]
pub async fn compress_workstream_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CompressResponse>, ServerError> {
    validate_id(&id)?;
    let mgr = get_manager(&state)?;

    let compressor = state
        .compressor()
        .ok_or_else(|| ServerError::ServiceUnavailable("Compression not configured".to_string()))?;

    // Verify workstream exists
    mgr.get_workstream(&id)?;

    // Compress all ended sessions that haven't been compressed yet
    let sessions = mgr.list_sessions(&id)?;
    let mut compressed_count = 0;

    for session in &sessions {
        if session.ended_at.is_some() && !session.compressed {
            match compressor.compress_session(mgr, &session.id).await {
                Ok(_) => compressed_count += 1,
                Err(e) => {
                    tracing::warn!(
                        session_id = %session.id,
                        error = %e,
                        "Failed to compress session"
                    );
                }
            }
        }
    }

    // Reduce all session summaries into a workstream summary
    let summary = compressor
        .compress_workstream(mgr, &id)
        .await
        .map_err(|e| ServerError::Internal(format!("Workstream compression failed: {e}")))?;

    Ok(Json(CompressResponse {
        summary,
        sessions_compressed: compressed_count,
    }))
}

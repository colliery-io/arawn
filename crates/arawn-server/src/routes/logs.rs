//! Server logs endpoint.
//!
//! Provides access to operational log files so that remote clients (TUI, CLI)
//! can fetch recent server log entries without direct filesystem access.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use axum::{Extension, Json, extract::Query, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::auth::Identity;
use crate::error::ServerError;
use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Query parameters for the logs endpoint.
#[derive(Debug, Deserialize, IntoParams)]
pub struct LogsQuery {
    /// Number of lines to return (default: 50, max: 1000).
    pub lines: Option<usize>,
    /// Log file name (without extension). Defaults to the latest daily log.
    pub file: Option<String>,
}

/// A single log entry.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogEntry {
    /// Raw log line content.
    pub line: String,
}

/// Response for the logs endpoint.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogsResponse {
    /// Log file that was read.
    pub file: String,
    /// Total number of lines returned.
    pub count: usize,
    /// The log entries (tail of file).
    pub entries: Vec<LogEntry>,
}

/// Response listing available log files.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogFilesResponse {
    /// Available log files.
    pub files: Vec<LogFileInfo>,
}

/// Info about a log file.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogFileInfo {
    /// File name (without directory).
    pub name: String,
    /// File size in bytes.
    pub size: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn log_dir() -> Result<PathBuf, ServerError> {
    // Check ARAWN_CONFIG_DIR env var first, then fall back to ~/.config/arawn
    let config_dir = if let Ok(dir) = std::env::var("ARAWN_CONFIG_DIR") {
        if !dir.is_empty() {
            Some(PathBuf::from(dir))
        } else {
            None
        }
    } else {
        dirs::home_dir().map(|d| d.join(".config").join("arawn"))
    };

    config_dir
        .map(|d| d.join("logs"))
        .ok_or_else(|| ServerError::Internal("Could not determine log directory".to_string()))
}

fn find_latest_log(log_dir: &std::path::Path) -> Result<PathBuf, ServerError> {
    let mut entries: Vec<_> = std::fs::read_dir(log_dir)
        .map_err(|e| ServerError::Internal(format!("Failed to read log directory: {}", e)))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "log"))
        .collect();

    entries.sort_by_key(|e| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok()));

    entries
        .first()
        .map(|e| e.path())
        .ok_or_else(|| ServerError::NotFound("No log files found".to_string()))
}

fn resolve_log_file(log_dir: &std::path::Path, name: Option<&str>) -> Result<PathBuf, ServerError> {
    match name {
        Some(name) => {
            let path = log_dir.join(format!("{}.log", name));
            if path.exists() {
                return Ok(path);
            }
            let exact = log_dir.join(name);
            if exact.exists() {
                return Ok(exact);
            }
            Err(ServerError::NotFound(format!(
                "Log file not found: {}",
                name
            )))
        }
        None => find_latest_log(log_dir),
    }
}

fn tail_lines(path: &std::path::Path, n: usize) -> Result<Vec<String>, ServerError> {
    let file = std::fs::File::open(path)
        .map_err(|e| ServerError::Internal(format!("Failed to open log file: {}", e)))?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ServerError::Internal(format!("Failed to read log file: {}", e)))?;

    let start = lines.len().saturating_sub(n);
    Ok(lines[start..].to_vec())
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/v1/logs - Get recent server log entries.
#[utoipa::path(
    get,
    path = "/api/v1/logs",
    params(LogsQuery),
    responses(
        (status = 200, description = "Recent log entries", body = LogsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Log file not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "logs"
)]
pub async fn get_logs_handler(
    State(_state): State<AppState>,
    Extension(_identity): Extension<Identity>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<LogsResponse>, ServerError> {
    let dir = log_dir()?;
    if !dir.exists() {
        return Err(ServerError::NotFound("Log directory not found".to_string()));
    }

    let n = query.lines.unwrap_or(50).min(1000);
    let log_file = resolve_log_file(&dir, query.file.as_deref())?;

    let file_name = log_file
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    let lines = tail_lines(&log_file, n)?;
    let count = lines.len();
    let entries = lines.into_iter().map(|line| LogEntry { line }).collect();

    Ok(Json(LogsResponse {
        file: file_name,
        count,
        entries,
    }))
}

/// GET /api/v1/logs/files - List available log files.
#[utoipa::path(
    get,
    path = "/api/v1/logs/files",
    responses(
        (status = 200, description = "Available log files", body = LogFilesResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "logs"
)]
pub async fn list_log_files_handler(
    State(_state): State<AppState>,
    Extension(_identity): Extension<Identity>,
) -> Result<Json<LogFilesResponse>, ServerError> {
    let dir = log_dir()?;
    if !dir.exists() {
        return Ok(Json(LogFilesResponse { files: vec![] }));
    }

    let mut files: Vec<LogFileInfo> = std::fs::read_dir(&dir)
        .map_err(|e| ServerError::Internal(format!("Failed to read log directory: {}", e)))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "log"))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let size = e.metadata().ok()?.len();
            Some(LogFileInfo { name, size })
        })
        .collect();

    files.sort_by(|a, b| b.name.cmp(&a.name));

    Ok(Json(LogFilesResponse { files }))
}

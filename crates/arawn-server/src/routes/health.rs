//! Health check endpoints.

use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::state::AppState;

/// Health check response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Service status.
    pub status: String,
    /// Service version.
    pub version: String,
}

/// Deep health check response with subsystem status.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeepHealthResponse {
    /// Overall status ("ok" or "degraded").
    pub status: String,
    /// Service version.
    pub version: String,
    /// Individual subsystem checks.
    pub checks: Vec<HealthCheck>,
}

/// Status of an individual subsystem.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthCheck {
    /// Subsystem name.
    pub name: String,
    /// Status ("ok", "warn", "error", "unavailable").
    pub status: String,
    /// Optional detail message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Simple health check (no auth required).
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
    ),
    tag = "health"
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Deep health check — verifies subsystem connectivity (no auth required).
///
/// Checks:
/// - Memory store: SQLite connectivity via `SELECT 1`
/// - Session cache: reports size
/// - Disk: checks available space on the config directory
#[utoipa::path(
    get,
    path = "/health/deep",
    responses(
        (status = 200, description = "Deep health check results", body = DeepHealthResponse),
    ),
    tag = "health"
)]
pub async fn health_deep(State(state): State<AppState>) -> Json<DeepHealthResponse> {
    let mut checks = Vec::new();
    let mut overall_ok = true;

    // Check memory store connectivity
    if let Some(store) = state.memory_store() {
        match store.stats() {
            Ok(stats) => {
                checks.push(HealthCheck {
                    name: "memory_store".to_string(),
                    status: "ok".to_string(),
                    detail: Some(format!("{} memories stored", stats.memory_count)),
                });
            }
            Err(e) => {
                overall_ok = false;
                checks.push(HealthCheck {
                    name: "memory_store".to_string(),
                    status: "error".to_string(),
                    detail: Some(format!("SQLite query failed: {}", e)),
                });
            }
        }
    } else {
        checks.push(HealthCheck {
            name: "memory_store".to_string(),
            status: "unavailable".to_string(),
            detail: Some("Not initialized".to_string()),
        });
    }

    // Check session cache
    let cache_size = state.session_cache().len().await;
    checks.push(HealthCheck {
        name: "session_cache".to_string(),
        status: "ok".to_string(),
        detail: Some(format!("{} active sessions", cache_size)),
    });

    // Check disk space on config directory
    if let Some(config_dir) = dirs::config_dir().map(|d| d.join("arawn")) {
        match fs_available_bytes(&config_dir) {
            Some(available) => {
                let gb = available as f64 / (1024.0 * 1024.0 * 1024.0);
                let (status, detail) = if gb < 1.0 {
                    overall_ok = false;
                    ("warn".to_string(), format!("{:.1} GB available (low)", gb))
                } else {
                    ("ok".to_string(), format!("{:.1} GB available", gb))
                };
                checks.push(HealthCheck {
                    name: "disk".to_string(),
                    status,
                    detail: Some(detail),
                });
            }
            None => {
                checks.push(HealthCheck {
                    name: "disk".to_string(),
                    status: "unavailable".to_string(),
                    detail: Some("Could not query disk space".to_string()),
                });
            }
        }
    }

    Json(DeepHealthResponse {
        status: if overall_ok {
            "ok".to_string()
        } else {
            "degraded".to_string()
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks,
    })
}

/// Basic runtime metrics response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MetricsResponse {
    /// Server uptime in seconds.
    pub uptime_seconds: u64,
    /// Number of active WebSocket connections.
    pub active_ws_connections: usize,
    /// Number of cached sessions.
    pub cached_sessions: usize,
    /// Memory store statistics (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_stats: Option<MemoryMetrics>,
    /// Disk available in bytes on config partition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_available_bytes: Option<u64>,
}

/// Memory subsystem metrics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MemoryMetrics {
    pub memories: usize,
    pub sessions: usize,
    pub notes: usize,
    pub embeddings: usize,
}

/// Basic runtime metrics (no auth required).
#[utoipa::path(
    get,
    path = "/metrics",
    responses(
        (status = 200, description = "Runtime metrics", body = MetricsResponse),
    ),
    tag = "health"
)]
pub async fn metrics(State(state): State<AppState>) -> Json<MetricsResponse> {
    let active_ws = state
        .active_connections()
        .read()
        .await
        .len();

    let cached_sessions = state.session_cache().len().await;

    let memory_stats = state.memory_store().and_then(|store| {
        store.stats().ok().map(|s| MemoryMetrics {
            memories: s.memory_count,
            sessions: s.session_count,
            notes: s.note_count,
            embeddings: s.embedding_count,
        })
    });

    let disk_available_bytes = dirs::config_dir()
        .map(|d| d.join("arawn"))
        .and_then(|p| fs_available_bytes(&p));

    Json(MetricsResponse {
        uptime_seconds: 0, // TODO: track server start time in AppState
        active_ws_connections: active_ws,
        cached_sessions,
        memory_stats,
        disk_available_bytes,
    })
}

/// Get available bytes on the filesystem containing the given path.
fn fs_available_bytes(path: &std::path::Path) -> Option<u64> {
    use fs2::available_space;
    available_space(path).ok()
}

/// Create health check routes.
pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/health/deep", get(health_deep))
        .route("/metrics", get(metrics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = Router::new().route("/health", get(health));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(health.status, "ok");
        assert!(!health.version.is_empty());
    }
}

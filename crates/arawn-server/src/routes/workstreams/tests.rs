use super::*;
use crate::auth::auth_middleware;
use crate::config::ServerConfig;
use arawn_domain::{Agent, ToolRegistry};
use arawn_llm::MockBackend;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::{get, post},
};
use tower::ServiceExt;

fn create_state_with_workstreams() -> (AppState, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();
    let config = ServerConfig::new(Some("test-token".to_string()));
    let state = AppState::new(agent, config);

    let ws_config = arawn_workstream::WorkstreamConfig {
        db_path: temp_dir.path().join("workstreams.db"),
        data_dir: temp_dir.path().join("workstreams"),
        session_timeout_minutes: 30,
    };
    let mgr = arawn_workstream::WorkstreamManager::new(&ws_config).unwrap();
    let state = state.with_workstreams(mgr);

    (state, temp_dir)
}

fn create_state_without_workstreams() -> AppState {
    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();
    AppState::new(agent, ServerConfig::new(Some("test-token".to_string())))
}

fn create_test_router(state: AppState) -> Router {
    Router::new()
        .route(
            "/workstreams",
            post(create_workstream_handler).get(list_workstreams_handler),
        )
        .route(
            "/workstreams/{id}",
            get(get_workstream_handler)
                .patch(update_workstream_handler)
                .delete(delete_workstream_handler),
        )
        .route(
            "/workstreams/{id}/sessions",
            get(list_workstream_sessions_handler),
        )
        .route(
            "/workstreams/{id}/messages",
            post(send_message_handler).get(list_messages_handler),
        )
        .route("/workstreams/{id}/promote", post(promote_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

fn auth_header() -> (&'static str, &'static str) {
    ("Authorization", "Bearer test-token")
}

// ── validate_id tests ──────────────────────────────────────────────────

#[test]
fn test_validate_id_valid() {
    assert!(validate_id("my-workstream").is_ok());
    assert!(validate_id("test_123").is_ok());
    assert!(validate_id("ABC").is_ok());
}

#[test]
fn test_validate_id_invalid() {
    assert!(validate_id("../etc/passwd").is_err());
    assert!(validate_id("foo/bar").is_err());
    assert!(validate_id("").is_err());
}

// ── Helper function tests ──────────────────────────────────────────────

#[test]
fn test_is_zero() {
    assert!(is_zero(&0));
    assert!(!is_zero(&1));
    assert!(!is_zero(&100));
}

// ── Handler tests (with workstreams) ────────────────────────────────────

#[tokio::test]
async fn test_list_workstreams_empty() {
    let (state, _tmp) = create_state_with_workstreams();
    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/workstreams")
                .header(key, val)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // May or may not have scratch yet (lazily created)
    assert!(result["total"].as_u64().is_some());
}

#[tokio::test]
async fn test_create_workstream() {
    let (state, _tmp) = create_state_with_workstreams();
    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/workstreams")
                .header(key, val)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"title": "Test WS"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["title"], "Test WS");
    assert_eq!(result["is_scratch"], false);
    assert_eq!(result["state"], "active");
}

#[tokio::test]
async fn test_get_workstream_scratch() {
    let (state, _tmp) = create_state_with_workstreams();
    // Ensure scratch exists by creating a session in it
    let mgr = state.workstreams().unwrap();
    mgr.create_workstream("scratch_test", None, &[]).ok();
    // Use the created workstream ID instead of scratch (which is lazily created)
    let ws = mgr.list_workstreams().unwrap();
    let ws_id = &ws[0].id;

    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/workstreams/{}", ws_id))
                .header(key, val)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["id"], ws_id.as_str());
}

#[tokio::test]
async fn test_get_workstream_not_found() {
    let (state, _tmp) = create_state_with_workstreams();
    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/workstreams/nonexistent")
                .header(key, val)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_workstream_invalid_id() {
    let (state, _tmp) = create_state_with_workstreams();
    let app = create_test_router(state);

    let (key, val) = auth_header();
    // Use a name with spaces/special chars that fails validate_id
    let response = app
        .oneshot(
            Request::builder()
                .uri("/workstreams/bad%20name!")
                .header(key, val)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_update_workstream() {
    let (state, _tmp) = create_state_with_workstreams();
    // Create a workstream first
    let mgr = state.workstreams().unwrap();
    let ws = mgr.create_workstream("Original", None, &[]).unwrap();

    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/workstreams/{}", ws.id))
                .header(key, val)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"title": "Updated"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["title"], "Updated");
}

#[tokio::test]
async fn test_delete_workstream() {
    let (state, _tmp) = create_state_with_workstreams();
    let mgr = state.workstreams().unwrap();
    let ws = mgr.create_workstream("To Delete", None, &[]).unwrap();

    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/workstreams/{}", ws.id))
                .header(key, val)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_workstream_not_found() {
    let (state, _tmp) = create_state_with_workstreams();
    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/workstreams/nonexistent")
                .header(key, val)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_workstream_sessions_empty() {
    let (state, _tmp) = create_state_with_workstreams();
    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/workstreams/scratch/sessions")
                .header(key, val)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["total"], 0);
}

#[tokio::test]
async fn test_send_message() {
    let (state, _tmp) = create_state_with_workstreams();
    // Create a workstream to send messages to
    let mgr = state.workstreams().unwrap();
    let ws = mgr.create_workstream("msg-test", None, &[]).unwrap();

    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workstreams/{}/messages", ws.id))
                .header(key, val)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"content": "Hello workstream"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["content"], "Hello workstream");
    assert_eq!(result["role"], "user");
}

#[tokio::test]
async fn test_send_message_invalid_role() {
    let (state, _tmp) = create_state_with_workstreams();
    let mgr = state.workstreams().unwrap();
    let ws = mgr.create_workstream("role-test", None, &[]).unwrap();
    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workstreams/{}/messages", ws.id))
                .header(key, val)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"role": "invalid", "content": "test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_messages_empty() {
    let (state, _tmp) = create_state_with_workstreams();
    let mgr = state.workstreams().unwrap();
    let ws = mgr.create_workstream("list-msg", None, &[]).unwrap();
    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/workstreams/{}/messages", ws.id))
                .header(key, val)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["total"], 0);
}

#[tokio::test]
async fn test_list_messages_invalid_since() {
    let (state, _tmp) = create_state_with_workstreams();
    let mgr = state.workstreams().unwrap();
    let ws = mgr.create_workstream("since-test", None, &[]).unwrap();
    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/workstreams/{}/messages?since=not-a-date", ws.id))
                .header(key, val)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_promote_non_scratch() {
    let (state, _tmp) = create_state_with_workstreams();
    let mgr = state.workstreams().unwrap();
    let ws = mgr.create_workstream("Named WS", None, &[]).unwrap();

    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workstreams/{}/promote", ws.id))
                .header(key, val)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"title": "Promoted"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ── Without workstreams configured ─────────────────────────────────────

#[tokio::test]
async fn test_list_workstreams_not_configured() {
    let state = create_state_without_workstreams();
    let app = create_test_router(state);

    let (key, val) = auth_header();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/workstreams")
                .header(key, val)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

// ── Type serialization tests ───────────────────────────────────────────

#[test]
fn test_create_workstream_request_deserialize() {
    let json = r#"{"title": "My WS", "tags": ["tag1"]}"#;
    let req: CreateWorkstreamRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.title, "My WS");
    assert_eq!(req.tags, vec!["tag1"]);
    assert!(req.default_model.is_none());
}

#[test]
fn test_send_message_request_deserialize() {
    let json = r#"{"content": "hello"}"#;
    let req: SendMessageRequest = serde_json::from_str(json).unwrap();
    assert!(req.role.is_none());
    assert_eq!(req.content, "hello");
    assert!(req.metadata.is_none());
}

#[test]
fn test_update_workstream_request_deserialize() {
    let json = r#"{"title": "New Title", "summary": "Summary text"}"#;
    let req: UpdateWorkstreamRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.title, Some("New Title".to_string()));
    assert_eq!(req.summary, Some("Summary text".to_string()));
    assert!(req.default_model.is_none());
    assert!(req.tags.is_none());
}

#[test]
fn test_workstream_response_serialization() {
    let resp = WorkstreamResponse {
        id: "ws-1".to_string(),
        title: "Test".to_string(),
        summary: None,
        state: "active".to_string(),
        default_model: None,
        is_scratch: false,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
        tags: None,
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"id\":\"ws-1\""));
    assert!(json.contains("\"is_scratch\":false"));
    // tags=None should be omitted
    assert!(!json.contains("\"tags\""));
}

#[test]
fn test_promote_file_response_serialization() {
    let resp = PromoteFileResponse {
        path: "output.txt".to_string(),
        bytes: 1024,
        renamed: false,
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"bytes\":1024"));
    // renamed=false should be omitted (skip_serializing_if = Not::not)
    assert!(!json.contains("\"renamed\""));
}

#[test]
fn test_cleanup_response_serialization() {
    let resp = CleanupResponse {
        deleted_files: 5,
        freed_mb: 12.5,
        pending_files: 0,
        requires_confirmation: false,
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"deleted_files\":5"));
    // pending_files=0 and requires_confirmation=false should be omitted
    assert!(!json.contains("\"pending_files\""));
    assert!(!json.contains("\"requires_confirmation\""));
}

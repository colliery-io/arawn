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

fn create_test_state() -> AppState {
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
            "/sessions",
            post(create_session_handler).get(list_sessions_handler),
        )
        .route(
            "/sessions/{id}",
            get(get_session_handler)
                .patch(update_session_handler)
                .delete(delete_session_handler),
        )
        .route("/sessions/{id}/messages", get(get_session_messages_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

#[tokio::test]
async fn test_list_sessions_empty() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sessions")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: ListSessionsResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.total, 0);
    assert!(result.sessions.is_empty());
    assert_eq!(result.offset, 0);
    assert!(result.limit > 0);
}

#[tokio::test]
async fn test_list_sessions_with_data() {
    let state = create_test_state();

    // Create some sessions
    state.get_or_create_session(None).await;
    state.get_or_create_session(None).await;

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sessions")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: ListSessionsResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.total, 2);
}

#[tokio::test]
async fn test_get_session() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/sessions/{}", session_id))
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: SessionDetail = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.id, session_id.to_string());
    assert!(result.turns.is_empty());
}

#[tokio::test]
async fn test_get_session_not_found() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sessions/00000000-0000-0000-0000-000000000000")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_session_invalid_id() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sessions/not-a-uuid")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_delete_session() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;

    let app = create_test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/sessions/{}", session_id))
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify deleted
    assert!(!state.session_cache().contains(&session_id).await);
}

#[tokio::test]
async fn test_delete_session_not_found() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/sessions/00000000-0000-0000-0000-000000000000")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_session() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/sessions")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"title": "Test Session"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: SessionDetail = serde_json::from_slice(&body).unwrap();
    assert!(!result.id.is_empty());
    assert!(result.metadata.contains_key("title"));
    assert_eq!(result.metadata["title"], "Test Session");
}

#[tokio::test]
async fn test_create_session_with_metadata() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/sessions")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"title": "My Session", "metadata": {"project": "test"}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: SessionDetail = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.metadata["project"], "test");
}

#[tokio::test]
async fn test_update_session() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/sessions/{}", session_id))
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"title": "Updated Title"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: SessionDetail = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.metadata["title"], "Updated Title");
}

#[tokio::test]
async fn test_update_session_not_found() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/sessions/00000000-0000-0000-0000-000000000000")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"title": "Test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_session_messages_empty() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/sessions/{}/messages", session_id))
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: SessionMessagesResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.session_id, session_id.to_string());
    assert!(result.messages.is_empty());
    assert_eq!(result.count, 0);
}

#[tokio::test]
async fn test_get_session_messages_with_data() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;

    // Add a turn with messages
    state
        .session_cache()
        .with_session_mut(&session_id, |session| {
            let turn = session.start_turn("Hello");
            turn.complete("Hi there!");
        })
        .await;

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/sessions/{}/messages", session_id))
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: SessionMessagesResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.count, 2);
    assert_eq!(result.messages[0].role, "user");
    assert_eq!(result.messages[0].content, "Hello");
    assert_eq!(result.messages[1].role, "assistant");
    assert_eq!(result.messages[1].content, "Hi there!");
}

#[tokio::test]
async fn test_get_session_messages_not_found() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sessions/00000000-0000-0000-0000-000000000000/messages")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_session_empty_body() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/sessions")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: SessionDetail = serde_json::from_slice(&body).unwrap();
    assert!(!result.id.is_empty());
    assert!(result.metadata.is_empty());
}

#[tokio::test]
async fn test_list_sessions_with_pagination() {
    let state = create_test_state();

    // Create 3 sessions
    state.get_or_create_session(None).await;
    state.get_or_create_session(None).await;
    state.get_or_create_session(None).await;

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sessions?limit=2&offset=0")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: ListSessionsResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.total, 3);
    assert_eq!(result.sessions.len(), 2);
    assert_eq!(result.limit, 2);
    assert_eq!(result.offset, 0);
}

#[tokio::test]
async fn test_list_sessions_with_offset() {
    let state = create_test_state();

    state.get_or_create_session(None).await;
    state.get_or_create_session(None).await;
    state.get_or_create_session(None).await;

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sessions?limit=10&offset=2")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: ListSessionsResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.total, 3);
    assert_eq!(result.sessions.len(), 1); // Only 1 remaining after offset 2
}

#[tokio::test]
async fn test_unauthorized_request() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_unauthorized_wrong_token() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sessions")
                .header("Authorization", "Bearer wrong-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_delete_session_invalid_id() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/sessions/not-a-uuid")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_update_session_merge_metadata() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;

    // Set initial metadata
    state
        .session_cache()
        .with_session_mut(&session_id, |session| {
            session.metadata.insert(
                "existing_key".to_string(),
                serde_json::Value::String("old_value".to_string()),
            );
        })
        .await;

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/sessions/{}", session_id))
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"metadata": {"new_key": "new_value"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: SessionDetail = serde_json::from_slice(&body).unwrap();
    // Both old and new keys should be present
    assert_eq!(result.metadata["existing_key"], "old_value");
    assert_eq!(result.metadata["new_key"], "new_value");
}

#[tokio::test]
async fn test_update_session_workstream_without_workstreams() {
    let state = create_test_state();
    let session_id = state.get_or_create_session(None).await;
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/sessions/{}", session_id))
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"workstream_id": "my-workstream"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should fail because workstreams not configured
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_session_messages_invalid_id() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sessions/bad-uuid/messages")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ── Helper function tests ──────────────────────────────────────────────

#[test]
fn test_parse_session_id_valid() {
    let uuid = uuid::Uuid::new_v4();
    let result = parse_session_id(&uuid.to_string());
    assert!(result.is_ok());
}

#[test]
fn test_parse_session_id_invalid() {
    let result = parse_session_id("not-a-uuid");
    assert!(result.is_err());
}

#[test]
fn test_session_to_detail_empty_session() {
    let session = Session::new();
    let detail = session_to_detail(&session);
    assert!(detail.turns.is_empty());
    assert!(detail.metadata.is_empty());
    assert!(detail.workstream_id.is_none());
    assert!(detail.files_migrated.is_none());
    assert!(detail.allowed_paths.is_none());
}

#[test]
fn test_session_to_detail_with_migration_info() {
    let session = Session::new();
    let detail = session_to_detail_with_migration(
        &session,
        Some("ws-1".to_string()),
        Some(5),
        Some(vec!["/tmp/work".to_string()]),
    );
    assert_eq!(detail.workstream_id, Some("ws-1".to_string()));
    assert_eq!(detail.files_migrated, Some(5));
    assert_eq!(detail.allowed_paths, Some(vec!["/tmp/work".to_string()]));
}

#[test]
fn test_session_to_detail_with_turns() {
    let mut session = Session::new();
    let turn = session.start_turn("Hello");
    turn.complete("World");
    let detail = session_to_detail(&session);
    assert_eq!(detail.turns.len(), 1);
    assert_eq!(detail.turns[0].user_message, "Hello");
    assert_eq!(
        detail.turns[0].assistant_response,
        Some("World".to_string())
    );
}

// ── Type serialization tests ───────────────────────────────────────────

#[test]
fn test_create_session_request_deserialize() {
    let json = r#"{"title": "Test"}"#;
    let req: CreateSessionRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.title, Some("Test".to_string()));
    assert!(req.metadata.is_empty());
}

#[test]
fn test_create_session_request_deserialize_minimal() {
    let json = r#"{}"#;
    let req: CreateSessionRequest = serde_json::from_str(json).unwrap();
    assert!(req.title.is_none());
    assert!(req.metadata.is_empty());
}

#[test]
fn test_update_session_request_deserialize() {
    let json = r#"{"title": "New", "workstream_id": "ws-1"}"#;
    let req: UpdateSessionRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.title, Some("New".to_string()));
    assert_eq!(req.workstream_id, Some("ws-1".to_string()));
    assert!(req.metadata.is_none());
}

#[test]
fn test_list_sessions_response_serialization() {
    let resp = ListSessionsResponse {
        sessions: vec![],
        total: 0,
        limit: 20,
        offset: 0,
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"total\":0"));
    assert!(json.contains("\"limit\":20"));
}

#[test]
fn test_session_messages_response_serialization() {
    let resp = SessionMessagesResponse {
        session_id: "abc".to_string(),
        messages: vec![MessageInfo {
            role: "user".to_string(),
            content: "hi".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            metadata: None,
        }],
        count: 1,
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"role\":\"user\""));
    assert!(json.contains("\"count\":1"));
    // metadata=None should be skipped
    assert!(!json.contains("\"metadata\""));
}

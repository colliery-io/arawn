use arawn_client::{
    ArawnClient, CreateSessionRequest, Error, ListSessionsResponse, SessionDetail,
    SessionMessagesResponse, UpdateSessionRequest,
};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_client(uri: &str) -> ArawnClient {
    ArawnClient::builder()
        .base_url(uri)
        .auth_token("test-token")
        .build()
        .unwrap()
}

// ─────────────────────────────────────────────────────────────────────────────
// List sessions
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_sessions_list() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("GET"))
        .and(path("/api/v1/sessions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sessions": [
                {
                    "id": "sess-001",
                    "title": "First session",
                    "turn_count": 3,
                    "created_at": "2026-03-01T10:00:00Z",
                    "updated_at": "2026-03-01T11:00:00Z"
                },
                {
                    "id": "sess-002",
                    "title": null,
                    "turn_count": 0,
                    "created_at": "2026-03-02T10:00:00Z",
                    "updated_at": "2026-03-02T10:00:00Z"
                }
            ],
            "total": 2
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp: ListSessionsResponse = client.sessions().list().await.unwrap();
    assert_eq!(resp.total, 2);
    assert_eq!(resp.sessions.len(), 2);
    assert_eq!(resp.sessions[0].id, "sess-001");
    assert_eq!(resp.sessions[0].title.as_deref(), Some("First session"));
    assert_eq!(resp.sessions[0].turn_count, 3);
    assert_eq!(resp.sessions[1].id, "sess-002");
    assert!(resp.sessions[1].title.is_none());
}

#[tokio::test]
async fn test_sessions_list_empty() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("GET"))
        .and(path("/api/v1/sessions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sessions": [],
            "total": 0
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client.sessions().list().await.unwrap();
    assert_eq!(resp.total, 0);
    assert!(resp.sessions.is_empty());
}

#[tokio::test]
async fn test_sessions_list_auth_header() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("GET"))
        .and(path("/api/v1/sessions"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sessions": [],
            "total": 0
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client.sessions().list().await.unwrap();
    assert_eq!(resp.total, 0);
}

#[tokio::test]
async fn test_sessions_list_server_error() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("GET"))
        .and(path("/api/v1/sessions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = client.sessions().list().await.unwrap_err();
    assert!(err.is_server_error());
    match &err {
        Error::Api {
            status,
            code,
            message,
        } => {
            assert_eq!(*status, 500);
            assert_eq!(code, "internal_error");
            assert_eq!(message, "Internal server error");
        }
        other => panic!("Expected Error::Api, got {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Get session
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_sessions_get() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("GET"))
        .and(path("/api/v1/sessions/sess-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "sess-abc",
            "turns": [],
            "created_at": "2026-03-01T10:00:00Z",
            "updated_at": "2026-03-01T10:00:00Z",
            "metadata": {}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let detail: SessionDetail = client.sessions().get("sess-abc").await.unwrap();
    assert_eq!(detail.id, "sess-abc");
    assert!(detail.turns.is_empty());
    assert!(detail.metadata.is_empty());
}

#[tokio::test]
async fn test_sessions_get_with_turns() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("GET"))
        .and(path("/api/v1/sessions/sess-turns"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "sess-turns",
            "turns": [
                {
                    "id": "turn-001",
                    "user_message": "Hello, how are you?",
                    "assistant_response": "I'm doing well, thanks!",
                    "tool_call_count": 0,
                    "started_at": "2026-03-01T10:00:00Z",
                    "completed_at": "2026-03-01T10:00:05Z"
                },
                {
                    "id": "turn-002",
                    "user_message": "Run a search for me",
                    "assistant_response": null,
                    "tool_call_count": 2,
                    "started_at": "2026-03-01T10:01:00Z",
                    "completed_at": null
                }
            ],
            "created_at": "2026-03-01T10:00:00Z",
            "updated_at": "2026-03-01T10:01:00Z",
            "metadata": {
                "project": "arawn"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let detail = client.sessions().get("sess-turns").await.unwrap();
    assert_eq!(detail.id, "sess-turns");
    assert_eq!(detail.turns.len(), 2);

    let turn0 = &detail.turns[0];
    assert_eq!(turn0.id, "turn-001");
    assert_eq!(turn0.user_message, "Hello, how are you?");
    assert_eq!(
        turn0.assistant_response.as_deref(),
        Some("I'm doing well, thanks!")
    );
    assert_eq!(turn0.tool_call_count, 0);
    assert!(turn0.completed_at.is_some());

    let turn1 = &detail.turns[1];
    assert_eq!(turn1.id, "turn-002");
    assert_eq!(turn1.tool_call_count, 2);
    assert!(turn1.assistant_response.is_none());
    assert!(turn1.completed_at.is_none());

    assert_eq!(detail.metadata.get("project").unwrap(), "arawn");
}

#[tokio::test]
async fn test_sessions_get_not_found() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("GET"))
        .and(path("/api/v1/sessions/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "code": "not_found",
            "message": "Session not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = client.sessions().get("nonexistent").await.unwrap_err();
    assert!(err.is_not_found());
    match &err {
        Error::NotFound(msg) => assert_eq!(msg, "Session not found"),
        other => panic!("Expected Error::NotFound, got {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Create session
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_sessions_create_default() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("POST"))
        .and(path("/api/v1/sessions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "sess-new",
            "turns": [],
            "created_at": "2026-03-08T12:00:00Z",
            "updated_at": "2026-03-08T12:00:00Z",
            "metadata": {}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let detail = client
        .sessions()
        .create(CreateSessionRequest::default())
        .await
        .unwrap();
    assert_eq!(detail.id, "sess-new");
    assert!(detail.turns.is_empty());
}

#[tokio::test]
async fn test_sessions_create_with_title() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("POST"))
        .and(path("/api/v1/sessions"))
        .and(body_json(json!({
            "title": "My Session"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "sess-titled",
            "turns": [],
            "created_at": "2026-03-08T12:00:00Z",
            "updated_at": "2026-03-08T12:00:00Z",
            "metadata": {}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let request = CreateSessionRequest {
        title: Some("My Session".to_string()),
        ..Default::default()
    };
    let detail = client.sessions().create(request).await.unwrap();
    assert_eq!(detail.id, "sess-titled");
}

#[tokio::test]
async fn test_sessions_create_server_error() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("POST"))
        .and(path("/api/v1/sessions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = client
        .sessions()
        .create(CreateSessionRequest::default())
        .await
        .unwrap_err();
    assert!(err.is_server_error());
}

// ─────────────────────────────────────────────────────────────────────────────
// Update session
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_sessions_update_title() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("PATCH"))
        .and(path("/api/v1/sessions/sess-upd"))
        .and(body_json(json!({
            "title": "Renamed Session"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "sess-upd",
            "turns": [],
            "created_at": "2026-03-01T10:00:00Z",
            "updated_at": "2026-03-08T14:00:00Z",
            "metadata": {}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let request = UpdateSessionRequest {
        title: Some("Renamed Session".to_string()),
        ..Default::default()
    };
    let detail = client.sessions().update("sess-upd", request).await.unwrap();
    assert_eq!(detail.id, "sess-upd");
    assert_eq!(detail.updated_at, "2026-03-08T14:00:00Z");
}

#[tokio::test]
async fn test_sessions_update_not_found() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("PATCH"))
        .and(path("/api/v1/sessions/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "code": "not_found",
            "message": "Session not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let request = UpdateSessionRequest {
        title: Some("Won't work".to_string()),
        ..Default::default()
    };
    let err = client
        .sessions()
        .update("nonexistent", request)
        .await
        .unwrap_err();
    assert!(err.is_not_found());
}

// ─────────────────────────────────────────────────────────────────────────────
// Delete session
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_sessions_delete() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("DELETE"))
        .and(path("/api/v1/sessions/sess-del"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    client.sessions().delete("sess-del").await.unwrap();
}

#[tokio::test]
async fn test_sessions_delete_not_found() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("DELETE"))
        .and(path("/api/v1/sessions/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "code": "not_found",
            "message": "Session not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = client.sessions().delete("nonexistent").await.unwrap_err();
    assert!(err.is_not_found());
}

#[tokio::test]
async fn test_sessions_delete_server_error() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("DELETE"))
        .and(path("/api/v1/sessions/sess-err"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = client.sessions().delete("sess-err").await.unwrap_err();
    assert!(err.is_server_error());
}

// ─────────────────────────────────────────────────────────────────────────────
// Session messages
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_sessions_messages() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("GET"))
        .and(path("/api/v1/sessions/sess-msg/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "session_id": "sess-msg",
            "messages": [
                {
                    "role": "user",
                    "content": "What is Rust?",
                    "timestamp": "2026-03-01T10:00:00Z",
                    "metadata": null
                },
                {
                    "role": "assistant",
                    "content": "Rust is a systems programming language.",
                    "timestamp": "2026-03-01T10:00:05Z",
                    "metadata": {"model": "claude-3"}
                }
            ],
            "count": 2
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp: SessionMessagesResponse = client.sessions().messages("sess-msg").await.unwrap();
    assert_eq!(resp.session_id, "sess-msg");
    assert_eq!(resp.count, 2);
    assert_eq!(resp.messages.len(), 2);
    assert_eq!(resp.messages[0].role, "user");
    assert_eq!(resp.messages[0].content, "What is Rust?");
    assert!(resp.messages[0].metadata.is_none());
    assert_eq!(resp.messages[1].role, "assistant");
    assert!(resp.messages[1].metadata.is_some());
}

#[tokio::test]
async fn test_sessions_messages_empty() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("GET"))
        .and(path("/api/v1/sessions/sess-empty/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "session_id": "sess-empty",
            "messages": [],
            "count": 0
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client.sessions().messages("sess-empty").await.unwrap();
    assert_eq!(resp.session_id, "sess-empty");
    assert_eq!(resp.count, 0);
    assert!(resp.messages.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// Auth / unauthorized
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_sessions_list_unauthorized() {
    let server = MockServer::start().await;
    let client = test_client(&server.uri());

    Mock::given(method("GET"))
        .and(path("/api/v1/sessions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "code": "unauthorized",
            "message": "Invalid token"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = client.sessions().list().await.unwrap_err();
    assert!(err.is_auth_error());
    match &err {
        Error::Auth(msg) => assert_eq!(msg, "Invalid token"),
        other => panic!("Expected Error::Auth, got {:?}", other),
    }
}

use arawn_client::ArawnClient;
use futures::StreamExt;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_client(uri: &str) -> ArawnClient {
    ArawnClient::builder()
        .base_url(uri)
        .auth_token("test-token")
        .build()
        .unwrap()
}

// ─────────────────────────────────────────────────────────────────────────────
// Workstreams API
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_workstreams_list() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/workstreams"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "workstreams": [
                {
                    "id": "ws-1",
                    "title": "My Workstream",
                    "summary": null,
                    "state": "active",
                    "default_model": null,
                    "is_scratch": false,
                    "created_at": "2026-03-01T00:00:00Z",
                    "updated_at": "2026-03-01T00:00:00Z",
                    "tags": []
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.workstreams().list().await.unwrap();

    assert_eq!(resp.workstreams.len(), 1);
    assert_eq!(resp.workstreams[0].id, "ws-1");
    assert_eq!(resp.workstreams[0].title, "My Workstream");
    assert_eq!(resp.workstreams[0].state, "active");
    assert!(!resp.workstreams[0].is_scratch);
}

#[tokio::test]
async fn test_workstreams_list_all_includes_archived() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/workstreams"))
        .and(query_param("include_archived", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "workstreams": [
                {
                    "id": "ws-1",
                    "title": "Active",
                    "state": "active",
                    "is_scratch": false,
                    "created_at": "2026-03-01T00:00:00Z",
                    "updated_at": "2026-03-01T00:00:00Z"
                },
                {
                    "id": "ws-2",
                    "title": "Archived",
                    "state": "archived",
                    "is_scratch": false,
                    "created_at": "2026-02-01T00:00:00Z",
                    "updated_at": "2026-02-15T00:00:00Z"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.workstreams().list_all().await.unwrap();

    assert_eq!(resp.workstreams.len(), 2);
    assert_eq!(resp.workstreams[0].state, "active");
    assert_eq!(resp.workstreams[1].state, "archived");
}

#[tokio::test]
async fn test_workstreams_get() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/workstreams/ws-42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ws-42",
            "title": "Deep Dive",
            "summary": "A deep investigation",
            "state": "active",
            "default_model": "claude-sonnet-4-20250514",
            "is_scratch": false,
            "created_at": "2026-03-01T00:00:00Z",
            "updated_at": "2026-03-02T00:00:00Z",
            "tags": ["research", "ai"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let ws = client.workstreams().get("ws-42").await.unwrap();

    assert_eq!(ws.id, "ws-42");
    assert_eq!(ws.title, "Deep Dive");
    assert_eq!(ws.summary.as_deref(), Some("A deep investigation"));
    assert_eq!(
        ws.default_model.as_deref(),
        Some("claude-sonnet-4-20250514")
    );
    assert_eq!(
        ws.tags.as_deref(),
        Some(&["research".to_string(), "ai".to_string()][..])
    );
}

#[tokio::test]
async fn test_workstreams_get_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/workstreams/ws-missing"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "code": "not_found",
            "message": "Workstream not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.workstreams().get("ws-missing").await.unwrap_err();

    assert!(err.is_not_found());
    assert!(matches!(err, arawn_client::Error::NotFound(ref msg) if msg.contains("not found")));
}

#[tokio::test]
async fn test_workstreams_create() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/workstreams"))
        .and(body_json(json!({
            "title": "New Project",
            "default_model": "claude-sonnet-4-20250514",
            "tags": ["rust"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ws-new",
            "title": "New Project",
            "summary": null,
            "state": "active",
            "default_model": "claude-sonnet-4-20250514",
            "is_scratch": false,
            "created_at": "2026-03-08T00:00:00Z",
            "updated_at": "2026-03-08T00:00:00Z",
            "tags": ["rust"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let ws = client
        .workstreams()
        .create(arawn_client::CreateWorkstreamRequest {
            title: "New Project".to_string(),
            default_model: Some("claude-sonnet-4-20250514".to_string()),
            tags: vec!["rust".to_string()],
        })
        .await
        .unwrap();

    assert_eq!(ws.id, "ws-new");
    assert_eq!(ws.title, "New Project");
    assert_eq!(ws.state, "active");
}

#[tokio::test]
async fn test_workstreams_update() {
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/api/v1/workstreams/ws-42"))
        .and(body_json(json!({
            "title": "Updated Title",
            "summary": "New summary"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ws-42",
            "title": "Updated Title",
            "summary": "New summary",
            "state": "active",
            "is_scratch": false,
            "created_at": "2026-03-01T00:00:00Z",
            "updated_at": "2026-03-08T00:00:00Z"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let ws = client
        .workstreams()
        .update(
            "ws-42",
            arawn_client::UpdateWorkstreamRequest {
                title: Some("Updated Title".to_string()),
                summary: Some("New summary".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(ws.title, "Updated Title");
    assert_eq!(ws.summary.as_deref(), Some("New summary"));
}

#[tokio::test]
async fn test_workstreams_delete() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/workstreams/ws-42"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    client.workstreams().delete("ws-42").await.unwrap();
}

#[tokio::test]
async fn test_workstreams_send_message() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/workstreams/ws-42/messages"))
        .and(body_json(json!({
            "content": "Hello workstream",
            "role": "user"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg-1",
            "workstream_id": "ws-42",
            "session_id": null,
            "role": "user",
            "content": "Hello workstream",
            "timestamp": "2026-03-08T12:00:00Z",
            "metadata": null
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let msg = client
        .workstreams()
        .send_message(
            "ws-42",
            arawn_client::SendMessageRequest {
                role: Some("user".to_string()),
                content: "Hello workstream".to_string(),
                metadata: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(msg.id, "msg-1");
    assert_eq!(msg.workstream_id, "ws-42");
    assert_eq!(msg.role, "user");
    assert_eq!(msg.content, "Hello workstream");
}

#[tokio::test]
async fn test_workstreams_messages() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/workstreams/ws-42/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "messages": [
                {
                    "id": "msg-1",
                    "workstream_id": "ws-42",
                    "session_id": "sess-1",
                    "role": "user",
                    "content": "Hello",
                    "timestamp": "2026-03-08T12:00:00Z"
                },
                {
                    "id": "msg-2",
                    "workstream_id": "ws-42",
                    "session_id": "sess-1",
                    "role": "assistant",
                    "content": "Hi there!",
                    "timestamp": "2026-03-08T12:00:01Z"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.workstreams().messages("ws-42").await.unwrap();

    assert_eq!(resp.messages.len(), 2);
    assert_eq!(resp.messages[0].role, "user");
    assert_eq!(resp.messages[1].role, "assistant");
    assert_eq!(resp.messages[1].content, "Hi there!");
}

#[tokio::test]
async fn test_workstreams_messages_since() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/workstreams/ws-42/messages"))
        .and(query_param("since", "2026-03-08T10:00:00Z"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "messages": [
                {
                    "id": "msg-3",
                    "workstream_id": "ws-42",
                    "role": "user",
                    "content": "Recent message",
                    "timestamp": "2026-03-08T12:00:00Z"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client
        .workstreams()
        .messages_since("ws-42", "2026-03-08T10:00:00Z")
        .await
        .unwrap();

    assert_eq!(resp.messages.len(), 1);
    assert_eq!(resp.messages[0].content, "Recent message");
}

#[tokio::test]
async fn test_workstreams_sessions() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/workstreams/ws-42/sessions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sessions": [
                {
                    "id": "sess-1",
                    "workstream_id": "ws-42",
                    "started_at": "2026-03-08T10:00:00Z",
                    "ended_at": null,
                    "is_active": true
                },
                {
                    "id": "sess-2",
                    "workstream_id": "ws-42",
                    "started_at": "2026-03-07T10:00:00Z",
                    "ended_at": "2026-03-07T11:00:00Z",
                    "is_active": false
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.workstreams().sessions("ws-42").await.unwrap();

    assert_eq!(resp.sessions.len(), 2);
    assert!(resp.sessions[0].is_active);
    assert!(!resp.sessions[1].is_active);
    assert_eq!(
        resp.sessions[1].ended_at.as_deref(),
        Some("2026-03-07T11:00:00Z")
    );
}

#[tokio::test]
async fn test_workstreams_promote_scratch() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/workstreams/scratch/promote"))
        .and(body_json(json!({
            "title": "Promoted Workstream",
            "tags": ["important"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ws-promoted",
            "title": "Promoted Workstream",
            "summary": null,
            "state": "active",
            "is_scratch": false,
            "created_at": "2026-03-08T00:00:00Z",
            "updated_at": "2026-03-08T00:00:00Z",
            "tags": ["important"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let ws = client
        .workstreams()
        .promote_scratch(arawn_client::PromoteRequest {
            title: "Promoted Workstream".to_string(),
            tags: vec!["important".to_string()],
            default_model: None,
        })
        .await
        .unwrap();

    assert_eq!(ws.id, "ws-promoted");
    assert_eq!(ws.title, "Promoted Workstream");
    assert!(!ws.is_scratch);
}

#[tokio::test]
async fn test_workstreams_list_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/workstreams"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.workstreams().list().await.unwrap_err();

    assert!(err.is_server_error());
    assert!(matches!(
        err,
        arawn_client::Error::Api {
            status: 500,
            ref code,
            ..
        } if code == "internal_error"
    ));
}

#[tokio::test]
async fn test_workstreams_create_auth_header() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/workstreams"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ws-auth",
            "title": "Auth Test",
            "state": "active",
            "is_scratch": false,
            "created_at": "2026-03-08T00:00:00Z",
            "updated_at": "2026-03-08T00:00:00Z"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let ws = client
        .workstreams()
        .create(arawn_client::CreateWorkstreamRequest {
            title: "Auth Test".to_string(),
            default_model: None,
            tags: vec![],
        })
        .await
        .unwrap();

    assert_eq!(ws.id, "ws-auth");
}

// ─────────────────────────────────────────────────────────────────────────────
// Chat API
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_chat_send() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "response": "Hello! How can I help?",
            "session_id": "sess-1",
            "turn_id": "turn-1",
            "tool_calls": [],
            "model": "claude-sonnet-4-20250514",
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client
        .chat()
        .send(arawn_client::ChatRequest::new("hello"))
        .await
        .unwrap();

    assert_eq!(resp.response, "Hello! How can I help?");
    assert_eq!(resp.session_id, "sess-1");
    assert_eq!(resp.turn_id, "turn-1");
    assert!(resp.tool_calls.is_empty());
    assert_eq!(resp.model.as_deref(), Some("claude-sonnet-4-20250514"));
    let usage = resp.usage.unwrap();
    assert_eq!(usage.prompt_tokens, 10);
    assert_eq!(usage.completion_tokens, 8);
    assert_eq!(usage.total_tokens, 18);
}

#[tokio::test]
async fn test_chat_message() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "response": "Hi!",
            "session_id": "sess-2",
            "turn_id": "turn-2",
            "tool_calls": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.chat().message("hey there").await.unwrap();

    assert_eq!(resp.response, "Hi!");
    assert_eq!(resp.session_id, "sess-2");
}

#[tokio::test]
async fn test_chat_message_in_session() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat"))
        .and(body_json(json!({
            "message": "continue",
            "session_id": "sess-existing"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "response": "Continuing...",
            "session_id": "sess-existing",
            "turn_id": "turn-3",
            "tool_calls": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client
        .chat()
        .message_in_session("sess-existing", "continue")
        .await
        .unwrap();

    assert_eq!(resp.session_id, "sess-existing");
    assert_eq!(resp.response, "Continuing...");
}

#[tokio::test]
async fn test_chat_send_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "code": "not_found",
            "message": "Not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.chat().message("hello").await.unwrap_err();

    assert!(err.is_not_found());
}

#[tokio::test]
async fn test_chat_send_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.chat().message("hello").await.unwrap_err();

    assert!(err.is_server_error());
}

#[tokio::test]
async fn test_chat_send_unauthorized() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "code": "unauthorized",
            "message": "Unauthorized"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.chat().message("hello").await.unwrap_err();

    assert!(err.is_auth_error());
    assert!(matches!(err, arawn_client::Error::Auth(ref msg) if msg == "Unauthorized"));
}

#[tokio::test]
async fn test_chat_send_auth_header() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "response": "Authenticated!",
            "session_id": "sess-auth",
            "turn_id": "turn-auth",
            "tool_calls": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.chat().message("hello").await.unwrap();

    assert_eq!(resp.response, "Authenticated!");
}

#[tokio::test]
async fn test_chat_send_with_model() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat"))
        .and(body_json(json!({
            "message": "hello",
            "model": "claude-opus-4-20250514"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "response": "Hello from Opus!",
            "session_id": "sess-model",
            "turn_id": "turn-model",
            "tool_calls": [],
            "model": "claude-opus-4-20250514"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client
        .chat()
        .send(arawn_client::ChatRequest::new("hello").with_model("claude-opus-4-20250514"))
        .await
        .unwrap();

    assert_eq!(resp.model.as_deref(), Some("claude-opus-4-20250514"));
}

#[tokio::test]
async fn test_chat_stream() {
    let server = MockServer::start().await;

    let sse_body = "\
data: {\"type\":\"session_start\",\"session_id\":\"s1\",\"turn_id\":\"t1\"}\n\n\
data: {\"type\":\"content\",\"text\":\"Hello\"}\n\n\
data: {\"type\":\"content\",\"text\":\" world\"}\n\n\
data: {\"type\":\"done\",\"response\":\"Hello world\",\"usage\":null}\n\n";

    Mock::given(method("POST"))
        .and(path("/api/v1/chat/stream"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/event-stream")
                .set_body_raw(sse_body, "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let stream = client
        .chat()
        .stream(arawn_client::ChatRequest::new("hello"))
        .await
        .unwrap();

    tokio::pin!(stream);

    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event.unwrap());
    }

    assert_eq!(events.len(), 4);

    // Verify session_start
    match &events[0] {
        arawn_client::StreamEvent::SessionStart {
            session_id,
            turn_id,
        } => {
            assert_eq!(session_id, "s1");
            assert_eq!(turn_id, "t1");
        }
        other => panic!("Expected SessionStart, got {:?}", other),
    }

    // Verify content chunks
    match &events[1] {
        arawn_client::StreamEvent::Content { text } => assert_eq!(text, "Hello"),
        other => panic!("Expected Content, got {:?}", other),
    }
    match &events[2] {
        arawn_client::StreamEvent::Content { text } => assert_eq!(text, " world"),
        other => panic!("Expected Content, got {:?}", other),
    }

    // Verify done
    match &events[3] {
        arawn_client::StreamEvent::Done { response, usage } => {
            assert_eq!(response, "Hello world");
            assert!(usage.is_none());
        }
        other => panic!("Expected Done, got {:?}", other),
    }
}

#[tokio::test]
async fn test_chat_stream_error_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat/stream"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let result = client
        .chat()
        .stream(arawn_client::ChatRequest::new("hello"))
        .await;

    match result {
        Err(err) => assert!(err.is_server_error()),
        Ok(_) => panic!("Expected server error, got Ok"),
    }
}

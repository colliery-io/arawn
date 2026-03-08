use arawn_client::ArawnClient;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_client(uri: &str) -> ArawnClient {
    ArawnClient::builder()
        .base_url(uri)
        .auth_token("test-token")
        .build()
        .unwrap()
}

// ─────────────────────────────────────────────────────────────────────────────
// Notes API
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_notes_list() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/notes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "notes": [
                {
                    "id": "note-1",
                    "content": "First note",
                    "tags": ["important"],
                    "created_at": "2026-03-08T10:00:00Z"
                },
                {
                    "id": "note-2",
                    "content": "Second note",
                    "tags": [],
                    "created_at": "2026-03-08T11:00:00Z"
                }
            ],
            "total": 2
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.notes().list().await.unwrap();

    assert_eq!(resp.total, 2);
    assert_eq!(resp.notes.len(), 2);
    assert_eq!(resp.notes[0].id, "note-1");
    assert_eq!(resp.notes[0].content, "First note");
    assert_eq!(resp.notes[0].tags, vec!["important"]);
    assert_eq!(resp.notes[1].id, "note-2");
    assert_eq!(resp.notes[1].content, "Second note");
    assert!(resp.notes[1].tags.is_empty());
}

#[tokio::test]
async fn test_notes_list_by_tag() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/notes"))
        .and(query_param("tag", "important"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "notes": [
                {
                    "id": "note-1",
                    "content": "Tagged note",
                    "tags": ["important"],
                    "created_at": "2026-03-08T10:00:00Z"
                }
            ],
            "total": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.notes().list_by_tag("important").await.unwrap();

    assert_eq!(resp.total, 1);
    assert_eq!(resp.notes.len(), 1);
    assert_eq!(resp.notes[0].id, "note-1");
    assert_eq!(resp.notes[0].tags, vec!["important"]);
}

#[tokio::test]
async fn test_notes_get() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/notes/note-42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "note": {
                "id": "note-42",
                "content": "My note content",
                "tags": ["rust", "testing"],
                "created_at": "2026-03-08T12:00:00Z"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let note = client.notes().get("note-42").await.unwrap();

    assert_eq!(note.id, "note-42");
    assert_eq!(note.content, "My note content");
    assert_eq!(note.tags, vec!["rust", "testing"]);
    assert_eq!(note.created_at, "2026-03-08T12:00:00Z");
}

#[tokio::test]
async fn test_notes_get_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/notes/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "code": "not_found",
            "message": "Note not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.notes().get("nonexistent").await.unwrap_err();

    assert!(matches!(err, arawn_client::Error::NotFound(_)));
    assert!(err.is_not_found());
}

#[tokio::test]
async fn test_notes_create() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/notes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "note": {
                "id": "note-new",
                "content": "New note content",
                "tags": ["created"],
                "created_at": "2026-03-08T14:00:00Z"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let note = client
        .notes()
        .create(arawn_client::CreateNoteRequest {
            content: "New note content".to_string(),
            tags: vec!["created".to_string()],
        })
        .await
        .unwrap();

    assert_eq!(note.id, "note-new");
    assert_eq!(note.content, "New note content");
    assert_eq!(note.tags, vec!["created"]);
}

#[tokio::test]
async fn test_notes_create_simple() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/notes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "note": {
                "id": "note-simple",
                "content": "Simple note",
                "tags": [],
                "created_at": "2026-03-08T15:00:00Z"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let note = client.notes().create_simple("Simple note").await.unwrap();

    assert_eq!(note.id, "note-simple");
    assert_eq!(note.content, "Simple note");
    assert!(note.tags.is_empty());
}

#[tokio::test]
async fn test_notes_update() {
    let server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/api/v1/notes/note-42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "note": {
                "id": "note-42",
                "content": "Updated content",
                "tags": ["updated"],
                "created_at": "2026-03-08T12:00:00Z"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let note = client
        .notes()
        .update(
            "note-42",
            arawn_client::UpdateNoteRequest {
                content: Some("Updated content".to_string()),
                tags: Some(vec!["updated".to_string()]),
            },
        )
        .await
        .unwrap();

    assert_eq!(note.id, "note-42");
    assert_eq!(note.content, "Updated content");
    assert_eq!(note.tags, vec!["updated"]);
}

#[tokio::test]
async fn test_notes_delete() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/notes/note-42"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let result = client.notes().delete("note-42").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_notes_delete_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/notes/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "code": "not_found",
            "message": "Note not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.notes().delete("nonexistent").await.unwrap_err();

    assert!(matches!(err, arawn_client::Error::NotFound(_)));
    assert!(err.is_not_found());
}

#[tokio::test]
async fn test_notes_list_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/notes"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.notes().list().await.unwrap_err();

    assert!(err.is_server_error());
    assert!(matches!(
        err,
        arawn_client::Error::Api {
            status: 500,
            ref code,
            ref message,
        } if code == "internal_error" && message == "Internal server error"
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// Memory API
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_memory_search() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/memory/search"))
        .and(query_param("q", "rust programming"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {
                    "id": "mem-1",
                    "content_type": "fact",
                    "content": "Rust is a systems programming language",
                    "session_id": null,
                    "score": 0.95,
                    "source": "user_input",
                    "citation": null
                }
            ],
            "query": "rust programming",
            "count": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.memory().search("rust programming").await.unwrap();

    assert_eq!(resp.query, "rust programming");
    assert_eq!(resp.count, 1);
    assert_eq!(resp.results.len(), 1);
    assert_eq!(resp.results[0].id, "mem-1");
    assert_eq!(resp.results[0].content_type, "fact");
    assert_eq!(
        resp.results[0].content,
        "Rust is a systems programming language"
    );
    assert_eq!(resp.results[0].score, 0.95);
    assert_eq!(resp.results[0].source, "user_input");
}

#[tokio::test]
async fn test_memory_search_with_options() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/memory/search"))
        .and(query_param("q", "testing"))
        .and(query_param("limit", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {
                    "id": "mem-2",
                    "content_type": "summary",
                    "content": "Testing best practices",
                    "score": 0.88,
                    "source": "session",
                    "citation": { "session": "sess-1" }
                }
            ],
            "query": "testing",
            "count": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client
        .memory()
        .search_with_options(arawn_client::MemorySearchQuery {
            q: "testing".to_string(),
            limit: Some(5),
            session_id: None,
        })
        .await
        .unwrap();

    assert_eq!(resp.query, "testing");
    assert_eq!(resp.count, 1);
    assert_eq!(resp.results[0].id, "mem-2");
    assert_eq!(resp.results[0].content_type, "summary");
}

#[tokio::test]
async fn test_memory_search_in_session() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/memory/search"))
        .and(query_param("q", "context"))
        .and(query_param("session_id", "sess-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {
                    "id": "mem-3",
                    "content_type": "fact",
                    "content": "Session-specific memory",
                    "session_id": "sess-abc",
                    "score": 0.92,
                    "source": "session"
                }
            ],
            "query": "context",
            "count": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client
        .memory()
        .search_in_session("context", "sess-abc")
        .await
        .unwrap();

    assert_eq!(resp.query, "context");
    assert_eq!(resp.count, 1);
    assert_eq!(resp.results[0].session_id.as_deref(), Some("sess-abc"));
}

#[tokio::test]
async fn test_memory_search_empty() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/memory/search"))
        .and(query_param("q", "nonexistent topic"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [],
            "query": "nonexistent topic",
            "count": 0
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.memory().search("nonexistent topic").await.unwrap();

    assert_eq!(resp.query, "nonexistent topic");
    assert_eq!(resp.count, 0);
    assert!(resp.results.is_empty());
}

#[tokio::test]
async fn test_memory_store() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/memory"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "mem-new",
            "content_type": "fact",
            "message": "Memory stored successfully"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client
        .memory()
        .store(arawn_client::StoreMemoryRequest {
            content: "Rust ownership model".to_string(),
            content_type: "fact".to_string(),
            session_id: Some("sess-1".to_string()),
            metadata: Default::default(),
            confidence: 0.9,
        })
        .await
        .unwrap();

    assert_eq!(resp.id, "mem-new");
    assert_eq!(resp.content_type, "fact");
    assert_eq!(resp.message, "Memory stored successfully");
}

#[tokio::test]
async fn test_memory_store_fact() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/memory"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "mem-fact",
            "content_type": "fact",
            "message": "Fact stored"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.memory().store_fact("The sky is blue").await.unwrap();

    assert_eq!(resp.id, "mem-fact");
    assert_eq!(resp.content_type, "fact");
    assert_eq!(resp.message, "Fact stored");
}

#[tokio::test]
async fn test_memory_delete() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/memory/mem-42"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let result = client.memory().delete("mem-42").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_memory_search_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/memory/search"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.memory().search("anything").await.unwrap_err();

    assert!(err.is_server_error());
    assert!(matches!(
        err,
        arawn_client::Error::Api {
            status: 500,
            ref code,
            ref message,
        } if code == "internal_error" && message == "Internal server error"
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// Tasks API
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_tasks_list() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tasks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tasks": [
                {
                    "id": "task-1",
                    "task_type": "indexing",
                    "status": "running",
                    "progress": 50,
                    "created_at": "2026-03-08T10:00:00Z"
                },
                {
                    "id": "task-2",
                    "task_type": "embedding",
                    "status": "completed",
                    "progress": 100,
                    "created_at": "2026-03-08T09:00:00Z"
                }
            ],
            "total": 2
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.tasks().list().await.unwrap();

    assert_eq!(resp.total, 2);
    assert_eq!(resp.tasks.len(), 2);
    assert_eq!(resp.tasks[0].id, "task-1");
    assert_eq!(resp.tasks[0].task_type, "indexing");
    assert_eq!(resp.tasks[0].status, arawn_client::TaskStatus::Running);
    assert_eq!(resp.tasks[0].progress, Some(50));
    assert_eq!(resp.tasks[1].id, "task-2");
    assert_eq!(resp.tasks[1].status, arawn_client::TaskStatus::Completed);
}

#[tokio::test]
async fn test_tasks_list_running() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tasks"))
        .and(query_param("status", "running"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tasks": [
                {
                    "id": "task-1",
                    "task_type": "indexing",
                    "status": "running",
                    "progress": 75,
                    "created_at": "2026-03-08T10:00:00Z"
                }
            ],
            "total": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.tasks().list_running().await.unwrap();

    assert_eq!(resp.total, 1);
    assert_eq!(resp.tasks.len(), 1);
    assert_eq!(resp.tasks[0].status, arawn_client::TaskStatus::Running);
}

#[tokio::test]
async fn test_tasks_list_for_session() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tasks"))
        .and(query_param("session_id", "sess-xyz"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tasks": [
                {
                    "id": "task-3",
                    "task_type": "analysis",
                    "status": "pending",
                    "created_at": "2026-03-08T11:00:00Z"
                }
            ],
            "total": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.tasks().list_for_session("sess-xyz").await.unwrap();

    assert_eq!(resp.total, 1);
    assert_eq!(resp.tasks[0].id, "task-3");
    assert_eq!(resp.tasks[0].status, arawn_client::TaskStatus::Pending);
}

#[tokio::test]
async fn test_tasks_get() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tasks/task-99"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "task-99",
            "task_type": "indexing",
            "status": "running",
            "progress": 60,
            "message": "Processing files...",
            "session_id": "sess-1",
            "created_at": "2026-03-08T10:00:00Z",
            "started_at": "2026-03-08T10:00:01Z",
            "completed_at": null,
            "error": null
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let task = client.tasks().get("task-99").await.unwrap();

    assert_eq!(task.id, "task-99");
    assert_eq!(task.task_type, "indexing");
    assert_eq!(task.status, arawn_client::TaskStatus::Running);
    assert_eq!(task.progress, Some(60));
    assert_eq!(task.message.as_deref(), Some("Processing files..."));
    assert_eq!(task.session_id.as_deref(), Some("sess-1"));
    assert_eq!(task.started_at.as_deref(), Some("2026-03-08T10:00:01Z"));
    assert!(task.completed_at.is_none());
    assert!(task.error.is_none());
}

#[tokio::test]
async fn test_tasks_get_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tasks/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "code": "not_found",
            "message": "Task not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.tasks().get("nonexistent").await.unwrap_err();

    assert!(matches!(err, arawn_client::Error::NotFound(_)));
    assert!(err.is_not_found());
}

#[tokio::test]
async fn test_tasks_cancel() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/tasks/task-99"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let result = client.tasks().cancel("task-99").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_tasks_cancel_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/tasks/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "code": "not_found",
            "message": "Task not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.tasks().cancel("nonexistent").await.unwrap_err();

    assert!(matches!(err, arawn_client::Error::NotFound(_)));
    assert!(err.is_not_found());
}

#[tokio::test]
async fn test_tasks_list_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tasks"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.tasks().list().await.unwrap_err();

    assert!(err.is_server_error());
    assert!(matches!(
        err,
        arawn_client::Error::Api {
            status: 500,
            ref code,
            ref message,
        } if code == "internal_error" && message == "Internal server error"
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// MCP API
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_list_servers() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/mcp/servers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "servers": [
                {
                    "name": "filesystem",
                    "server_type": "stdio",
                    "connected": true,
                    "tool_count": 5
                },
                {
                    "name": "web-search",
                    "server_type": "http",
                    "connected": false,
                    "tool_count": null
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.mcp().list_servers().await.unwrap();

    assert_eq!(resp.servers.len(), 2);
    assert_eq!(resp.servers[0].name, "filesystem");
    assert_eq!(resp.servers[0].server_type, "stdio");
    assert!(resp.servers[0].connected);
    assert_eq!(resp.servers[0].tool_count, Some(5));
    assert_eq!(resp.servers[1].name, "web-search");
    assert_eq!(resp.servers[1].server_type, "http");
    assert!(!resp.servers[1].connected);
    assert!(resp.servers[1].tool_count.is_none());
}

#[tokio::test]
async fn test_mcp_list_servers_empty() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/mcp/servers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "servers": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.mcp().list_servers().await.unwrap();

    assert!(resp.servers.is_empty());
}

#[tokio::test]
async fn test_mcp_add_server() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/mcp/servers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "my-server",
            "connected": true,
            "tools": ["read_file", "write_file", "list_dir"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client
        .mcp()
        .add_server(arawn_client::AddServerRequest {
            name: "my-server".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
            ],
            env: Default::default(),
            url: None,
            auto_connect: true,
        })
        .await
        .unwrap();

    assert_eq!(resp.name, "my-server");
    assert!(resp.connected);
    assert_eq!(resp.tools, vec!["read_file", "write_file", "list_dir"]);
}

#[tokio::test]
async fn test_mcp_add_stdio_server() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/mcp/servers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "fs-server",
            "connected": true,
            "tools": ["read_file"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client
        .mcp()
        .add_stdio_server(
            "fs-server",
            "npx",
            vec!["-y".to_string(), "mcp-server".to_string()],
            true,
        )
        .await
        .unwrap();

    assert_eq!(resp.name, "fs-server");
    assert!(resp.connected);
    assert_eq!(resp.tools, vec!["read_file"]);
}

#[tokio::test]
async fn test_mcp_add_http_server() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/mcp/servers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "remote-server",
            "connected": true,
            "tools": ["search"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client
        .mcp()
        .add_http_server("remote-server", "https://mcp.example.com", true)
        .await
        .unwrap();

    assert_eq!(resp.name, "remote-server");
    assert!(resp.connected);
    assert_eq!(resp.tools, vec!["search"]);
}

#[tokio::test]
async fn test_mcp_remove_server() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/mcp/servers/old-server"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let result = client.mcp().remove_server("old-server").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mcp_remove_server_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/mcp/servers/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "code": "not_found",
            "message": "Server not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.mcp().remove_server("nonexistent").await.unwrap_err();

    assert!(matches!(err, arawn_client::Error::NotFound(_)));
    assert!(err.is_not_found());
}

#[tokio::test]
async fn test_mcp_list_tools() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/mcp/servers/filesystem/tools"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "server": "filesystem",
            "tools": [
                {
                    "name": "read_file",
                    "description": "Read a file from disk"
                },
                {
                    "name": "write_file",
                    "description": "Write content to a file"
                },
                {
                    "name": "list_dir",
                    "description": null
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.mcp().list_tools("filesystem").await.unwrap();

    assert_eq!(resp.server, "filesystem");
    assert_eq!(resp.tools.len(), 3);
    assert_eq!(resp.tools[0].name, "read_file");
    assert_eq!(
        resp.tools[0].description.as_deref(),
        Some("Read a file from disk")
    );
    assert_eq!(resp.tools[1].name, "write_file");
    assert_eq!(
        resp.tools[1].description.as_deref(),
        Some("Write content to a file")
    );
    assert_eq!(resp.tools[2].name, "list_dir");
    assert!(resp.tools[2].description.is_none());
}

#[tokio::test]
async fn test_mcp_connect() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/mcp/servers/filesystem/connect"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let result = client.mcp().connect("filesystem").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mcp_disconnect() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/mcp/servers/filesystem/disconnect"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let result = client.mcp().disconnect("filesystem").await;

    assert!(result.is_ok());
}

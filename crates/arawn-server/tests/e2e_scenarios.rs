//! End-to-end BDD-style test scenarios.
//!
//! These tests exercise the full Arawn system with mocked LLM backends,
//! real tool execution, and multiple subsystems (sessions, memory, notes,
//! workstreams) working together.

mod common;

use anyhow::Result;
use serde_json::json;

use arawn_test_utils::server::TestServerBuilder;
use arawn_test_utils::{
    ScriptedMockBackend, StreamingMockBackend, TestServer, collect_sse_events, events_of_type,
    mock_tool_registry, reconstruct_text,
};

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 1: Chat with tool execution (sync endpoint)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_chat_with_tool_execution() -> Result<()> {
    // Given: a server with the echo tool registered and a scripted backend
    // that first requests tool use, then responds with text.
    let backend = ScriptedMockBackend::tool_then_text(
        "echo",
        "tc-1",
        json!({"message": "hello from test"}),
        "The echo tool responded successfully.",
    );

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    // When: we send a chat message
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Please echo hello from test"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;

    // Then: session was created
    let session_id = body["session_id"].as_str().expect("should have session_id");
    assert!(!session_id.is_empty());

    // Then: tool was called and succeeded
    let tool_calls = body["tool_calls"]
        .as_array()
        .expect("should have tool_calls");
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["name"], "echo");
    assert_eq!(tool_calls[0]["success"], true);

    // Then: final response contains the text from the second LLM call
    assert_eq!(
        body["response"].as_str().unwrap(),
        "The echo tool responded successfully."
    );

    Ok(())
}

#[tokio::test]
async fn scenario_tool_execution_with_read_file() -> Result<()> {
    // Given: server with read_file tool, backend requests file read then responds
    let backend = ScriptedMockBackend::tool_then_text(
        "read_file",
        "tc-2",
        json!({"path": "/test/hello.txt"}),
        "The file contains a greeting.",
    );

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    // When: chat
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Read the hello file"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;

    // Then: read_file tool executed successfully
    let tool_calls = body["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["name"], "read_file");
    assert_eq!(tool_calls[0]["success"], true);

    Ok(())
}

#[tokio::test]
async fn scenario_tool_failure_is_reported() -> Result<()> {
    // Given: server where backend requests the fail_tool
    let backend = ScriptedMockBackend::tool_then_text(
        "fail_tool",
        "tc-3",
        json!({"reason": "intentional failure"}),
        "The tool failed but I can continue.",
    );

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    // When: chat
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Try the fail tool"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;

    // Then: tool call reported as failed
    let tool_calls = body["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["name"], "fail_tool");
    assert_eq!(tool_calls[0]["success"], false);

    // Then: agent still returned a text response (recovered from tool failure)
    assert!(!body["response"].as_str().unwrap().is_empty());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 2: Multi-turn conversation with session persistence
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_multi_turn_conversation() -> Result<()> {
    // Given: server with multiple responses for multi-turn chat
    let server = TestServer::start_with_responses(vec![
        "First response: I understand.".to_string(),
        "Second response: Building on our conversation.".to_string(),
    ])
    .await?;

    // When: first turn creates a new session
    let resp1 = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Hello, this is the first message"}))
        .send()
        .await?;

    assert_eq!(resp1.status().as_u16(), 200);
    let body1: serde_json::Value = resp1.json().await?;
    let session_id = body1["session_id"].as_str().unwrap().to_string();

    // When: second turn uses the same session
    let resp2 = server
        .post("/api/v1/chat")
        .json(&json!({
            "session_id": session_id,
            "message": "This is the second message"
        }))
        .send()
        .await?;

    assert_eq!(resp2.status().as_u16(), 200);
    let body2: serde_json::Value = resp2.json().await?;
    assert_eq!(body2["session_id"], session_id);

    // Then: session has two turns
    let session_resp = server
        .get(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;

    assert_eq!(session_resp.status().as_u16(), 200);
    let session: serde_json::Value = session_resp.json().await?;
    let turns = session["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 2);

    // Verify turn content
    assert!(
        turns[0]["user_message"]
            .as_str()
            .unwrap()
            .contains("first message")
    );
    assert!(
        turns[1]["user_message"]
            .as_str()
            .unwrap()
            .contains("second message")
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 3: Memory store and search
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_memory_store_and_search() -> Result<()> {
    // Given: server with memory enabled
    let server = TestServer::start().await?;

    // When: store a memory fact
    let store_resp = server
        .post("/api/v1/memory")
        .json(&json!({
            "content": "The capital of France is Paris",
            "content_type": "fact",
            "confidence": 0.95
        }))
        .send()
        .await?;

    assert_eq!(store_resp.status().as_u16(), 201);
    let stored: serde_json::Value = store_resp.json().await?;
    assert_eq!(stored["content_type"], "fact");
    let memory_id = stored["id"].as_str().unwrap().to_string();

    // When: search for the memory (use single substring — LIKE '%Paris%')
    let search_resp = server.get("/api/v1/memory/search?q=Paris").send().await?;

    assert_eq!(search_resp.status().as_u16(), 200);
    let search_result: serde_json::Value = search_resp.json().await?;
    assert!(search_result["count"].as_u64().unwrap() >= 1);

    let results = search_result["results"].as_array().unwrap();
    let found = results
        .iter()
        .any(|r| r["content"].as_str().unwrap().contains("Paris"));
    assert!(found, "Search should find the stored memory");

    // When: delete the memory
    let delete_resp = server
        .delete(&format!("/api/v1/memory/{}", memory_id))
        .send()
        .await?;

    assert_eq!(delete_resp.status().as_u16(), 204);

    Ok(())
}

#[tokio::test]
async fn scenario_memory_search_includes_notes() -> Result<()> {
    // Given: server with both a stored memory and a note
    let server = TestServer::start().await?;

    // Store a memory
    server
        .post("/api/v1/memory")
        .json(&json!({
            "content": "Rust was created by Mozilla",
            "content_type": "fact"
        }))
        .send()
        .await?;

    // Create a note about the same topic
    server
        .post("/api/v1/notes")
        .json(&json!({
            "content": "Rust is a systems programming language by Mozilla",
            "tags": ["rust", "languages"]
        }))
        .send()
        .await?;

    // When: search for "Mozilla" (single substring for LIKE matching)
    let search_resp = server.get("/api/v1/memory/search?q=Mozilla").send().await?;

    assert_eq!(search_resp.status().as_u16(), 200);
    let result: serde_json::Value = search_resp.json().await?;

    // Then: both memory and note appear in results
    let results = result["results"].as_array().unwrap();
    assert!(results.len() >= 2, "Should find both memory and note");

    let sources: Vec<&str> = results
        .iter()
        .map(|r| r["source"].as_str().unwrap())
        .collect();
    assert!(sources.contains(&"memory_store"));
    assert!(sources.contains(&"notes"));

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 4: Notes lifecycle (CRUD)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_notes_full_lifecycle() -> Result<()> {
    let server = TestServer::start().await?;

    // Create
    let create_resp = server
        .post("/api/v1/notes")
        .json(&json!({
            "content": "Initial note content",
            "title": "Test Note",
            "tags": ["test", "e2e"]
        }))
        .send()
        .await?;

    assert_eq!(create_resp.status().as_u16(), 201);
    let note: serde_json::Value = create_resp.json().await?;
    let note_id = note["id"].as_str().unwrap().to_string();
    assert_eq!(note["content"], "Initial note content");
    assert_eq!(note["title"], "Test Note");
    assert_eq!(note["tags"], json!(["test", "e2e"]));

    // Read
    let get_resp = server
        .get(&format!("/api/v1/notes/{}", note_id))
        .send()
        .await?;

    assert_eq!(get_resp.status().as_u16(), 200);
    let fetched: serde_json::Value = get_resp.json().await?;
    assert_eq!(fetched["id"], note_id);
    assert_eq!(fetched["content"], "Initial note content");

    // Update
    let update_resp = server
        .put(&format!("/api/v1/notes/{}", note_id))
        .json(&json!({
            "content": "Updated note content",
            "tags": ["test", "e2e", "updated"]
        }))
        .send()
        .await?;

    assert_eq!(update_resp.status().as_u16(), 200);
    let updated: serde_json::Value = update_resp.json().await?;
    assert_eq!(updated["content"], "Updated note content");
    assert_eq!(updated["tags"], json!(["test", "e2e", "updated"]));

    // List with tag filter
    let list_resp = server.get("/api/v1/notes?tag=updated").send().await?;

    assert_eq!(list_resp.status().as_u16(), 200);
    let list: serde_json::Value = list_resp.json().await?;
    assert_eq!(list["total"], 1);
    assert_eq!(list["notes"][0]["id"], note_id);

    // Delete
    let delete_resp = server
        .delete(&format!("/api/v1/notes/{}", note_id))
        .send()
        .await?;

    assert_eq!(delete_resp.status().as_u16(), 204);

    // Verify deleted
    let get_deleted = server
        .get(&format!("/api/v1/notes/{}", note_id))
        .send()
        .await?;

    assert_eq!(get_deleted.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 5: Session management
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_session_create_list_delete() -> Result<()> {
    let server = TestServer::start().await?;

    // Create a session via chat
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Hello"}))
        .send()
        .await?;

    assert_eq!(chat_resp.status().as_u16(), 200);
    let body: serde_json::Value = chat_resp.json().await?;
    let session_id = body["session_id"].as_str().unwrap().to_string();

    // List sessions
    let list_resp = server.get("/api/v1/sessions").send().await?;
    assert_eq!(list_resp.status().as_u16(), 200);
    let list: serde_json::Value = list_resp.json().await?;
    let sessions = list["sessions"].as_array().unwrap();
    assert!(sessions.iter().any(|s| s["id"] == session_id));

    // Get session detail
    let detail_resp = server
        .get(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;

    assert_eq!(detail_resp.status().as_u16(), 200);
    let detail: serde_json::Value = detail_resp.json().await?;
    assert_eq!(detail["id"], session_id);
    assert_eq!(detail["turns"].as_array().unwrap().len(), 1);

    // Get session messages
    let messages_resp = server
        .get(&format!("/api/v1/sessions/{}/messages", session_id))
        .send()
        .await?;

    assert_eq!(messages_resp.status().as_u16(), 200);
    let messages: serde_json::Value = messages_resp.json().await?;
    assert!(!messages["messages"].as_array().unwrap().is_empty());

    // Delete session
    let delete_resp = server
        .delete(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;

    assert_eq!(delete_resp.status().as_u16(), 204);

    // Verify deleted
    let get_deleted = server
        .get(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;

    assert_eq!(get_deleted.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 6: Workstream operations
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_workstream_create_and_list() -> Result<()> {
    // Given: server with workstream support
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    // When: create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({
            "title": "project-alpha",
            "tags": ["frontend", "v2"]
        }))
        .send()
        .await?;

    assert_eq!(create_resp.status().as_u16(), 201);
    let ws: serde_json::Value = create_resp.json().await?;
    let ws_id = ws["id"].as_str().unwrap().to_string();
    assert_eq!(ws["title"], "project-alpha");

    // When: create another workstream
    let create_resp2 = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "project-beta"}))
        .send()
        .await?;

    assert_eq!(create_resp2.status().as_u16(), 201);

    // Then: both appear in listing
    let list_resp = server.get("/api/v1/workstreams").send().await?;
    assert_eq!(list_resp.status().as_u16(), 200);
    let list: serde_json::Value = list_resp.json().await?;
    let workstreams = list["workstreams"].as_array().unwrap();

    assert!(
        workstreams.len() >= 2,
        "Should have at least 2 workstreams, got {}",
        workstreams.len()
    );

    let titles: Vec<&str> = workstreams
        .iter()
        .filter_map(|w| w["title"].as_str())
        .collect();
    assert!(titles.contains(&"project-alpha"));
    assert!(titles.contains(&"project-beta"));

    // Get single workstream
    let get_resp = server
        .get(&format!("/api/v1/workstreams/{}", ws_id))
        .send()
        .await?;

    assert_eq!(get_resp.status().as_u16(), 200);
    let detail: serde_json::Value = get_resp.json().await?;
    assert_eq!(detail["title"], "project-alpha");

    Ok(())
}

#[tokio::test]
async fn scenario_workstream_messaging() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    // Create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "msg-test"}))
        .send()
        .await?;

    let ws: serde_json::Value = create_resp.json().await?;
    let ws_id = ws["id"].as_str().unwrap().to_string();

    // Send messages
    let msg_resp = server
        .post(&format!("/api/v1/workstreams/{}/messages", ws_id))
        .json(&json!({
            "content": "Hello from the workstream!",
            "role": "user"
        }))
        .send()
        .await?;

    assert_eq!(msg_resp.status().as_u16(), 201);

    // List messages
    let list_resp = server
        .get(&format!("/api/v1/workstreams/{}/messages", ws_id))
        .send()
        .await?;

    assert_eq!(list_resp.status().as_u16(), 200);
    let messages: serde_json::Value = list_resp.json().await?;
    let msgs = messages["messages"].as_array().unwrap();
    assert!(!msgs.is_empty());
    assert!(msgs.iter().any(|m| {
        m["content"]
            .as_str()
            .unwrap()
            .contains("Hello from the workstream")
    }));

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 7: SSE streaming with tool execution
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_streaming_chat_with_tool() -> Result<()> {
    // Given: server with echo tool and scripted backend
    let backend = ScriptedMockBackend::tool_then_text(
        "echo",
        "tc-stream-1",
        json!({"message": "streamed hello"}),
        "Stream response after tool.",
    );

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    // When: stream a chat message
    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "Echo something via stream"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    assert!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()?
            .starts_with("text/event-stream")
    );

    let events = collect_sse_events(resp).await;

    // Then: should have session event first
    assert!(events[0].is("session"));
    assert!(events[0].get_str("session_id").is_some());

    // Then: should have tool events
    let tool_starts = events_of_type(&events, "tool_start");
    assert!(
        !tool_starts.is_empty(),
        "Should have tool_start events, got events: {:?}",
        events.iter().map(|e| &e.event).collect::<Vec<_>>()
    );
    assert_eq!(tool_starts[0].get_str("name"), Some("echo"));

    let tool_outputs = events_of_type(&events, "tool_output");
    assert!(!tool_outputs.is_empty(), "Should have tool_output events");

    let tool_ends = events_of_type(&events, "tool_end");
    assert!(!tool_ends.is_empty(), "Should have tool_end events");
    assert_eq!(tool_ends[0].get_bool("success"), Some(true));

    // Then: should have text content after tool
    let text = reconstruct_text(&events);
    assert!(
        !text.is_empty(),
        "Should have text content after tool execution"
    );

    // Then: should end with done
    let last = events.last().unwrap();
    assert!(
        last.is("done") || last.is("error"),
        "Last event should be done or error, got: {}",
        last.event
    );

    Ok(())
}

#[tokio::test]
async fn scenario_streaming_text_only() -> Result<()> {
    // Given: simple text response, no tools
    let backend = StreamingMockBackend::from_text("The quick brown fox jumps over the lazy dog");

    let server = TestServerBuilder::new()
        .with_streaming_backend(backend)
        .build()
        .await?;

    // When: stream
    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "Tell me a story"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let events = collect_sse_events(resp).await;

    // Then: no tool events, just text
    assert!(events_of_type(&events, "tool_start").is_empty());

    let text = reconstruct_text(&events);
    assert!(text.contains("quick"));
    assert!(text.contains("brown"));
    assert!(text.contains("fox"));

    // Done event present
    assert!(events.iter().any(|e| e.is("done")));

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 8: Error paths
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_oversized_message_rejected() -> Result<()> {
    let server = TestServer::start().await?;

    // 100KB + 1 byte exceeds the limit
    let large_message = "x".repeat(102_401);

    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": large_message}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 400);

    Ok(())
}

#[tokio::test]
async fn scenario_nonexistent_session_returns_404() -> Result<()> {
    let server = TestServer::start().await?;

    let fake_id = "00000000-0000-0000-0000-000000000000";
    let resp = server
        .get(&format!("/api/v1/sessions/{}", fake_id))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

#[tokio::test]
async fn scenario_note_not_found() -> Result<()> {
    let server = TestServer::start().await?;

    let fake_id = "00000000-0000-0000-0000-000000000000";
    let resp = server
        .get(&format!("/api/v1/notes/{}", fake_id))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

#[tokio::test]
async fn scenario_unauthenticated_requests_rejected() -> Result<()> {
    let server = TestServer::start().await?;

    // Use raw client without auth header
    let resp = server
        .client
        .get(format!("{}/api/v1/sessions", server.base_url()))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 401);

    let resp = server
        .client
        .post(format!("{}/api/v1/chat", server.base_url()))
        .json(&json!({"message": "hi"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 401);

    Ok(())
}

#[tokio::test]
async fn scenario_memory_without_store_returns_503() -> Result<()> {
    // Given: server without memory store
    let server = TestServerBuilder::new().without_memory().build().await?;

    // Then: memory endpoints return 503
    let resp = server.get("/api/v1/memory/search?q=test").send().await?;

    assert_eq!(resp.status().as_u16(), 503);

    let resp = server
        .post("/api/v1/notes")
        .json(&json!({"content": "test"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 503);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 9: Cross-subsystem integration
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_chat_then_store_memory_then_search() -> Result<()> {
    // Given: server with memory
    let server = TestServer::start_with_responses(vec![
        "The answer is 42.".to_string(),
        "I recall the answer.".to_string(),
    ])
    .await?;

    // When: chat to create a session
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "What is the answer?"}))
        .send()
        .await?;

    assert_eq!(chat_resp.status().as_u16(), 200);
    let body: serde_json::Value = chat_resp.json().await?;
    let session_id = body["session_id"].as_str().unwrap();

    // When: store a memory associated with this session
    let store_resp = server
        .post("/api/v1/memory")
        .json(&json!({
            "content": "The answer to life, the universe, and everything is 42",
            "content_type": "fact",
            "session_id": session_id
        }))
        .send()
        .await?;

    assert_eq!(store_resp.status().as_u16(), 201);

    // When: search memory scoped to the session (single substring for LIKE matching)
    let search_resp = server
        .get(&format!(
            "/api/v1/memory/search?q=42&session_id={}",
            session_id
        ))
        .send()
        .await?;

    assert_eq!(search_resp.status().as_u16(), 200);
    let result: serde_json::Value = search_resp.json().await?;
    let results = result["results"].as_array().unwrap();

    // Then: the scoped memory is found
    assert!(
        results
            .iter()
            .any(|r| r["content"].as_str().unwrap().contains("42")),
        "Should find the stored memory"
    );

    Ok(())
}

#[tokio::test]
async fn scenario_multiple_sessions_independent() -> Result<()> {
    // Given: server with responses for two sessions
    let server = TestServer::start_with_responses(vec![
        "Response for session A".to_string(),
        "Response for session B".to_string(),
    ])
    .await?;

    // When: create two independent sessions
    let resp_a = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Session A message"}))
        .send()
        .await?;

    let body_a: serde_json::Value = resp_a.json().await?;
    let session_a = body_a["session_id"].as_str().unwrap().to_string();

    let resp_b = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Session B message"}))
        .send()
        .await?;

    let body_b: serde_json::Value = resp_b.json().await?;
    let session_b = body_b["session_id"].as_str().unwrap().to_string();

    // Then: sessions are different
    assert_ne!(session_a, session_b);

    // Then: each session has its own content
    let detail_a: serde_json::Value = server
        .get(&format!("/api/v1/sessions/{}", session_a))
        .send()
        .await?
        .json()
        .await?;

    let detail_b: serde_json::Value = server
        .get(&format!("/api/v1/sessions/{}", session_b))
        .send()
        .await?
        .json()
        .await?;

    assert_eq!(detail_a["turns"].as_array().unwrap().len(), 1);
    assert_eq!(detail_b["turns"].as_array().unwrap().len(), 1);

    assert!(
        detail_a["turns"][0]["user_message"]
            .as_str()
            .unwrap()
            .contains("Session A")
    );
    assert!(
        detail_b["turns"][0]["user_message"]
            .as_str()
            .unwrap()
            .contains("Session B")
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 10: Config endpoint
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_config_reflects_server_state() -> Result<()> {
    // Given: server with specific configuration
    let server = TestServerBuilder::new()
        .with_workstreams()
        .with_rate_limiting(true)
        .with_api_rpm(60)
        .build()
        .await?;

    // When: get config
    let resp = server.get("/api/v1/config").send().await?;

    assert_eq!(resp.status().as_u16(), 200);
    let config: serde_json::Value = resp.json().await?;

    // Then: config reflects server features
    // memory_enabled checks memory_store (available by default in test servers)
    assert_eq!(config["features"]["memory_enabled"].as_bool(), Some(true));
    // embeddings_enabled checks indexer (no embedding model in test servers)
    assert_eq!(
        config["features"]["embeddings_enabled"].as_bool(),
        Some(false)
    );
    assert_eq!(
        config["features"]["workstreams_enabled"].as_bool(),
        Some(true)
    );

    Ok(())
}

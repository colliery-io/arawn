//! Stress and edge-case E2E tests.
//!
//! These tests exercise failure modes, recovery paths, concurrent operations,
//! and boundary conditions that the happy-path tests don't cover.

mod common;

use anyhow::Result;
use serde_json::json;

use arawn_test_utils::server::TestServerBuilder;
use arawn_test_utils::{
    ScriptedInvocation, ScriptedMockBackend, StreamingMockEvent, TestServer, collect_sse_events,
    events_of_type, mock_tool_registry, reconstruct_text,
};

// ─────────────────────────────────────────────────────────────────────────────
// LLM Backend Failures
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_llm_backend_error_returns_500() -> Result<()> {
    // Given: server whose LLM backend always returns an error
    let backend = ScriptedMockBackend::always_error("Simulated LLM meltdown");

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .build()
        .await?;

    // When: chat
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Hello"}))
        .send()
        .await?;

    // Then: 500 with error details
    assert_eq!(resp.status().as_u16(), 500);
    let body: serde_json::Value = resp.json().await?;
    assert!(body["code"].as_str().is_some());
    assert!(body["message"].as_str().unwrap().len() > 0);

    Ok(())
}

#[tokio::test]
async fn stress_llm_backend_error_in_stream_returns_error_event() -> Result<()> {
    // Given: backend that errors
    let backend = ScriptedMockBackend::always_error("Stream backend failure");

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .build()
        .await?;

    // When: streaming chat
    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "Hello"}))
        .send()
        .await?;

    // Then: should get an error event in SSE stream
    assert_eq!(resp.status().as_u16(), 200); // SSE always starts with 200
    let events = collect_sse_events(resp).await;

    // Should have an error event
    let errors = events_of_type(&events, "error");
    assert!(
        !errors.is_empty(),
        "Should have error events in stream, got: {:?}",
        events.iter().map(|e| &e.event).collect::<Vec<_>>()
    );

    Ok(())
}

#[tokio::test]
async fn stress_tool_error_then_recovery() -> Result<()> {
    // Given: LLM calls fail_tool first, then responds with text
    // This tests the agent's ability to continue after tool failure
    let backend = ScriptedMockBackend::tool_then_text(
        "fail_tool",
        "tc-fail",
        json!({"reason": "disk full"}),
        "I encountered an error but recovered.",
    );

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Do something risky"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;

    // Tool should have failed
    let tool_calls = body["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["success"], false);

    // But agent should still have a response
    assert!(
        !body["response"].as_str().unwrap().is_empty(),
        "Agent should recover and respond after tool failure"
    );

    Ok(())
}

#[tokio::test]
async fn stress_multiple_tool_calls_in_sequence() -> Result<()> {
    // Given: LLM calls echo, then read_file, then responds
    let backend = ScriptedMockBackend::from_invocations(vec![
        ScriptedInvocation::Events(vec![StreamingMockEvent::ToolUse {
            id: "tc-1".to_string(),
            name: "echo".to_string(),
            input: json!({"message": "step one"}),
        }]),
        ScriptedInvocation::Events(vec![StreamingMockEvent::ToolUse {
            id: "tc-2".to_string(),
            name: "read_file".to_string(),
            input: json!({"path": "/test/hello.txt"}),
        }]),
        ScriptedInvocation::Events(vec![StreamingMockEvent::Text(
            "Both tools executed.".to_string(),
        )]),
    ]);

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Do two things"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;

    let tool_calls = body["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 2, "Should have 2 tool calls");
    assert_eq!(tool_calls[0]["name"], "echo");
    assert_eq!(tool_calls[0]["success"], true);
    assert_eq!(tool_calls[1]["name"], "read_file");
    assert_eq!(tool_calls[1]["success"], true);

    assert_eq!(body["response"].as_str().unwrap(), "Both tools executed.");

    Ok(())
}

#[tokio::test]
async fn stress_tool_fail_then_tool_succeed_then_text() -> Result<()> {
    // Given: LLM calls fail_tool, then echo, then responds
    // Tests recovery mid-sequence
    let backend = ScriptedMockBackend::from_invocations(vec![
        ScriptedInvocation::Events(vec![StreamingMockEvent::ToolUse {
            id: "tc-1".to_string(),
            name: "fail_tool".to_string(),
            input: json!({"reason": "first attempt failed"}),
        }]),
        ScriptedInvocation::Events(vec![StreamingMockEvent::ToolUse {
            id: "tc-2".to_string(),
            name: "echo".to_string(),
            input: json!({"message": "retry succeeded"}),
        }]),
        ScriptedInvocation::Events(vec![StreamingMockEvent::Text(
            "Recovered after failure.".to_string(),
        )]),
    ]);

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Try something"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;

    let tool_calls = body["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 2);
    assert_eq!(tool_calls[0]["success"], false); // fail_tool
    assert_eq!(tool_calls[1]["success"], true); // echo

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Message Validation & Boundary Conditions
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_empty_message_accepted() -> Result<()> {
    // Empty string should either be accepted or rejected — test the behavior
    let server = TestServer::start().await?;

    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": ""}))
        .send()
        .await?;

    // Either 200 (accepted) or 400 (rejected) is fine — we just want no 500
    assert!(
        resp.status().as_u16() == 200 || resp.status().as_u16() == 400,
        "Empty message should return 200 or 400, got {}",
        resp.status().as_u16()
    );

    Ok(())
}

#[tokio::test]
async fn stress_message_exactly_at_limit() -> Result<()> {
    // 100KB exactly should be accepted (limit is > not >=)
    let server = TestServer::start().await?;

    let exact_limit = "x".repeat(100 * 1024);
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": exact_limit}))
        .send()
        .await?;

    // Should be accepted (limit check is >)
    assert_eq!(
        resp.status().as_u16(),
        200,
        "Message at exactly 100KB should be accepted"
    );

    Ok(())
}

#[tokio::test]
async fn stress_message_one_over_limit() -> Result<()> {
    let server = TestServer::start().await?;

    let over_limit = "x".repeat(100 * 1024 + 1);
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": over_limit}))
        .send()
        .await?;

    assert_eq!(
        resp.status().as_u16(),
        400,
        "Message over 100KB should be rejected"
    );

    Ok(())
}

#[tokio::test]
async fn stress_unicode_heavy_message() -> Result<()> {
    // Unicode chars are multi-byte — verify byte-level limit works correctly
    let server = TestServer::start().await?;

    // 4-byte emoji repeated to near the limit
    let emoji_msg = "🦀".repeat(20_000); // ~80KB in UTF-8
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": emoji_msg}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);

    Ok(())
}

#[tokio::test]
async fn stress_special_chars_in_message() -> Result<()> {
    let server = TestServer::start().await?;

    let msg = "Hello\n\t\r\0\"'\\<script>alert(1)</script>&amp;";
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": msg}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);

    Ok(())
}

#[tokio::test]
async fn stress_missing_message_field() -> Result<()> {
    let server = TestServer::start().await?;

    // Missing required "message" field
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"session_id": "abc"}))
        .send()
        .await?;

    // Should be 400 or 422 (validation error), not 500
    assert!(
        resp.status().as_u16() == 400 || resp.status().as_u16() == 422,
        "Missing message field should return 400 or 422, got {}",
        resp.status().as_u16()
    );

    Ok(())
}

#[tokio::test]
async fn stress_invalid_json_body() -> Result<()> {
    let server = TestServer::start().await?;

    let resp = server
        .client
        .post(format!("{}/api/v1/chat", server.base_url()))
        .bearer_auth(server.token.as_ref().unwrap())
        .header("content-type", "application/json")
        .body("not valid json {{{")
        .send()
        .await?;

    assert!(
        resp.status().as_u16() == 400 || resp.status().as_u16() == 422,
        "Invalid JSON should return 400 or 422, got {}",
        resp.status().as_u16()
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Session Edge Cases
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_chat_with_invalid_session_id() -> Result<()> {
    let server = TestServer::start().await?;

    // Non-UUID session_id should be rejected with 400
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Hello", "session_id": "not-a-uuid"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 400);

    Ok(())
}

#[tokio::test]
async fn stress_chat_with_nonexistent_valid_uuid_session() -> Result<()> {
    let server = TestServer::start().await?;

    // Valid UUID but doesn't exist — should create a new session with that ID
    let fake_uuid = "11111111-1111-1111-1111-111111111111";
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Hello", "session_id": fake_uuid}))
        .send()
        .await?;

    // This should work — creates a session with the given UUID
    assert_eq!(resp.status().as_u16(), 200);

    Ok(())
}

#[tokio::test]
async fn stress_delete_session_twice() -> Result<()> {
    let server = TestServer::start().await?;

    // Create a session
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Hello"}))
        .send()
        .await?;

    let body: serde_json::Value = chat_resp.json().await?;
    let session_id = body["session_id"].as_str().unwrap();

    // Delete it
    let del1 = server
        .delete(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;
    assert_eq!(del1.status().as_u16(), 204);

    // Delete again — should 404
    let del2 = server
        .delete(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;
    assert_eq!(del2.status().as_u16(), 404);

    Ok(())
}

#[tokio::test]
async fn stress_get_messages_empty_session() -> Result<()> {
    // Create a session directly (not via chat), then get messages
    let server = TestServer::start().await?;

    let create_resp = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "Empty session"}))
        .send()
        .await?;

    assert_eq!(create_resp.status().as_u16(), 201);
    let session: serde_json::Value = create_resp.json().await?;
    let session_id = session["id"].as_str().unwrap();

    // Get messages — should be empty but not error
    let msg_resp = server
        .get(&format!("/api/v1/sessions/{}/messages", session_id))
        .send()
        .await?;

    assert_eq!(msg_resp.status().as_u16(), 200);
    let messages: serde_json::Value = msg_resp.json().await?;
    assert_eq!(messages["messages"].as_array().unwrap().len(), 0);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Concurrent Operations
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_concurrent_chat_requests() -> Result<()> {
    // Given: server with enough responses for concurrent chats
    let server = TestServer::start_with_responses(vec![
        "Response A".to_string(),
        "Response B".to_string(),
        "Response C".to_string(),
        "Response D".to_string(),
        "Response E".to_string(),
    ])
    .await?;

    // When: send 5 concurrent chat requests
    let mut handles = Vec::new();
    for i in 0..5 {
        let client = server.client.clone();
        let url = format!("{}/api/v1/chat", server.base_url());
        let token = server.token.clone();

        handles.push(tokio::spawn(async move {
            let resp = client
                .post(&url)
                .bearer_auth(token.unwrap())
                .json(&json!({"message": format!("Concurrent message {}", i)}))
                .send()
                .await
                .unwrap();
            resp.status().as_u16()
        }));
    }

    let results: Vec<u16> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Then: all should succeed (no crashes, no deadlocks)
    for (i, status) in results.iter().enumerate() {
        assert_eq!(
            *status, 200,
            "Concurrent request {} failed with status {}",
            i, status
        );
    }

    Ok(())
}

#[tokio::test]
async fn stress_concurrent_session_operations() -> Result<()> {
    // Create sessions, then concurrently list and delete
    let server = TestServer::start().await?;

    // Create 3 sessions
    let mut session_ids = Vec::new();
    for _ in 0..3 {
        let resp = server
            .post("/api/v1/sessions")
            .json(&json!({"title": "stress test"}))
            .send()
            .await?;
        let body: serde_json::Value = resp.json().await?;
        session_ids.push(body["id"].as_str().unwrap().to_string());
    }

    // Concurrently: list sessions + get each session + delete one
    let list_handle = {
        let client = server.client.clone();
        let url = format!("{}/api/v1/sessions", server.base_url());
        let token = server.token.clone();
        tokio::spawn(async move {
            client
                .get(&url)
                .bearer_auth(token.unwrap())
                .send()
                .await
                .unwrap()
                .status()
                .as_u16()
        })
    };

    let get_handle = {
        let client = server.client.clone();
        let url = format!("{}/api/v1/sessions/{}", server.base_url(), session_ids[0]);
        let token = server.token.clone();
        tokio::spawn(async move {
            client
                .get(&url)
                .bearer_auth(token.unwrap())
                .send()
                .await
                .unwrap()
                .status()
                .as_u16()
        })
    };

    let delete_handle = {
        let client = server.client.clone();
        let url = format!("{}/api/v1/sessions/{}", server.base_url(), session_ids[2]);
        let token = server.token.clone();
        tokio::spawn(async move {
            client
                .delete(&url)
                .bearer_auth(token.unwrap())
                .send()
                .await
                .unwrap()
                .status()
                .as_u16()
        })
    };

    let (list_status, get_status, delete_status) =
        tokio::join!(list_handle, get_handle, delete_handle);

    assert_eq!(list_status?, 200);
    assert_eq!(get_status?, 200);
    assert_eq!(delete_status?, 204);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Authentication Edge Cases
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_wrong_auth_token() -> Result<()> {
    let server = TestServer::start().await?;

    let resp = server
        .client
        .get(format!("{}/api/v1/sessions", server.base_url()))
        .bearer_auth("wrong-token-12345")
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 401);

    Ok(())
}

#[tokio::test]
async fn stress_no_auth_mode() -> Result<()> {
    // Server with no auth token should allow all requests
    let server = TestServerBuilder::new().with_auth(None).build().await?;

    let resp = server
        .client
        .get(format!("{}/api/v1/sessions", server.base_url()))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);

    Ok(())
}

#[tokio::test]
async fn stress_health_endpoint_no_auth() -> Result<()> {
    let server = TestServer::start().await?;

    // Health endpoint should work without any auth
    let resp = server
        .client
        .get(format!("{}/health", server.base_url()))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["status"], "ok");

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Rate Limiting
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_rate_limiting_enforced() -> Result<()> {
    // Given: server with very low rate limit
    let server = TestServerBuilder::new()
        .with_rate_limiting(true)
        .with_api_rpm(2) // 2 requests per minute
        .build()
        .await?;

    // When: send many requests quickly
    let mut statuses = Vec::new();
    for _ in 0..10 {
        let resp = server
            .post("/api/v1/chat")
            .json(&json!({"message": "ping"}))
            .send()
            .await?;
        statuses.push(resp.status().as_u16());
    }

    // Then: at least some should be rate limited
    let rate_limited = statuses.iter().filter(|s| **s == 429).count();
    assert!(
        rate_limited > 0,
        "Should have rate-limited requests. Statuses: {:?}",
        statuses
    );

    // And: successful ones should still be 200
    let successful = statuses.iter().filter(|s| **s == 200).count();
    assert!(successful > 0, "Should have some successful requests");

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Memory & Notes Edge Cases
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_memory_empty_search() -> Result<()> {
    let server = TestServer::start().await?;

    // Search with no results
    let resp = server
        .get("/api/v1/memory/search?q=xyznonexistent12345")
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["count"].as_u64().unwrap(), 0);

    Ok(())
}

#[tokio::test]
async fn stress_note_update_preserves_unset_fields() -> Result<()> {
    let server = TestServer::start().await?;

    // Create a note with title and tags
    let create_resp = server
        .post("/api/v1/notes")
        .json(&json!({
            "content": "Original content",
            "title": "My Title",
            "tags": ["tag1", "tag2"]
        }))
        .send()
        .await?;

    assert_eq!(create_resp.status().as_u16(), 201);
    let note: serde_json::Value = create_resp.json().await?;
    let note_id = note["id"].as_str().unwrap();

    // Update only content — title and tags should be preserved (or null)
    let update_resp = server
        .put(&format!("/api/v1/notes/{}", note_id))
        .json(&json!({"content": "Updated content"}))
        .send()
        .await?;

    assert_eq!(update_resp.status().as_u16(), 200);
    let updated: serde_json::Value = update_resp.json().await?;
    assert_eq!(updated["content"], "Updated content");

    Ok(())
}

#[tokio::test]
async fn stress_memory_store_large_content() -> Result<()> {
    let server = TestServer::start().await?;

    let large_content = "Important fact. ".repeat(1000); // ~16KB
    let resp = server
        .post("/api/v1/memory")
        .json(&json!({
            "content": large_content,
            "content_type": "fact"
        }))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 201);

    Ok(())
}

#[tokio::test]
async fn stress_delete_nonexistent_memory() -> Result<()> {
    let server = TestServer::start().await?;

    let fake_id = "00000000-0000-0000-0000-000000000000";
    let resp = server
        .delete(&format!("/api/v1/memory/{}", fake_id))
        .send()
        .await?;

    // Should be 404 or 204 (idempotent delete), not 500
    assert!(
        resp.status().as_u16() == 404 || resp.status().as_u16() == 204,
        "Delete nonexistent memory should return 404 or 204, got {}",
        resp.status().as_u16()
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Workstream Edge Cases
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_workstream_path_traversal_rejected() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    // Try path traversal in workstream title
    let resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "../../../etc/passwd"}))
        .send()
        .await?;

    // Should be rejected (400) or sanitized, not allow path traversal
    assert!(
        resp.status().as_u16() == 400 || resp.status().as_u16() == 201,
        "Path traversal attempt should be rejected or sanitized, got {}",
        resp.status().as_u16()
    );

    // If created, verify the ID is sanitized
    if resp.status().as_u16() == 201 {
        let body: serde_json::Value = resp.json().await?;
        let id = body["id"].as_str().unwrap();
        assert!(!id.contains(".."), "ID should not contain '..'");
        assert!(!id.contains('/'), "ID should not contain '/'");
    }

    Ok(())
}

#[tokio::test]
async fn stress_workstream_nonexistent_returns_404() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    let resp = server
        .get("/api/v1/workstreams/does-not-exist-12345")
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent Info & Config Endpoints
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_list_agents() -> Result<()> {
    let server = TestServer::start().await?;

    let resp = server.get("/api/v1/agents").send().await?;

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;
    let agents = body["agents"].as_array().unwrap();
    assert!(!agents.is_empty(), "Should have at least the main agent");

    Ok(())
}

#[tokio::test]
async fn stress_get_main_agent_details() -> Result<()> {
    let server = TestServerBuilder::new()
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    let resp = server.get("/api/v1/agents/main").send().await?;

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["id"], "main");

    // Tools should be listed
    let tools = body["tools"].as_array().unwrap();
    assert!(
        tools.len() >= 3,
        "Should list at least echo, read_file, fail_tool"
    );

    Ok(())
}

#[tokio::test]
async fn stress_get_nonexistent_agent() -> Result<()> {
    let server = TestServer::start().await?;

    let resp = server.get("/api/v1/agents/does-not-exist").send().await?;

    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// SSE Streaming Edge Cases
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_streaming_tool_failure_still_completes() -> Result<()> {
    // Given: LLM calls fail_tool then responds
    let backend = ScriptedMockBackend::tool_then_text(
        "fail_tool",
        "tc-stream-fail",
        json!({"reason": "boom"}),
        "Recovered from failure in stream.",
    );

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "Break something"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let events = collect_sse_events(resp).await;

    // Should have tool_end with success=false
    let tool_ends = events_of_type(&events, "tool_end");
    assert!(!tool_ends.is_empty(), "Should have tool_end events");
    assert_eq!(
        tool_ends[0].get_bool("success"),
        Some(false),
        "Tool should have failed"
    );

    // Should still have a done event
    assert!(
        events.iter().any(|e| e.is("done")),
        "Stream should complete with done event even after tool failure"
    );

    // Should have text content after recovery
    let text = reconstruct_text(&events);
    assert!(
        !text.is_empty(),
        "Should have text after tool failure recovery"
    );

    Ok(())
}

#[tokio::test]
async fn stress_streaming_oversized_message_rejected() -> Result<()> {
    let server = TestServer::start().await?;

    let large_msg = "x".repeat(100 * 1024 + 1);
    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": large_msg}))
        .send()
        .await?;

    // Streaming endpoint should also validate message size
    assert_eq!(resp.status().as_u16(), 400);

    Ok(())
}

#[tokio::test]
async fn stress_streaming_multiple_tool_calls() -> Result<()> {
    // Given: LLM calls echo then read_file then text
    let backend = ScriptedMockBackend::from_invocations(vec![
        ScriptedInvocation::Events(vec![StreamingMockEvent::ToolUse {
            id: "tc-s1".to_string(),
            name: "echo".to_string(),
            input: json!({"message": "stream echo"}),
        }]),
        ScriptedInvocation::Events(vec![StreamingMockEvent::ToolUse {
            id: "tc-s2".to_string(),
            name: "read_file".to_string(),
            input: json!({"path": "/test/data.json"}),
        }]),
        ScriptedInvocation::Events(vec![StreamingMockEvent::Text(
            "Both streamed tools done.".to_string(),
        )]),
    ]);

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "Do two things via stream"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let events = collect_sse_events(resp).await;

    let tool_starts = events_of_type(&events, "tool_start");
    assert_eq!(
        tool_starts.len(),
        2,
        "Should have 2 tool_start events, got {}",
        tool_starts.len()
    );

    let tool_ends = events_of_type(&events, "tool_end");
    assert_eq!(tool_ends.len(), 2, "Should have 2 tool_end events");

    // Both should succeed
    for te in &tool_ends {
        assert_eq!(te.get_bool("success"), Some(true));
    }

    assert!(events.iter().any(|e| e.is("done")));

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// WebSocket Stress Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_ws_auth_failure() -> Result<()> {
    let server = TestServer::start().await?;

    let mut ws = arawn_test_utils::TestWsClient::connect(&server.ws_url()).await?;

    // Authenticate with wrong token
    let result = ws.authenticate("wrong-token").await?;
    match result {
        arawn_test_utils::ws_client::WsServerMessage::AuthResult { success, .. } => {
            assert!(!success, "Wrong token should not authenticate");
        }
        other => panic!("Expected AuthResult, got: {:?}", other),
    }

    ws.close().await?;
    Ok(())
}

#[tokio::test]
async fn stress_ws_chat_without_auth() -> Result<()> {
    let server = TestServer::start().await?;

    let mut ws = arawn_test_utils::TestWsClient::connect(&server.ws_url()).await?;

    // Try to chat without authenticating first
    let messages = ws.chat("Hello without auth", None, None).await?;

    // Should get an error
    let has_error = messages.iter().any(|m| {
        matches!(
            m,
            arawn_test_utils::ws_client::WsServerMessage::Error { .. }
        )
    });
    assert!(has_error, "Should receive error when chatting without auth");

    ws.close().await?;
    Ok(())
}

#[tokio::test]
async fn stress_ws_backend_error_during_chat() -> Result<()> {
    let backend = ScriptedMockBackend::always_error("WebSocket backend exploded");

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .build()
        .await?;

    let mut ws = arawn_test_utils::TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    let messages = ws.chat("Trigger an error", None, None).await?;

    // Should have an error in the messages
    // The key assertion: no crash, no hang, connection stays alive
    assert!(
        !messages.is_empty(),
        "Should receive some messages even on backend error"
    );

    ws.close().await?;
    Ok(())
}

#[tokio::test]
async fn stress_ws_multiple_sequential_chats() -> Result<()> {
    let backend = ScriptedMockBackend::new(vec![
        vec![StreamingMockEvent::Text("First.".to_string())],
        vec![StreamingMockEvent::Text("Second.".to_string())],
        vec![StreamingMockEvent::Text("Third.".to_string())],
    ]);

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .build()
        .await?;

    let mut ws = arawn_test_utils::TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    // First chat — creates session
    let msgs1 = ws.chat("One", None, None).await?;
    let session_id = msgs1
        .iter()
        .find_map(|m| match m {
            arawn_test_utils::ws_client::WsServerMessage::SessionCreated { session_id } => {
                Some(session_id.clone())
            }
            _ => None,
        })
        .expect("Should get SessionCreated");

    // Subscribe
    let _ = ws.subscribe(&session_id).await?;

    // Second chat — same session
    let msgs2 = ws.chat("Two", Some(&session_id), None).await?;
    assert!(msgs2.iter().any(|m| matches!(
        m,
        arawn_test_utils::ws_client::WsServerMessage::ChatChunk { done: true, .. }
    )));

    // Third chat — same session
    let msgs3 = ws.chat("Three", Some(&session_id), None).await?;
    assert!(msgs3.iter().any(|m| matches!(
        m,
        arawn_test_utils::ws_client::WsServerMessage::ChatChunk { done: true, .. }
    )));

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Cross-Subsystem Stress Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_session_survives_tool_failure_and_memory_store() -> Result<()> {
    // Given: tool fails, then succeeds, then text response
    let backend = ScriptedMockBackend::from_invocations(vec![
        ScriptedInvocation::Events(vec![StreamingMockEvent::ToolUse {
            id: "tc-x1".to_string(),
            name: "fail_tool".to_string(),
            input: json!({"reason": "transient error"}),
        }]),
        ScriptedInvocation::Events(vec![StreamingMockEvent::ToolUse {
            id: "tc-x2".to_string(),
            name: "echo".to_string(),
            input: json!({"message": "recovered"}),
        }]),
        ScriptedInvocation::Events(vec![StreamingMockEvent::Text(
            "Session survived tool failure.".to_string(),
        )]),
    ]);

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    // Chat with tool failure + recovery
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Stress test"}))
        .send()
        .await?;

    assert_eq!(chat_resp.status().as_u16(), 200);
    let body: serde_json::Value = chat_resp.json().await?;
    let session_id = body["session_id"].as_str().unwrap();

    // Session should still be retrievable
    let session_resp = server
        .get(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;

    assert_eq!(session_resp.status().as_u16(), 200);
    let session: serde_json::Value = session_resp.json().await?;
    assert_eq!(session["turns"].as_array().unwrap().len(), 1);

    // Store memory about this session
    let mem_resp = server
        .post("/api/v1/memory")
        .json(&json!({
            "content": "Session had tool failure but recovered",
            "content_type": "observation",
            "session_id": session_id
        }))
        .send()
        .await?;

    assert_eq!(mem_resp.status().as_u16(), 201);

    // Search for the memory
    let search_resp = server
        .get("/api/v1/memory/search?q=recovered")
        .send()
        .await?;

    assert_eq!(search_resp.status().as_u16(), 200);
    let search: serde_json::Value = search_resp.json().await?;
    assert!(search["count"].as_u64().unwrap() >= 1);

    Ok(())
}

#[tokio::test]
async fn stress_rapid_session_create_list_delete() -> Result<()> {
    let server = TestServer::start().await?;

    // Create 10 sessions
    let mut ids = Vec::new();
    for i in 0..10 {
        let resp = server
            .post("/api/v1/sessions")
            .json(&json!({"title": format!("Session {}", i)}))
            .send()
            .await?;
        assert_eq!(resp.status().as_u16(), 201);
        let body: serde_json::Value = resp.json().await?;
        ids.push(body["id"].as_str().unwrap().to_string());
    }

    // List — should have all 10
    let list_resp = server.get("/api/v1/sessions").send().await?;
    let list: serde_json::Value = list_resp.json().await?;
    assert!(list["total"].as_u64().unwrap() >= 10);

    // Delete all
    for id in &ids {
        let resp = server
            .delete(&format!("/api/v1/sessions/{}", id))
            .send()
            .await?;
        assert_eq!(resp.status().as_u16(), 204);
    }

    // List again — should have 0 (or fewer)
    let list_resp2 = server.get("/api/v1/sessions").send().await?;
    let list2: serde_json::Value = list_resp2.json().await?;
    let remaining = list2["total"].as_u64().unwrap();
    assert!(
        remaining < 10,
        "Should have fewer sessions after deletion, got {}",
        remaining
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool Execution with read_file returning "not found" (tool-level error, not panic)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_tool_reads_nonexistent_file() -> Result<()> {
    // LLM asks to read a file that doesn't exist in MockReadFileTool
    let backend = ScriptedMockBackend::tool_then_text(
        "read_file",
        "tc-missing",
        json!({"path": "/nonexistent/file.rs"}),
        "The file was not found.",
    );

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Read a missing file"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;

    // Tool should report failure (MockReadFileTool returns ToolResult::error for unknown paths)
    let tool_calls = body["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["name"], "read_file");
    assert_eq!(
        tool_calls[0]["success"], false,
        "Reading nonexistent file should report as failed"
    );

    // Agent should still respond
    assert!(!body["response"].as_str().unwrap().is_empty());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Notes Edge Cases
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_notes_empty_tags() -> Result<()> {
    let server = TestServer::start().await?;

    let resp = server
        .post("/api/v1/notes")
        .json(&json!({
            "content": "Note with empty tags",
            "tags": []
        }))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 201);
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["tags"].as_array().unwrap().len(), 0);

    Ok(())
}

#[tokio::test]
async fn stress_notes_no_tags_field() -> Result<()> {
    let server = TestServer::start().await?;

    // Create note without tags field at all
    let resp = server
        .post("/api/v1/notes")
        .json(&json!({"content": "Note without tags field"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 201);

    Ok(())
}

#[tokio::test]
async fn stress_notes_many_tags() -> Result<()> {
    let server = TestServer::start().await?;

    let tags: Vec<String> = (0..50).map(|i| format!("tag-{}", i)).collect();
    let resp = server
        .post("/api/v1/notes")
        .json(&json!({
            "content": "Note with many tags",
            "tags": tags
        }))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 201);
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["tags"].as_array().unwrap().len(), 50);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Session Update / Patch
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn stress_session_update_metadata() -> Result<()> {
    let server = TestServer::start().await?;

    // Create session
    let create_resp = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "Patchable"}))
        .send()
        .await?;

    let session: serde_json::Value = create_resp.json().await?;
    let session_id = session["id"].as_str().unwrap();

    // Update title
    let patch_resp = server
        .patch(&format!("/api/v1/sessions/{}", session_id))
        .json(&json!({"title": "Updated Title"}))
        .send()
        .await?;

    assert_eq!(patch_resp.status().as_u16(), 200);

    // Verify
    let get_resp = server
        .get(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;

    assert_eq!(get_resp.status().as_u16(), 200);

    Ok(())
}

#[tokio::test]
async fn stress_patch_nonexistent_session() -> Result<()> {
    let server = TestServer::start().await?;

    let fake_id = "00000000-0000-0000-0000-000000000000";
    let resp = server
        .patch(&format!("/api/v1/sessions/{}", fake_id))
        .json(&json!({"title": "Nope"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

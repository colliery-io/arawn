//! E2E tests for the sessions endpoints.
//!
//! These tests exercise POST/GET/PATCH/DELETE /api/v1/sessions
//! and GET /api/v1/sessions/:id/messages.

mod common;

use anyhow::Result;
use serde_json::json;

use arawn_test_utils::server::TestServerBuilder;

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Create a session with title
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_create_session_with_title() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "My Test Session"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 201);

    let body: serde_json::Value = resp.json().await?;
    assert!(body["id"].as_str().is_some(), "Should have an id");
    assert_eq!(body["metadata"]["title"], "My Test Session");
    assert!(body["turns"].as_array().unwrap().is_empty());
    assert!(body["created_at"].as_str().is_some());
    assert!(body["updated_at"].as_str().is_some());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Create a session with metadata
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_create_session_with_metadata() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .post("/api/v1/sessions")
        .json(&json!({
            "title": "Metadata Session",
            "metadata": {
                "project": "arawn",
                "priority": 1
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 201);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["metadata"]["title"], "Metadata Session");
    assert_eq!(body["metadata"]["project"], "arawn");
    assert_eq!(body["metadata"]["priority"], 1);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Create a session with no title (minimal request)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_create_session_minimal() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .post("/api/v1/sessions")
        .json(&json!({}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 201);

    let body: serde_json::Value = resp.json().await?;
    assert!(body["id"].as_str().is_some());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: List sessions empty
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_list_sessions_empty() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/sessions").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["total"], 0);
    assert!(body["sessions"].as_array().unwrap().is_empty());
    assert!(body["limit"].as_u64().unwrap() > 0);
    assert_eq!(body["offset"], 0);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: List sessions after creating multiple
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_list_sessions_with_data() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create 3 sessions
    for i in 1..=3 {
        let resp = server
            .post("/api/v1/sessions")
            .json(&json!({"title": format!("Session {}", i)}))
            .send()
            .await?;
        assert_eq!(resp.status().as_u16(), 201);
    }

    let resp = server.get("/api/v1/sessions").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["total"], 3);
    let sessions = body["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 3);

    // Each session should have required fields
    for session in sessions {
        assert!(session["id"].as_str().is_some());
        assert!(session["created_at"].as_str().is_some());
        assert!(session["updated_at"].as_str().is_some());
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: List sessions with pagination (limit)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_list_sessions_with_limit() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create 5 sessions
    for i in 1..=5 {
        server
            .post("/api/v1/sessions")
            .json(&json!({"title": format!("Session {}", i)}))
            .send()
            .await?;
    }

    let resp = server.get("/api/v1/sessions?limit=2").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["total"], 5, "Total should reflect all sessions");
    assert_eq!(
        body["sessions"].as_array().unwrap().len(),
        2,
        "Should only return 2 sessions"
    );
    assert_eq!(body["limit"], 2);
    assert_eq!(body["offset"], 0);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: List sessions with pagination (offset)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_list_sessions_with_offset() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create 5 sessions
    for i in 1..=5 {
        server
            .post("/api/v1/sessions")
            .json(&json!({"title": format!("Session {}", i)}))
            .send()
            .await?;
    }

    let resp = server
        .get("/api/v1/sessions?limit=2&offset=3")
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["total"], 5);
    assert_eq!(
        body["sessions"].as_array().unwrap().len(),
        2,
        "Should return 2 sessions at offset 3"
    );
    assert_eq!(body["offset"], 3);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get session by ID
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_get_session_by_id() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create a session
    let create_resp = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "Get Me"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let session_id = created["id"].as_str().unwrap();

    // Get session
    let resp = server
        .get(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["id"], session_id);
    assert_eq!(body["metadata"]["title"], "Get Me");
    assert!(body["turns"].as_array().unwrap().is_empty());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get session not found (404)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_get_session_not_found() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .get("/api/v1/sessions/00000000-0000-0000-0000-000000000000")
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get session with invalid ID (400)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_get_session_invalid_id() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/sessions/not-a-uuid").send().await?;
    assert_eq!(
        resp.status().as_u16(),
        400,
        "Invalid UUID should return 400"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Delete session
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_delete_session() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create a session
    let create_resp = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "Delete Me"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let session_id = created["id"].as_str().unwrap();

    // Delete
    let resp = server
        .delete(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 204);

    // Verify deleted
    let get_resp = server
        .get(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;
    assert_eq!(get_resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Delete nonexistent session (404)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_delete_nonexistent_session() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .delete("/api/v1/sessions/00000000-0000-0000-0000-000000000000")
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Delete session with invalid ID (400)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_delete_session_invalid_id() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server.delete("/api/v1/sessions/not-a-uuid").send().await?;
    assert_eq!(resp.status().as_u16(), 400);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Update session title
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_update_session_title() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create a session
    let create_resp = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "Original Title"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let session_id = created["id"].as_str().unwrap();

    // Update title
    let resp = server
        .patch(&format!("/api/v1/sessions/{}", session_id))
        .json(&json!({"title": "Updated Title"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["metadata"]["title"], "Updated Title");

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Update session metadata
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_update_session_metadata() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create session with initial metadata
    let create_resp = server
        .post("/api/v1/sessions")
        .json(&json!({
            "title": "Meta Session",
            "metadata": {"existing_key": "value1"}
        }))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let session_id = created["id"].as_str().unwrap();

    // Update with new metadata (should merge)
    let resp = server
        .patch(&format!("/api/v1/sessions/{}", session_id))
        .json(&json!({
            "metadata": {"new_key": "value2", "existing_key": "overwritten"}
        }))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["metadata"]["existing_key"], "overwritten");
    assert_eq!(body["metadata"]["new_key"], "value2");
    // Title should still be there
    assert_eq!(body["metadata"]["title"], "Meta Session");

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Update nonexistent session (404)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_update_nonexistent_session() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .patch("/api/v1/sessions/00000000-0000-0000-0000-000000000000")
        .json(&json!({"title": "Nope"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Update session with invalid ID (400)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_update_session_invalid_id() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .patch("/api/v1/sessions/not-a-uuid")
        .json(&json!({"title": "Nope"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 400);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get messages for session with no turns
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_get_messages_empty_session() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create a session
    let create_resp = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "Empty Messages"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let session_id = created["id"].as_str().unwrap();

    let resp = server
        .get(&format!("/api/v1/sessions/{}/messages", session_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["session_id"], session_id);
    assert_eq!(body["count"], 0);
    assert!(body["messages"].as_array().unwrap().is_empty());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get messages for session created via chat (has turns)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_get_messages_with_conversation() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create session via chat (which populates turns)
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Hello from E2E test"}))
        .send()
        .await?;
    assert_eq!(chat_resp.status().as_u16(), 200);
    let chat_body: serde_json::Value = chat_resp.json().await?;
    let session_id = chat_body["session_id"].as_str().unwrap();

    // Get messages
    let resp = server
        .get(&format!("/api/v1/sessions/{}/messages", session_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["session_id"], session_id);
    assert!(
        body["count"].as_u64().unwrap() >= 2,
        "Should have at least user + assistant messages"
    );

    let messages = body["messages"].as_array().unwrap();
    // First message should be the user message
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[0]["content"], "Hello from E2E test");
    assert!(messages[0]["timestamp"].as_str().is_some());

    // Last message should be assistant
    let last = messages.last().unwrap();
    assert_eq!(last["role"], "assistant");

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get messages for nonexistent session (404)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_get_messages_not_found() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .get("/api/v1/sessions/00000000-0000-0000-0000-000000000000/messages")
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get messages with invalid session ID (400)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_get_messages_invalid_id() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .get("/api/v1/sessions/not-a-uuid/messages")
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 400);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Delete then list verifies removal
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_delete_session_then_list() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create 2 sessions
    let resp1 = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "Keep"}))
        .send()
        .await?;
    let s1: serde_json::Value = resp1.json().await?;

    let resp2 = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "Remove"}))
        .send()
        .await?;
    let s2: serde_json::Value = resp2.json().await?;
    let remove_id = s2["id"].as_str().unwrap();

    // Delete second session
    let del_resp = server
        .delete(&format!("/api/v1/sessions/{}", remove_id))
        .send()
        .await?;
    assert_eq!(del_resp.status().as_u16(), 204);

    // List should only have 1 session
    let list_resp = server.get("/api/v1/sessions").send().await?;
    let list_body: serde_json::Value = list_resp.json().await?;
    assert_eq!(list_body["total"], 1);

    let sessions = list_body["sessions"].as_array().unwrap();
    assert_eq!(sessions[0]["id"], s1["id"]);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Session created via chat appears in list
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_chat_session_appears_in_list() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create session via chat
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Create via chat"}))
        .send()
        .await?;
    assert_eq!(chat_resp.status().as_u16(), 200);
    let chat_body: serde_json::Value = chat_resp.json().await?;
    let session_id = chat_body["session_id"].as_str().unwrap();

    // Session should appear in list
    let list_resp = server.get("/api/v1/sessions").send().await?;
    let list_body: serde_json::Value = list_resp.json().await?;
    assert!(list_body["total"].as_u64().unwrap() >= 1);

    let sessions = list_body["sessions"].as_array().unwrap();
    assert!(
        sessions.iter().any(|s| s["id"] == session_id),
        "Chat-created session should appear in session list"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Session detail includes turns from chat
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_session_detail_with_turns() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create session via chat
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Turn 1"}))
        .send()
        .await?;
    let chat_body: serde_json::Value = chat_resp.json().await?;
    let session_id = chat_body["session_id"].as_str().unwrap();

    // Get session detail
    let resp = server
        .get(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    let turns = body["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 1, "Should have 1 turn");

    let turn = &turns[0];
    assert!(turn["id"].as_str().is_some());
    assert_eq!(turn["user_message"], "Turn 1");
    assert!(turn["assistant_response"].as_str().is_some());
    assert!(turn["started_at"].as_str().is_some());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Update session title and metadata together
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_update_session_title_and_metadata() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let create_resp = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "Original"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let session_id = created["id"].as_str().unwrap();

    // Update both title and metadata in one PATCH
    let resp = server
        .patch(&format!("/api/v1/sessions/{}", session_id))
        .json(&json!({
            "title": "New Title",
            "metadata": {"tag": "important"}
        }))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["metadata"]["title"], "New Title");
    assert_eq!(body["metadata"]["tag"], "important");

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Update session with workstream_id requires workstreams
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_update_session_workstream_without_workstreams() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create a session
    let create_resp = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "Move Me"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let session_id = created["id"].as_str().unwrap();

    // Try to reassign to a workstream when workstreams aren't configured
    let resp = server
        .patch(&format!("/api/v1/sessions/{}", session_id))
        .json(&json!({"workstream_id": "some-workstream"}))
        .send()
        .await?;
    assert_eq!(
        resp.status().as_u16(),
        400,
        "Should fail when workstreams not configured"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Update session with invalid workstream ID
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_update_session_invalid_workstream_id() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let create_resp = server
        .post("/api/v1/sessions")
        .json(&json!({}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let session_id = created["id"].as_str().unwrap();

    // Invalid workstream ID (path traversal attempt)
    let resp = server
        .patch(&format!("/api/v1/sessions/{}", session_id))
        .json(&json!({"workstream_id": "../../../etc/passwd"}))
        .send()
        .await?;
    assert_eq!(
        resp.status().as_u16(),
        400,
        "Path traversal should be rejected"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Create session with workstreams enabled
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_create_session_with_workstreams() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    let resp = server
        .post("/api/v1/sessions")
        .json(&json!({"title": "WS Session"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 201);

    let body: serde_json::Value = resp.json().await?;
    assert!(body["id"].as_str().is_some());
    assert_eq!(body["metadata"]["title"], "WS Session");
    // Should have workstream_id when workstreams are enabled
    assert!(body["workstream_id"].as_str().is_some());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: List sessions with workstreams merges sources
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_list_sessions_with_workstreams() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    // Create a session via chat (this stores in workstream storage too)
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Workstream chat"}))
        .send()
        .await?;
    assert_eq!(chat_resp.status().as_u16(), 200);

    let resp = server.get("/api/v1/sessions").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert!(body["total"].as_u64().unwrap() >= 1);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get session with workstreams (fallback to storage)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_get_session_with_workstreams() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    // Create session via chat
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "WS get test"}))
        .send()
        .await?;
    let chat_body: serde_json::Value = chat_resp.json().await?;
    let session_id = chat_body["session_id"].as_str().unwrap();

    // Get session detail (hits cache path first)
    let resp = server
        .get(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["id"], session_id);
    assert!(body["workstream_id"].as_str().is_some());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Delete session with workstreams
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_delete_session_with_workstreams() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    // Create session via chat
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Delete ws session"}))
        .send()
        .await?;
    let chat_body: serde_json::Value = chat_resp.json().await?;
    let session_id = chat_body["session_id"].as_str().unwrap();

    // Delete
    let resp = server
        .delete(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 204);

    // Should be gone
    let get_resp = server
        .get(&format!("/api/v1/sessions/{}", session_id))
        .send()
        .await?;
    assert_eq!(get_resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get messages with workstreams
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_get_messages_with_workstreams() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    // Create session via chat
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "WS messages test"}))
        .send()
        .await?;
    let chat_body: serde_json::Value = chat_resp.json().await?;
    let session_id = chat_body["session_id"].as_str().unwrap();

    let resp = server
        .get(&format!("/api/v1/sessions/{}/messages", session_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["session_id"], session_id);
    assert!(body["count"].as_u64().unwrap() >= 2);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Reassign session to nonexistent workstream returns error
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_reassign_session_nonexistent_workstream() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    // Create session via chat
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Reassign me"}))
        .send()
        .await?;
    let chat_body: serde_json::Value = chat_resp.json().await?;
    let session_id = chat_body["session_id"].as_str().unwrap();

    // Try to reassign to a workstream that doesn't exist in the store
    let resp = server
        .patch(&format!("/api/v1/sessions/{}", session_id))
        .json(&json!({"workstream_id": "nonexistent-ws"}))
        .send()
        .await?;
    assert_eq!(
        resp.status().as_u16(),
        400,
        "Should fail when target workstream not found"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Session list summary fields are correct
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_list_sessions_summary_fields() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create via chat so there's a turn
    let chat_resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Hello"}))
        .send()
        .await?;
    let chat_body: serde_json::Value = chat_resp.json().await?;
    let session_id = chat_body["session_id"].as_str().unwrap();

    let resp = server.get("/api/v1/sessions").send().await?;
    let body: serde_json::Value = resp.json().await?;
    let sessions = body["sessions"].as_array().unwrap();

    let session = sessions
        .iter()
        .find(|s| s["id"] == session_id)
        .expect("Session should be in list");

    assert_eq!(session["turn_count"], 1);
    assert!(session["created_at"].as_str().is_some());
    assert!(session["updated_at"].as_str().is_some());

    Ok(())
}

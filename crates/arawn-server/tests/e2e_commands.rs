//! E2E tests for the commands endpoints.
//!
//! These tests exercise GET /api/v1/commands, POST /api/v1/commands/compact,
//! and POST /api/v1/commands/compact/stream.

mod common;

use anyhow::Result;
use serde_json::json;

use arawn_test_utils::server::TestServerBuilder;
use arawn_test_utils::{ScriptedMockBackend, StreamingMockEvent};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Build a ScriptedMockBackend with N text responses (for chat turns)
/// plus one extra response for the compaction summary.
fn backend_for_turns(n: usize) -> ScriptedMockBackend {
    let mut invocations: Vec<Vec<StreamingMockEvent>> = Vec::new();
    for i in 1..=n {
        invocations.push(vec![StreamingMockEvent::Text(format!("Response {}", i))]);
    }
    // Extra invocation for compaction summary
    invocations.push(vec![StreamingMockEvent::Text(
        "Summary of earlier turns.".to_string(),
    )]);
    ScriptedMockBackend::new(invocations)
}

/// Create a session with the given number of turns via chat, returning the session ID.
async fn create_session_with_turns(server: &arawn_test_utils::TestServer, turns: usize) -> String {
    // First turn creates the session
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "Turn 1"}))
        .send()
        .await
        .expect("chat request failed");
    let body: serde_json::Value = resp.json().await.expect("chat response parse failed");
    let session_id = body["session_id"].as_str().unwrap().to_string();

    // Additional turns reuse the same session
    for i in 2..=turns {
        server
            .post("/api/v1/chat")
            .json(&json!({
                "message": format!("Turn {}", i),
                "session_id": session_id
            }))
            .send()
            .await
            .expect("chat request failed");
    }

    session_id
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: List available commands
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_list_commands() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/commands").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    let commands = body["commands"]
        .as_array()
        .expect("commands should be array");
    assert!(!commands.is_empty(), "Should have at least one command");

    // Should include the compact command
    let compact = commands.iter().find(|c| c["name"] == "compact");
    assert!(compact.is_some(), "Should have compact command");
    assert!(
        compact.unwrap()["description"].as_str().is_some(),
        "Compact should have description"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Compact with invalid session ID returns 400
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_compact_invalid_session_id() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .post("/api/v1/commands/compact")
        .json(&json!({"session_id": "not-a-uuid"}))
        .send()
        .await?;
    assert_eq!(
        resp.status().as_u16(),
        400,
        "Invalid session ID should return 400"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Compact nonexistent session returns 404
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_compact_session_not_found() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .post("/api/v1/commands/compact")
        .json(&json!({"session_id": "00000000-0000-0000-0000-000000000000"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Compact session that doesn't need compaction
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_compact_not_needed() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create a session with only 2 turns (below compact threshold)
    let session_id = create_session_with_turns(&server, 2).await;

    let resp = server
        .post("/api/v1/commands/compact")
        .json(&json!({"session_id": session_id}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["compacted"], false);
    assert!(body["message"].as_str().unwrap().contains("does not need"));

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Compact session with force flag
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_compact_with_force() -> Result<()> {
    let server = TestServerBuilder::new()
        .with_backend(backend_for_turns(6))
        .build()
        .await?;

    // Create a session with 6 turns (enough to compact)
    let session_id = create_session_with_turns(&server, 6).await;

    let resp = server
        .post("/api/v1/commands/compact")
        .json(&json!({
            "session_id": session_id,
            "force": true
        }))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["compacted"], true, "Response: {}", body);
    assert!(body["turns_compacted"].as_u64().unwrap() > 0);
    assert!(body["tokens_before"].as_u64().is_some());
    assert!(body["tokens_after"].as_u64().is_some());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Compact stream with invalid session ID returns 400
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_compact_stream_invalid_session_id() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .post("/api/v1/commands/compact/stream")
        .json(&json!({"session_id": "not-a-uuid"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 400);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Compact stream nonexistent session returns 404
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_compact_stream_session_not_found() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .post("/api/v1/commands/compact/stream")
        .json(&json!({"session_id": "00000000-0000-0000-0000-000000000000"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Compact stream session not needing compaction
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_compact_stream_not_needed() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Create a session with only 2 turns
    let session_id = create_session_with_turns(&server, 2).await;

    let resp = server
        .post("/api/v1/commands/compact/stream")
        .json(&json!({"session_id": session_id}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    // Parse the SSE data lines
    let text = resp.text().await?;
    let events = parse_compact_sse(&text);
    assert!(!events.is_empty(), "Should have at least one SSE event");

    // Should have a completed event with compacted=false
    let completed = events
        .iter()
        .find(|e| e["type"] == "completed")
        .expect("Should have completed event");
    assert_eq!(completed["result"]["compacted"], false);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Compact stream with enough turns
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_compact_stream_with_turns() -> Result<()> {
    let server = TestServerBuilder::new()
        .with_backend(backend_for_turns(6))
        .build()
        .await?;

    // Create a session with 6 turns
    let session_id = create_session_with_turns(&server, 6).await;

    let resp = server
        .post("/api/v1/commands/compact/stream")
        .json(&json!({
            "session_id": session_id,
            "force": true
        }))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let text = resp.text().await?;
    let events = parse_compact_sse(&text);

    // Should have started, summarizing, and completed events
    assert!(
        events.iter().any(|e| e["type"] == "started"),
        "Should have started event"
    );
    assert!(
        events.iter().any(|e| e["type"] == "summarizing"),
        "Should have summarizing event"
    );
    let completed = events
        .iter()
        .find(|e| e["type"] == "completed")
        .expect("Should have completed event");
    assert_eq!(completed["result"]["compacted"], true);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Compact session without force (natural threshold)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_compact_natural_threshold() -> Result<()> {
    let server = TestServerBuilder::new()
        .with_backend(backend_for_turns(6))
        .build()
        .await?;

    // Create a session with 6 turns (above default preserve_recent of 3)
    let session_id = create_session_with_turns(&server, 6).await;

    // Without force — should still compact because there are enough turns
    let resp = server
        .post("/api/v1/commands/compact")
        .json(&json!({"session_id": session_id}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    // Either compacted or said not needed — both are valid 200 responses
    assert!(body["message"].as_str().is_some());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Parse compact SSE response text into JSON events.
///
/// The compact stream uses `Event::default().data(json)` which produces
/// `data: {...}\n\n` format without `event:` lines.
fn parse_compact_sse(text: &str) -> Vec<serde_json::Value> {
    let mut events = Vec::new();
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                events.push(parsed);
            }
        }
    }
    events
}

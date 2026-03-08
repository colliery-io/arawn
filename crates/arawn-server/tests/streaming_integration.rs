//! SSE streaming chat integration tests.
//!
//! Tests the `/api/v1/chat/stream` endpoint which returns Server-Sent Events.

mod common;

use anyhow::Result;
use serde_json::json;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Collect SSE events from a streaming response into `(event_name, parsed_data)` pairs.
async fn collect_sse_events(resp: reqwest::Response) -> Vec<(String, serde_json::Value)> {
    let text = resp.text().await.unwrap();
    let mut events = Vec::new();

    let mut current_event = String::new();
    let mut current_data = String::new();

    for line in text.lines() {
        if let Some(ev) = line.strip_prefix("event: ") {
            current_event = ev.to_string();
        } else if let Some(d) = line.strip_prefix("data: ") {
            current_data = d.to_string();
        } else if line.is_empty() && !current_event.is_empty() {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&current_data) {
                events.push((current_event.clone(), data));
            }
            current_event.clear();
            current_data.clear();
        }
    }

    events
}

/// Reconstruct the full text content from SSE `text` events.
fn reconstruct_text(events: &[(String, serde_json::Value)]) -> String {
    events
        .iter()
        .filter(|(event, _)| event == "text")
        .filter_map(|(_, data)| data["content"].as_str())
        .collect::<Vec<_>>()
        .join("")
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_chat_stream_returns_sse() -> Result<()> {
    let server = common::TestServer::start().await?;

    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "hello"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 200, "Should return 200 OK");

    let content_type = resp
        .headers()
        .get("content-type")
        .expect("Should have Content-Type header")
        .to_str()?;
    assert!(
        content_type.starts_with("text/event-stream"),
        "Content-Type should start with text/event-stream, got: {}",
        content_type
    );

    Ok(())
}

#[tokio::test]
async fn test_chat_stream_session_event_first() -> Result<()> {
    let server = common::TestServer::start().await?;

    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "hello"}))
        .send()
        .await?;

    assert!(resp.status().is_success());

    let events = collect_sse_events(resp).await;
    assert!(!events.is_empty(), "Should receive at least one SSE event");

    let (first_event, first_data) = &events[0];
    assert_eq!(first_event, "session", "First event should be 'session'");
    assert!(
        first_data
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "Session event should contain a non-empty session_id"
    );

    Ok(())
}

#[tokio::test]
async fn test_chat_stream_contains_text_events() -> Result<()> {
    let server = common::TestServer::start().await?;

    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "hello"}))
        .send()
        .await?;

    assert!(resp.status().is_success());

    let events = collect_sse_events(resp).await;
    let text_events: Vec<_> = events.iter().filter(|(e, _)| e == "text").collect();

    assert!(
        !text_events.is_empty(),
        "Should contain at least one 'text' event"
    );

    for (_, data) in &text_events {
        let content = data
            .get("content")
            .and_then(|v| v.as_str())
            .expect("text event should have a content field");
        assert!(
            !content.is_empty(),
            "text event content should not be empty"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_chat_stream_ends_with_done_or_error() -> Result<()> {
    let server = common::TestServer::start().await?;

    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "hello"}))
        .send()
        .await?;

    assert!(resp.status().is_success());

    let events = collect_sse_events(resp).await;
    assert!(!events.is_empty(), "Should receive at least one event");

    let (last_event, last_data) = events.last().expect("events should not be empty");
    assert!(
        last_event == "done" || last_event == "error",
        "Last event should be 'done' or 'error', got: '{}'",
        last_event
    );

    if last_event == "done" {
        assert!(
            last_data.get("iterations").is_some(),
            "done event should have an 'iterations' field"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_chat_stream_text_content() -> Result<()> {
    let server = common::TestServer::start().await?;

    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "hello"}))
        .send()
        .await?;

    assert!(resp.status().is_success());

    let events = collect_sse_events(resp).await;
    let text = reconstruct_text(&events);

    assert!(
        text.contains("Test response"),
        "Reconstructed text should contain 'Test response', got: '{}'",
        text
    );

    Ok(())
}

#[tokio::test]
async fn test_chat_stream_requires_auth() -> Result<()> {
    let server = common::TestServer::start().await?;

    // Send request without bearer token by using the raw client
    let resp = server
        .client
        .post(format!("{}/api/v1/chat/stream", server.base_url()))
        .json(&json!({"message": "hi"}))
        .send()
        .await?;

    assert_eq!(
        resp.status().as_u16(),
        401,
        "Request without auth should return 401"
    );

    Ok(())
}

#[tokio::test]
async fn test_chat_stream_with_session_id() -> Result<()> {
    let server = common::TestServer::start_with_responses(vec![
        "First response".to_string(),
        "Second response".to_string(),
    ])
    .await?;

    // First request to get a session_id
    let resp = server
        .post("/api/v1/chat")
        .json(&json!({"message": "first"}))
        .send()
        .await?;

    assert!(resp.status().is_success(), "First chat should succeed");

    let body: serde_json::Value = resp.json().await?;
    let session_id = body
        .get("session_id")
        .and_then(|v| v.as_str())
        .expect("Should have session_id")
        .to_string();

    // Stream with the same session_id
    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "second", "session_id": session_id}))
        .send()
        .await?;

    assert!(resp.status().is_success(), "Stream request should succeed");

    let events = collect_sse_events(resp).await;
    assert!(!events.is_empty(), "Should receive events");

    let (first_event, first_data) = &events[0];
    assert_eq!(first_event, "session", "First event should be 'session'");

    let returned_session_id = first_data
        .get("session_id")
        .and_then(|v| v.as_str())
        .expect("Should have session_id in session event");

    assert_eq!(
        returned_session_id, session_id,
        "Stream should echo the same session_id"
    );

    Ok(())
}

#[tokio::test]
async fn test_chat_stream_message_too_large() -> Result<()> {
    let server = common::TestServer::start().await?;

    // 100KB = 100 * 1024 = 102400 bytes; send 102_401 bytes to exceed the limit
    let large_message = "x".repeat(102_401);

    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": large_message}))
        .send()
        .await?;

    assert_eq!(
        resp.status().as_u16(),
        400,
        "Message over 100KB should return 400"
    );

    Ok(())
}

#[tokio::test]
async fn test_chat_stream_multiple_chunks() -> Result<()> {
    let server =
        common::TestServer::start_with_responses(vec!["chunk1 chunk2 chunk3".into()]).await?;

    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": "hello"}))
        .send()
        .await?;

    assert!(resp.status().is_success());

    let events = collect_sse_events(resp).await;
    let text_events: Vec<_> = events.iter().filter(|(e, _)| e == "text").collect();

    assert!(
        !text_events.is_empty(),
        "Should have at least one text event"
    );

    let full_text = reconstruct_text(&events);
    assert!(
        full_text.contains("chunk1")
            && full_text.contains("chunk2")
            && full_text.contains("chunk3"),
        "Reconstructed text should contain all chunks, got: '{}'",
        full_text
    );

    Ok(())
}

#[tokio::test]
async fn test_chat_stream_empty_message() -> Result<()> {
    let server = common::TestServer::start().await?;

    let resp = server
        .post("/api/v1/chat/stream")
        .json(&json!({"message": ""}))
        .send()
        .await?;

    // Should get a response (success or 4xx client error), but not a 500
    assert_ne!(
        resp.status().as_u16(),
        500,
        "Empty message should not cause a 500 Internal Server Error"
    );

    Ok(())
}

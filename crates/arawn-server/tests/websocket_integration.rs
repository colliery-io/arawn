//! WebSocket E2E integration tests for the Arawn server.
//!
//! Exercises the WebSocket chat flow: auth, subscribe, chat, ping/pong,
//! and error handling using the shared test utilities.

use anyhow::Result;
use arawn_test_utils::ws_client::WsServerMessage;
use arawn_test_utils::{TestServer, TestWsClient};

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_connect_and_auth() -> Result<()> {
    let server = TestServer::start().await?;
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    let result = ws.authenticate("test-token").await?;

    match result {
        WsServerMessage::AuthResult { success, error } => {
            assert!(success, "Auth should succeed with valid token");
            assert!(error.is_none(), "No error expected on success");
        }
        other => panic!("Expected AuthResult, got: {:?}", other),
    }

    ws.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_ws_auth_invalid_token() -> Result<()> {
    let server = TestServer::start().await?;
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    let result = ws.authenticate("wrong-token").await?;

    match result {
        WsServerMessage::AuthResult { success, error } => {
            assert!(!success, "Auth should fail with invalid token");
            assert!(error.is_some(), "Error message expected on failure");
        }
        other => panic!("Expected AuthResult, got: {:?}", other),
    }

    Ok(())
}

#[tokio::test]
async fn test_ws_no_auth_mode() -> Result<()> {
    let server = TestServer::builder().with_auth(None).build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    // In no-auth (localhost) mode, subscribe should work without authentication.
    let session_id = uuid::Uuid::new_v4().to_string();
    let result = ws.subscribe(&session_id).await?;

    match result {
        WsServerMessage::SubscribeAck {
            session_id: sid, ..
        } => {
            assert_eq!(sid, session_id, "SubscribeAck should echo the session_id");
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    ws.close().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Ping / Pong
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_ping_pong() -> Result<()> {
    let server = TestServer::start().await?;
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    let _ = ws.authenticate("test-token").await?;
    ws.ping().await?; // panics internally if Pong is not received

    ws.close().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Subscribe
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_subscribe_gets_ownership() -> Result<()> {
    let server = TestServer::start().await?;
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    let _ = ws.authenticate("test-token").await?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let result = ws.subscribe(&session_id).await?;

    match result {
        WsServerMessage::SubscribeAck {
            session_id: sid,
            owner,
            reconnect_token,
        } => {
            assert_eq!(sid, session_id);
            assert!(owner, "First subscriber should be the owner");
            assert!(
                reconnect_token.is_some(),
                "Owner should receive a reconnect token"
            );
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    ws.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_ws_subscribe_invalid_session_id() -> Result<()> {
    let server = TestServer::start().await?;
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    let _ = ws.authenticate("test-token").await?;

    let result = ws.subscribe("not-a-uuid").await?;

    match result {
        WsServerMessage::Error { code, .. } => {
            assert_eq!(code, "invalid_session", "Expected invalid_session error");
        }
        other => panic!("Expected Error, got: {:?}", other),
    }

    Ok(())
}

#[tokio::test]
async fn test_ws_subscribe_requires_auth() -> Result<()> {
    let server = TestServer::start().await?;
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    // Do NOT authenticate — subscribe immediately.
    let session_id = uuid::Uuid::new_v4().to_string();
    let result = ws.subscribe(&session_id).await?;

    match result {
        WsServerMessage::Error { code, .. } => {
            assert_eq!(code, "unauthorized", "Expected unauthorized error");
        }
        other => panic!("Expected Error, got: {:?}", other),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Chat
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_chat_basic_flow() -> Result<()> {
    let server = TestServer::start().await?;
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    let _ = ws.authenticate("test-token").await?;

    let messages = ws.chat("Hello", None, None).await?;

    assert!(!messages.is_empty(), "Should receive at least one message");

    // First message should be SessionCreated.
    match &messages[0] {
        WsServerMessage::SessionCreated { session_id } => {
            assert!(!session_id.is_empty(), "session_id must not be empty");
        }
        other => panic!("Expected SessionCreated as first message, got: {:?}", other),
    }

    // There should be at least one ChatChunk.
    let has_chunk = messages
        .iter()
        .any(|m| matches!(m, WsServerMessage::ChatChunk { .. }));
    assert!(has_chunk, "Should contain ChatChunk messages");

    // Last message should be either the done sentinel or an agent error
    // (MockBackend may exhaust responses if the agent loop iterates).
    match messages.last().unwrap() {
        WsServerMessage::ChatChunk { done: true, .. } => {}
        WsServerMessage::Error { .. } => {} // acceptable terminal signal from mock
        other => panic!(
            "Last message should be ChatChunk(done=true) or Error, got: {:?}",
            other
        ),
    }

    ws.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_ws_chat_in_existing_session() -> Result<()> {
    let server = TestServer::start_with_responses(vec!["First".into(), "Second".into()]).await?;
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    let _ = ws.authenticate("test-token").await?;

    // First chat — creates a session.
    let first_messages = ws.chat("Hello", None, None).await?;
    let first_session_id = match &first_messages[0] {
        WsServerMessage::SessionCreated { session_id } => session_id.clone(),
        other => panic!("Expected SessionCreated, got: {:?}", other),
    };

    // Second chat — reuse the session.
    let second_messages = ws.chat("Follow-up", Some(&first_session_id), None).await?;

    // All ChatChunks in the second response should reference the same session.
    for msg in &second_messages {
        if let WsServerMessage::ChatChunk { session_id, .. } = msg {
            assert_eq!(
                session_id, &first_session_id,
                "Second chat should use the same session"
            );
        }
    }

    ws.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_ws_chat_requires_auth() -> Result<()> {
    let server = TestServer::start().await?;
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    // Do NOT authenticate — send chat immediately.
    ws.send_json(&serde_json::json!({
        "type": "chat",
        "message": "Hello"
    }))
    .await?;

    let msg = ws.recv().await?;

    match msg {
        WsServerMessage::Error { code, .. } => {
            assert_eq!(code, "unauthorized", "Expected unauthorized error");
        }
        other => panic!("Expected Error, got: {:?}", other),
    }

    Ok(())
}

#[tokio::test]
async fn test_ws_chat_response_contains_text() -> Result<()> {
    let server = TestServer::start().await?; // default response is "Test response"
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    let _ = ws.authenticate("test-token").await?;

    let messages = ws.chat("What is 2+2?", None, None).await?;

    let has_text = messages.iter().any(|m| match m {
        WsServerMessage::ChatChunk {
            chunk, done: false, ..
        } => chunk.contains("Test response"),
        _ => false,
    });
    assert!(
        has_text,
        "At least one ChatChunk should contain 'Test response'"
    );

    ws.close().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Malformed input
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_malformed_json() -> Result<()> {
    let server = TestServer::start().await?;
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    // Send a JSON object that has no recognised "type" field.
    ws.send_json(&serde_json::json!({"not_a_valid_message": true}))
        .await?;

    let msg = ws.recv().await?;

    match msg {
        WsServerMessage::Error { code, .. } => {
            assert_eq!(code, "parse_error", "Expected parse_error for bad JSON");
        }
        other => panic!("Expected Error, got: {:?}", other),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Multiple sessions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_multiple_sessions() -> Result<()> {
    let server = TestServer::start_with_responses(vec!["R1".into(), "R2".into()]).await?;
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    let _ = ws.authenticate("test-token").await?;

    // First chat — no session_id, should create a new session.
    let first = ws.chat("Message 1", None, None).await?;
    let sid1 = match &first[0] {
        WsServerMessage::SessionCreated { session_id } => session_id.clone(),
        other => panic!("Expected SessionCreated, got: {:?}", other),
    };

    // Second chat — also no session_id, should create a different session.
    let second = ws.chat("Message 2", None, None).await?;
    let sid2 = match &second[0] {
        WsServerMessage::SessionCreated { session_id } => session_id.clone(),
        other => panic!("Expected SessionCreated, got: {:?}", other),
    };

    assert_ne!(
        sid1, sid2,
        "Two chats without session_id should create distinct sessions"
    );

    ws.close().await?;
    Ok(())
}

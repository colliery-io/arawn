//! Integration tests for the TUI WebSocket client against a real server.
//!
//! These tests verify the actual message flow that users experience:
//! connect → (auth) → send chat → receive response.

use std::time::Duration;

use arawn_test_utils::server::TestServerBuilder;
use arawn_test_utils::ws_client::WsServerMessage;
use arawn_test_utils::{ScriptedMockBackend, StreamingMockEvent, TestWsClient};
use arawn_tui::client::WsClient;
use arawn_tui::protocol::ServerMessage;

/// Helper: create a test server WITH auth that responds with a single text message.
async fn text_server(response: &str) -> anyhow::Result<arawn_test_utils::TestServer> {
    let backend =
        ScriptedMockBackend::new(vec![vec![StreamingMockEvent::Text(response.to_string())]]);
    TestServerBuilder::new().with_backend(backend).build().await
}

/// Helper: create a test server WITHOUT auth (localhost mode).
async fn noauth_text_server(response: &str) -> anyhow::Result<arawn_test_utils::TestServer> {
    let backend =
        ScriptedMockBackend::new(vec![vec![StreamingMockEvent::Text(response.to_string())]]);
    TestServerBuilder::new()
        .with_auth(None)
        .with_backend(backend)
        .build()
        .await
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Baseline: TestWsClient works (proves server WS is functional)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_baseline_ws_client_chat() -> anyhow::Result<()> {
    let server = text_server("baseline response").await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    ws.authenticate(server.token.as_deref().unwrap_or("test-token"))
        .await?;

    let messages = ws.chat("hello", None, None).await?;

    // Should get SessionCreated + ChatChunk(s) + ChatChunk(done=true)
    assert!(!messages.is_empty(), "Expected WS messages from server");

    let has_session_created = messages
        .iter()
        .any(|m| matches!(m, WsServerMessage::SessionCreated { .. }));
    assert!(has_session_created, "Expected SessionCreated message");

    let has_chat_chunk = messages
        .iter()
        .any(|m| matches!(m, WsServerMessage::ChatChunk { .. }));
    assert!(has_chat_chunk, "Expected ChatChunk message");

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. TUI WsClient connects and receives messages
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_tui_ws_client_receives_messages() -> anyhow::Result<()> {
    let server = noauth_text_server("tui client test").await?;

    // Create the actual TUI WsClient (what the real app uses)
    let mut ws_client = WsClient::new(&server.base_url());

    // Wait for connection
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check connection status
    let status = ws_client.poll_status();
    println!("Connection status after 500ms: {:?}", status);

    // Send a chat message (no session, no workstream — server creates scratch session)
    ws_client.send_chat("hello from tui client".to_string(), None, None)?;

    // Wait for response
    let msg = tokio::time::timeout(Duration::from_secs(10), ws_client.recv()).await;

    match msg {
        Ok(Some(server_msg)) => {
            println!("Received: {:?}", server_msg);
            // Should be SessionCreated or ChatChunk or Error
            match server_msg {
                ServerMessage::SessionCreated { session_id } => {
                    println!("✓ SessionCreated: {}", session_id);
                    // Now wait for the chat chunks
                    let mut chunks = Vec::new();
                    loop {
                        match tokio::time::timeout(Duration::from_secs(5), ws_client.recv()).await {
                            Ok(Some(chunk_msg)) => {
                                println!("  chunk: {:?}", chunk_msg);
                                let done = matches!(
                                    &chunk_msg,
                                    ServerMessage::ChatChunk { done: true, .. }
                                );
                                chunks.push(chunk_msg);
                                if done {
                                    break;
                                }
                            }
                            Ok(None) => {
                                panic!("WS channel closed before done");
                            }
                            Err(_) => {
                                panic!(
                                    "Timeout waiting for ChatChunk (got {} chunks so far)",
                                    chunks.len()
                                );
                            }
                        }
                    }
                    assert!(!chunks.is_empty(), "Expected at least one ChatChunk");
                }
                ServerMessage::Error { code, message } => {
                    panic!("Server returned error: {} - {}", code, message);
                }
                other => {
                    println!("Unexpected first message: {:?}", other);
                    // Not necessarily a failure — could be auth-related
                }
            }
        }
        Ok(None) => {
            panic!("WS channel returned None — connection dropped");
        }
        Err(_) => {
            panic!("Timeout (10s) waiting for any WS message from server");
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 2b. TUI WsClient against NO-AUTH server (matches real user setup)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_tui_ws_client_noauth_server() -> anyhow::Result<()> {
    let server = noauth_text_server("noauth response").await?;

    let mut ws_client = WsClient::new(&server.base_url());

    tokio::time::sleep(Duration::from_millis(500)).await;

    let status = ws_client.poll_status();
    println!("Connection status (noauth): {:?}", status);

    ws_client.send_chat("hello noauth".to_string(), None, None)?;

    let msg = tokio::time::timeout(Duration::from_secs(10), ws_client.recv()).await;

    match msg {
        Ok(Some(server_msg)) => {
            println!("Received (noauth): {:?}", server_msg);
            match server_msg {
                ServerMessage::SessionCreated { session_id } => {
                    println!("✓ SessionCreated (noauth): {}", session_id);
                }
                ServerMessage::Error { code, message } => {
                    panic!("Server error even without auth: {} - {}", code, message);
                }
                other => {
                    println!("First message (noauth): {:?}", other);
                }
            }
        }
        Ok(None) => panic!("WS channel returned None"),
        Err(_) => panic!("Timeout (10s) waiting for WS message from noauth server"),
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. TUI App full message flow
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_tui_app_message_flow() -> anyhow::Result<()> {
    use arawn_tui::LogBuffer;
    use arawn_tui::app::App;

    let server = noauth_text_server("app flow response").await?;

    let mut app = App::new(server.base_url(), LogBuffer::new())?;

    // Wait for WS connection
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Poll connection status (mimics the tick handler)
    if let Some(status) = app.ws_client.poll_status() {
        app.connection_status = status;
    }
    println!("App connection status: {:?}", app.connection_status);

    // Simulate typing and sending
    app.input.set_text("test message");
    app.send_message();

    assert!(app.waiting, "App should be in waiting state after send");
    assert_eq!(app.messages.len(), 1, "Should have user message");
    assert!(app.messages[0].is_user);

    // Collect server messages for up to 10 seconds
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut received_count = 0;

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), app.ws_client.recv()).await {
            Ok(Some(msg)) => {
                println!("App received: {:?}", msg);
                app.handle_server_message(msg);
                received_count += 1;

                // Check if we got the complete response
                if !app.waiting {
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => continue, // timeout, try again
        }
    }

    println!("Received {} server messages", received_count);
    println!("Messages in app: {}", app.messages.len());
    for (i, msg) in app.messages.iter().enumerate() {
        println!(
            "  [{}] user={} streaming={} content={}",
            i,
            msg.is_user,
            msg.streaming,
            &msg.content[..msg.content.len().min(80)]
        );
    }

    assert!(
        received_count > 0,
        "Expected to receive at least one server message"
    );
    assert!(
        app.messages.len() >= 2,
        "Expected user message + assistant message, got {}",
        app.messages.len()
    );
    assert!(!app.waiting, "App should not still be waiting");

    Ok(())
}

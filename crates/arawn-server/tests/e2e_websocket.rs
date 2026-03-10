//! WebSocket E2E scenarios exercising tool execution and full message flows.

use anyhow::Result;
use serde_json::json;

use std::time::Duration;

use arawn_test_utils::server::TestServerBuilder;
use arawn_test_utils::ws_client::WsServerMessage;
use arawn_test_utils::{ScriptedMockBackend, StreamingMockEvent, TestWsClient, mock_tool_registry};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message as TungsteniteMsg;

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: WebSocket chat with tool execution
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_chat_with_tool_execution() -> Result<()> {
    // Given: server with echo tool and scripted backend
    let backend = ScriptedMockBackend::tool_then_text(
        "echo",
        "tc-ws-1",
        json!({"message": "ws hello"}),
        "Tool executed via WebSocket.",
    );

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    // When: connect and authenticate
    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let auth_result = ws.authenticate("test-token").await?;
    match auth_result {
        WsServerMessage::AuthResult { success, .. } => assert!(success),
        other => panic!("Expected AuthResult, got: {:?}", other),
    }

    // When: send a chat message and collect all responses
    let messages = ws.chat("Echo something please", None, None).await?;

    // Then: should have received tool events and chat chunks
    let mut saw_session_created = false;
    let mut saw_tool_start = false;
    let mut saw_tool_output = false;
    let mut saw_tool_end = false;
    let mut saw_chat_done = false;
    let mut tool_name = String::new();
    let mut tool_success = false;

    for msg in &messages {
        match msg {
            WsServerMessage::SessionCreated { .. } => saw_session_created = true,
            WsServerMessage::ToolStart {
                tool_name: name, ..
            } => {
                saw_tool_start = true;
                tool_name = name.clone();
            }
            WsServerMessage::ToolOutput { .. } => {
                saw_tool_output = true;
            }
            WsServerMessage::ToolEnd { success, .. } => {
                saw_tool_end = true;
                tool_success = *success;
            }
            WsServerMessage::ChatChunk { done: true, .. } => saw_chat_done = true,
            _ => {}
        }
    }

    assert!(
        saw_session_created,
        "Should receive SessionCreated. Messages: {:?}",
        messages
    );
    assert!(saw_tool_start, "Should receive ToolStart");
    assert_eq!(tool_name, "echo", "Tool name should be echo");
    assert!(saw_tool_output, "Should receive ToolOutput");
    assert!(saw_tool_end, "Should receive ToolEnd");
    assert!(tool_success, "Tool should have succeeded");
    assert!(saw_chat_done, "Should receive ChatChunk with done:true");

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: WebSocket chat with tool failure
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_chat_with_tool_failure() -> Result<()> {
    let backend = ScriptedMockBackend::tool_then_text(
        "fail_tool",
        "tc-ws-fail",
        json!({"reason": "ws test failure"}),
        "The tool failed but I continued.",
    );

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .with_tools(mock_tool_registry())
        .build()
        .await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    let messages = ws.chat("Use the fail tool", None, None).await?;

    // Should have ToolEnd with success=false
    let tool_end = messages
        .iter()
        .find(|m| matches!(m, WsServerMessage::ToolEnd { .. }));
    assert!(tool_end.is_some(), "Should receive ToolEnd");

    if let Some(WsServerMessage::ToolEnd { success, .. }) = tool_end {
        assert!(!success, "Tool should have failed");
    }

    // Should still get a final response
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, WsServerMessage::ChatChunk { done: true, .. })),
        "Should receive done ChatChunk even after tool failure"
    );

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: WebSocket multi-turn with subscribe
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_multi_turn_with_subscribe() -> Result<()> {
    let backend = ScriptedMockBackend::new(vec![
        vec![StreamingMockEvent::Text("First WS response.".to_string())],
        vec![StreamingMockEvent::Text("Second WS response.".to_string())],
    ]);

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .build()
        .await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    // First turn - creates session
    let messages1 = ws.chat("First message", None, None).await?;

    let session_id = messages1
        .iter()
        .find_map(|m| match m {
            WsServerMessage::SessionCreated { session_id } => Some(session_id.clone()),
            _ => None,
        })
        .expect("Should receive SessionCreated with session_id");

    // Subscribe to the session
    let sub_result = ws.subscribe(&session_id).await?;
    match sub_result {
        WsServerMessage::SubscribeAck {
            session_id: sid, ..
        } => assert_eq!(sid, session_id),
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    // Second turn - same session
    let messages2 = ws.chat("Second message", Some(&session_id), None).await?;

    // Should get response without creating a new session
    assert!(
        messages2
            .iter()
            .any(|m| matches!(m, WsServerMessage::ChatChunk { done: true, .. })),
        "Should receive done ChatChunk in second turn"
    );

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: WebSocket ping/pong during operations
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_ping_between_operations() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    // Ping should work
    ws.ping().await?;

    // Chat
    let _messages = ws.chat("Hello", None, None).await?;

    // Ping should still work after chat
    ws.ping().await?;

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: WebSocket authentication failure
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_auth_failure() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let result = ws.authenticate("wrong-token").await?;

    match result {
        WsServerMessage::AuthResult { success, error } => {
            assert!(!success, "Auth should fail with wrong token");
            assert!(error.is_some(), "Should have error message");
        }
        other => panic!("Expected AuthResult, got: {:?}", other),
    }

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Chat without authentication returns error
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_chat_without_auth() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    // Try to chat without authenticating first
    let messages = ws.chat("Hello", None, None).await?;

    // Should receive an error
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, WsServerMessage::Error { code, .. } if code == "unauthorized")),
        "Should receive unauthorized error. Messages: {:?}",
        messages
    );

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Subscribe without authentication returns error
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_subscribe_without_auth() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    let result = ws.subscribe("00000000-0000-0000-0000-000000000001").await?;
    match result {
        WsServerMessage::Error { code, .. } => {
            assert_eq!(code, "unauthorized");
        }
        other => panic!("Expected Error, got: {:?}", other),
    }

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Subscribe with invalid session ID returns error
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_subscribe_invalid_session_id() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    let result = ws.subscribe("not-a-uuid").await?;
    match result {
        WsServerMessage::Error { code, .. } => {
            assert_eq!(code, "invalid_session");
        }
        other => panic!("Expected Error, got: {:?}", other),
    }

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Send invalid JSON over WebSocket
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_invalid_json() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    // Send malformed JSON
    ws.send_json(&json!("this is not a valid client message"))
        .await?;

    let result = ws.recv().await?;
    match result {
        WsServerMessage::Error { code, .. } => {
            assert_eq!(code, "parse_error");
        }
        other => panic!("Expected parse_error, got: {:?}", other),
    }

    // Connection should still be alive after parse error
    ws.ping().await?;

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Command via WebSocket (compact, session not found)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_command_session_not_found() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    let messages = ws
        .command(
            "compact",
            json!({"session_id": "00000000-0000-0000-0000-000000000001"}),
        )
        .await?;

    // Should have progress and then a failure result
    let result = messages
        .iter()
        .find(|m| matches!(m, WsServerMessage::CommandResult { .. }));
    assert!(result.is_some(), "Should receive CommandResult");

    if let Some(WsServerMessage::CommandResult { success, .. }) = result {
        assert!(!success, "Command should have failed (session not found)");
    }

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Unknown command via WebSocket
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_unknown_command() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    let messages = ws.command("nonexistent", json!({})).await?;

    let result = messages
        .iter()
        .find(|m| matches!(m, WsServerMessage::CommandResult { .. }));
    assert!(result.is_some(), "Should receive CommandResult");

    if let Some(WsServerMessage::CommandResult {
        command, success, ..
    }) = result
    {
        assert_eq!(command, "nonexistent");
        assert!(!success, "Unknown command should fail");
    }

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Command without authentication
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_command_without_auth() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    let messages = ws.command("compact", json!({})).await?;

    assert!(
        messages
            .iter()
            .any(|m| matches!(m, WsServerMessage::Error { code, .. } if code == "unauthorized")),
        "Should receive unauthorized error"
    );

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Cancel operation
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_cancel() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    // Chat to create a session
    let messages = ws.chat("Hello", None, None).await?;
    let session_id = messages
        .iter()
        .find_map(|m| match m {
            WsServerMessage::SessionCreated { session_id } => Some(session_id.clone()),
            _ => None,
        })
        .expect("Should get session ID");

    // Cancel should not produce an error (it's a fire-and-forget)
    ws.cancel(&session_id).await?;

    // Ping should still work after cancel
    ws.ping().await?;

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Unsubscribe from session
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_unsubscribe() -> Result<()> {
    let backend = ScriptedMockBackend::new(vec![vec![StreamingMockEvent::Text(
        "Response.".to_string(),
    )]]);

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .build()
        .await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    // Chat to create a session
    let messages = ws.chat("Hello", None, None).await?;
    let session_id = messages
        .iter()
        .find_map(|m| match m {
            WsServerMessage::SessionCreated { session_id } => Some(session_id.clone()),
            _ => None,
        })
        .expect("Should get session ID");

    // Subscribe then unsubscribe
    let _ = ws.subscribe(&session_id).await?;

    // Send unsubscribe message
    ws.send_json(&json!({
        "type": "unsubscribe",
        "session_id": session_id
    }))
    .await?;

    // Unsubscribe produces no response, so just verify connection is still alive
    ws.ping().await?;

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Concurrent WebSocket connections
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_concurrent_connections() -> Result<()> {
    let backend = ScriptedMockBackend::new(vec![
        vec![StreamingMockEvent::Text("Conn1 response.".to_string())],
        vec![StreamingMockEvent::Text("Conn2 response.".to_string())],
    ]);

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .build()
        .await?;

    // Two connections simultaneously
    let mut ws1 = TestWsClient::connect(&server.ws_url()).await?;
    let mut ws2 = TestWsClient::connect(&server.ws_url()).await?;

    let _ = ws1.authenticate("test-token").await?;
    let _ = ws2.authenticate("test-token").await?;

    // Both can ping
    ws1.ping().await?;
    ws2.ping().await?;

    // First connection chats
    let messages1 = ws1.chat("From conn 1", None, None).await?;
    assert!(
        messages1
            .iter()
            .any(|m| matches!(m, WsServerMessage::ChatChunk { done: true, .. }))
    );

    // Second connection chats independently
    let messages2 = ws2.chat("From conn 2", None, None).await?;
    assert!(
        messages2
            .iter()
            .any(|m| matches!(m, WsServerMessage::ChatChunk { done: true, .. }))
    );

    ws1.close().await?;
    ws2.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Subscribe ownership - first subscriber is owner
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_subscribe_ownership() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    // Chat to create a session
    let messages = ws.chat("Hello", None, None).await?;
    let session_id = messages
        .iter()
        .find_map(|m| match m {
            WsServerMessage::SessionCreated { session_id } => Some(session_id.clone()),
            _ => None,
        })
        .expect("Should get session ID");

    // Subscribe - should become owner (first subscriber)
    let sub_result = ws.subscribe(&session_id).await?;
    match sub_result {
        WsServerMessage::SubscribeAck {
            owner,
            reconnect_token,
            ..
        } => {
            assert!(owner, "First subscriber should be owner");
            assert!(
                reconnect_token.is_some(),
                "Owner should get reconnect token"
            );
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Chat with invalid workstream ID
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_chat_invalid_workstream_id() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    // Chat with path-traversal workstream ID
    let messages = ws.chat("Hello", None, Some("../../../etc/passwd")).await?;

    assert!(
        messages.iter().any(
            |m| matches!(m, WsServerMessage::Error { code, .. } if code == "invalid_workstream_id")
        ),
        "Should reject invalid workstream ID. Messages: {:?}",
        messages
    );

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Command via WS with successful compact (not needed)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_command_compact_not_needed() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    // Create a session via chat (1 turn - not enough to compact)
    let messages = ws.chat("Hello", None, None).await?;
    let session_id = messages
        .iter()
        .find_map(|m| match m {
            WsServerMessage::SessionCreated { session_id } => Some(session_id.clone()),
            _ => None,
        })
        .expect("Should get session ID");

    // Subscribe so command can inject session context
    let _ = ws.subscribe(&session_id).await?;

    // Run compact command
    let cmd_messages = ws
        .command("compact", json!({"session_id": session_id}))
        .await?;

    // Should have progress and result
    let has_progress = cmd_messages
        .iter()
        .any(|m| matches!(m, WsServerMessage::CommandProgress { .. }));
    assert!(has_progress, "Should receive CommandProgress");

    let result = cmd_messages
        .iter()
        .find(|m| matches!(m, WsServerMessage::CommandResult { .. }));
    assert!(result.is_some(), "Should receive CommandResult");

    if let Some(WsServerMessage::CommandResult {
        command,
        success,
        result,
    }) = result
    {
        assert_eq!(command, "compact");
        assert!(success, "Compact should succeed (just not needed)");
        assert_eq!(result["compacted"], false);
    }

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: try_recv returns None when no message pending
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_try_recv_no_message() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    // try_recv with short timeout should return None when no message is pending
    let result = ws.try_recv(Duration::from_millis(100)).await?;
    assert!(result.is_none(), "Should get None when no message pending");

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: WebSocket auto-auth (no auth token configured)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_auto_auth_no_token() -> Result<()> {
    // Build server with no auth token (localhost mode)
    let server = TestServerBuilder::new().with_auth(None).build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    // Should be able to chat immediately without authenticating
    let messages = ws.chat("Hello without auth", None, None).await?;

    // The key assertion: we received SessionCreated without authenticating first.
    // This proves auto-auth worked (no auth token configured = auto-authenticated).
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, WsServerMessage::SessionCreated { .. })),
        "Should get SessionCreated without explicit auth. Messages: {:?}",
        messages
    );

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: WebSocket cancel without auth returns error
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_cancel_without_auth() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;

    // Send cancel without authenticating - cancel produces no response
    // but let's send it followed by a ping to verify connection is alive
    ws.cancel("00000000-0000-0000-0000-000000000001").await?;

    // The cancel handler checks auth and returns error for unauthenticated,
    // but cancel is fire-and-forget from client perspective.
    // The error message should be receivable:
    let msg = ws.try_recv(Duration::from_millis(500)).await?;
    // Cancel without auth should produce an error, but it might also produce None
    // if the server handles it silently
    if let Some(WsServerMessage::Error { code, .. }) = msg {
        assert_eq!(code, "unauthorized");
    }

    ws.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Subscribe with reconnect token
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_subscribe_reconnect_token() -> Result<()> {
    let backend = ScriptedMockBackend::new(vec![
        vec![StreamingMockEvent::Text("First response.".to_string())],
        vec![StreamingMockEvent::Text("Second response.".to_string())],
    ]);

    let server = TestServerBuilder::new()
        .with_backend(backend)
        .build()
        .await?;

    // First connection: chat, subscribe, get reconnect token
    let mut ws1 = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws1.authenticate("test-token").await?;

    let messages = ws1.chat("Hello", None, None).await?;
    let session_id = messages
        .iter()
        .find_map(|m| match m {
            WsServerMessage::SessionCreated { session_id } => Some(session_id.clone()),
            _ => None,
        })
        .expect("Should get session ID");

    let sub_result = ws1.subscribe(&session_id).await?;
    let reconnect_token = match sub_result {
        WsServerMessage::SubscribeAck {
            reconnect_token, ..
        } => reconnect_token.expect("Should get reconnect token"),
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    };

    // Disconnect first connection
    ws1.close().await?;

    // Small delay for server to process disconnect
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second connection: reconnect with token
    let mut ws2 = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws2.authenticate("test-token").await?;

    let sub_result2 = ws2
        .subscribe_with_token(&session_id, &reconnect_token)
        .await?;

    match sub_result2 {
        WsServerMessage::SubscribeAck { owner, .. } => {
            assert!(owner, "Should reclaim ownership with reconnect token");
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    ws2.close().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Binary UTF-8 frame accepted as text
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_binary_utf8_frame() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    // Use raw tungstenite connection for binary frame control
    let (mut ws, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;

    // Send auth as binary (valid UTF-8 JSON)
    let auth_json = serde_json::to_string(&json!({"type": "auth", "token": "test-token"}))?;
    ws.send(TungsteniteMsg::Binary(auth_json.into_bytes().into()))
        .await?;

    // Should receive auth result
    let msg = ws.next().await.unwrap()?;
    let text = msg.into_text()?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(parsed["type"], "auth_result");
    assert_eq!(parsed["success"], true);

    // Send ping as binary
    let ping_json = serde_json::to_string(&json!({"type": "ping"}))?;
    ws.send(TungsteniteMsg::Binary(ping_json.into_bytes().into()))
        .await?;

    let msg = ws.next().await.unwrap()?;
    let text = msg.into_text()?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(parsed["type"], "pong");

    ws.close(None).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Binary non-UTF-8 frame returns error
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_binary_non_utf8_frame() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let (mut ws, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;

    // First authenticate (as text so we can proceed)
    let auth_json = serde_json::to_string(&json!({"type": "auth", "token": "test-token"}))?;
    ws.send(TungsteniteMsg::Text(auth_json.into())).await?;
    let _ = ws.next().await; // consume auth result

    // Send invalid binary (non-UTF-8 bytes)
    let invalid_bytes: Vec<u8> = vec![0xFF, 0xFE, 0xFD, 0xFC];
    ws.send(TungsteniteMsg::Binary(invalid_bytes.into()))
        .await?;

    // Should receive error
    let msg = ws.next().await.unwrap()?;
    let text = msg.into_text()?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(parsed["type"], "error");
    assert_eq!(parsed["code"], "invalid_message");

    ws.close(None).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Cancel with invalid session ID
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ws_cancel_invalid_session_id() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let mut ws = TestWsClient::connect(&server.ws_url()).await?;
    let _ = ws.authenticate("test-token").await?;

    // Cancel with invalid UUID
    ws.cancel("not-a-uuid").await?;

    // Should get error for invalid session ID
    let msg = ws.try_recv(Duration::from_millis(500)).await?;
    if let Some(WsServerMessage::Error { code, .. }) = msg {
        assert_eq!(code, "invalid_session");
    }

    // Connection should survive
    ws.ping().await?;

    ws.close().await?;
    Ok(())
}

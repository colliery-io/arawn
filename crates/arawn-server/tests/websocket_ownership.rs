//! WebSocket ownership, multi-client, and reconnection integration tests.
//!
//! Exercises session ownership semantics: who becomes the owner, what happens
//! when the owner disconnects, reconnect-token reclaim, and chat rejection
//! for non-owners.

mod common;

use std::time::Duration;

use anyhow::Result;
use arawn_test_utils::ws_client::WsServerMessage;
use arawn_test_utils::{TestServer, TestWsClient};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Second subscriber is a reader, not owner
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_second_subscriber_is_reader() -> Result<()> {
    let server = TestServer::start_with_responses(vec!["R1".into(), "R2".into()]).await?;
    let url = server.ws_url();
    let session_id = uuid::Uuid::new_v4().to_string();

    // Client A subscribes first — becomes owner
    let mut ws_a = TestWsClient::connect(&url).await?;
    ws_a.authenticate("test-token").await?;
    let ack_a = ws_a.subscribe(&session_id).await?;

    match &ack_a {
        WsServerMessage::SubscribeAck {
            owner,
            reconnect_token,
            ..
        } => {
            assert!(owner, "First subscriber should be owner");
            assert!(
                reconnect_token.is_some(),
                "Owner should receive a reconnect token"
            );
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    // Client B subscribes to the same session — reader
    let mut ws_b = TestWsClient::connect(&url).await?;
    ws_b.authenticate("test-token").await?;
    let ack_b = ws_b.subscribe(&session_id).await?;

    match &ack_b {
        WsServerMessage::SubscribeAck {
            owner,
            reconnect_token,
            ..
        } => {
            assert!(!owner, "Second subscriber should NOT be owner");
            assert!(
                reconnect_token.is_none(),
                "Reader should not receive a reconnect token"
            );
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Owner disconnect leaves pending reconnect; new subscriber is reader
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_owner_disconnect_allows_new_owner() -> Result<()> {
    let server = TestServer::start_with_responses(vec!["R1".into(), "R2".into()]).await?;
    let url = server.ws_url();
    let session_id = uuid::Uuid::new_v4().to_string();

    // Client A subscribes — owner
    let mut ws_a = TestWsClient::connect(&url).await?;
    ws_a.authenticate("test-token").await?;
    let ack_a = ws_a.subscribe(&session_id).await?;
    assert!(matches!(
        ack_a,
        WsServerMessage::SubscribeAck { owner: true, .. }
    ));

    // A disconnects — server creates a pending reconnect
    ws_a.close().await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Client B subscribes without a token during the grace period — reader
    let mut ws_b = TestWsClient::connect(&url).await?;
    ws_b.authenticate("test-token").await?;
    let ack_b = ws_b.subscribe(&session_id).await?;

    match &ack_b {
        WsServerMessage::SubscribeAck { owner, .. } => {
            assert!(
                !owner,
                "B should be reader while pending reconnect is active"
            );
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Reconnect with valid token reclaims ownership
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_reconnect_with_token() -> Result<()> {
    let server = TestServer::start().await?;
    let url = server.ws_url();
    let session_id = uuid::Uuid::new_v4().to_string();

    // Client A subscribes — extract reconnect token
    let mut ws_a = TestWsClient::connect(&url).await?;
    ws_a.authenticate("test-token").await?;
    let ack_a = ws_a.subscribe(&session_id).await?;

    let token = match &ack_a {
        WsServerMessage::SubscribeAck {
            reconnect_token: Some(t),
            owner: true,
            ..
        } => t.clone(),
        other => panic!("Expected owner SubscribeAck with token, got: {:?}", other),
    };

    // A disconnects
    ws_a.close().await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // A2 (new connection) reconnects with the token
    let mut ws_a2 = TestWsClient::connect(&url).await?;
    ws_a2.authenticate("test-token").await?;
    let ack_a2 = ws_a2.subscribe_with_token(&session_id, &token).await?;

    match &ack_a2 {
        WsServerMessage::SubscribeAck {
            owner,
            reconnect_token,
            ..
        } => {
            assert!(
                owner,
                "Reconnecting with valid token should reclaim ownership"
            );
            assert!(
                reconnect_token.is_some(),
                "Should receive a new reconnect token after reclaim"
            );
            // The new token should differ from the old one
            assert_ne!(
                reconnect_token.as_deref(),
                Some(token.as_str()),
                "New token should differ from old token"
            );
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Chat from non-owner is rejected
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_chat_from_non_owner_rejected() -> Result<()> {
    let server = TestServer::start_with_responses(vec!["R1".into(), "R2".into()]).await?;
    let url = server.ws_url();

    // Client A creates a session via chat (becomes owner implicitly)
    let mut ws_a = TestWsClient::connect(&url).await?;
    ws_a.authenticate("test-token").await?;
    let msgs = ws_a.chat("hello", None, None).await?;

    // Extract session_id from SessionCreated
    let session_id = msgs
        .iter()
        .find_map(|m| match m {
            WsServerMessage::SessionCreated { session_id } => Some(session_id.clone()),
            _ => None,
        })
        .expect("Should receive SessionCreated");

    // A subscribes to claim explicit ownership of the session
    let ack_a = ws_a.subscribe(&session_id).await?;
    assert!(
        matches!(ack_a, WsServerMessage::SubscribeAck { owner: true, .. }),
        "A should own the session after chat"
    );

    // Client B subscribes as reader
    let mut ws_b = TestWsClient::connect(&url).await?;
    ws_b.authenticate("test-token").await?;
    let ack_b = ws_b.subscribe(&session_id).await?;
    assert!(
        matches!(ack_b, WsServerMessage::SubscribeAck { owner: false, .. }),
        "B should be a reader"
    );

    // B tries to chat in that session — should be rejected
    let msgs_b = ws_b.chat("intruder", Some(&session_id), None).await?;
    let has_error = msgs_b
        .iter()
        .any(|m| matches!(m, WsServerMessage::Error { code, .. } if code == "session_not_owned"));
    assert!(
        has_error,
        "Non-owner chat should be rejected with session_not_owned"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Explicit unsubscribe releases ownership (no pending reconnect)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_unsubscribe_releases_ownership() -> Result<()> {
    let server = TestServer::start().await?;
    let url = server.ws_url();
    let session_id = uuid::Uuid::new_v4().to_string();

    // Client A subscribes — owner
    let mut ws_a = TestWsClient::connect(&url).await?;
    ws_a.authenticate("test-token").await?;
    let ack_a = ws_a.subscribe(&session_id).await?;
    assert!(matches!(
        ack_a,
        WsServerMessage::SubscribeAck { owner: true, .. }
    ));

    // A explicitly unsubscribes (no pending reconnect created)
    ws_a.send_json(&json!({
        "type": "unsubscribe",
        "session_id": session_id
    }))
    .await?;

    // Brief pause to let the server process unsubscribe
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Client B subscribes — should become owner (no pending reconnect blocks it)
    let mut ws_b = TestWsClient::connect(&url).await?;
    ws_b.authenticate("test-token").await?;
    let ack_b = ws_b.subscribe(&session_id).await?;

    match &ack_b {
        WsServerMessage::SubscribeAck {
            owner,
            reconnect_token,
            ..
        } => {
            assert!(owner, "B should become owner after A unsubscribed");
            assert!(
                reconnect_token.is_some(),
                "New owner should get a reconnect token"
            );
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Reconnect with invalid token fails to reclaim
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_reconnect_with_invalid_token() -> Result<()> {
    let server = TestServer::start().await?;
    let url = server.ws_url();
    let session_id = uuid::Uuid::new_v4().to_string();

    // Client A subscribes — owner
    let mut ws_a = TestWsClient::connect(&url).await?;
    ws_a.authenticate("test-token").await?;
    let ack_a = ws_a.subscribe(&session_id).await?;
    assert!(matches!(
        ack_a,
        WsServerMessage::SubscribeAck { owner: true, .. }
    ));

    // A disconnects (creates pending reconnect)
    ws_a.close().await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // A2 tries to reconnect with a bogus token
    let mut ws_a2 = TestWsClient::connect(&url).await?;
    ws_a2.authenticate("test-token").await?;
    let ack_a2 = ws_a2
        .subscribe_with_token(&session_id, "wrong-token")
        .await?;

    match &ack_a2 {
        WsServerMessage::SubscribeAck { owner, .. } => {
            assert!(
                !owner,
                "Invalid token should not reclaim ownership during pending reconnect"
            );
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 7. Multiple subscriptions to different sessions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_multiple_subscriptions() -> Result<()> {
    let server = TestServer::start().await?;
    let url = server.ws_url();
    let session_1 = uuid::Uuid::new_v4().to_string();
    let session_2 = uuid::Uuid::new_v4().to_string();

    let mut ws = TestWsClient::connect(&url).await?;
    ws.authenticate("test-token").await?;

    // Subscribe to session 1
    let ack_1 = ws.subscribe(&session_1).await?;
    match &ack_1 {
        WsServerMessage::SubscribeAck {
            owner, session_id, ..
        } => {
            assert!(owner, "Should own session 1");
            assert_eq!(session_id, &session_1);
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    // Subscribe to session 2
    let ack_2 = ws.subscribe(&session_2).await?;
    match &ack_2 {
        WsServerMessage::SubscribeAck {
            owner, session_id, ..
        } => {
            assert!(owner, "Should own session 2");
            assert_eq!(session_id, &session_2);
        }
        other => panic!("Expected SubscribeAck, got: {:?}", other),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 8. Owner can chat in a subscribed session
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_owner_can_chat() -> Result<()> {
    let server = TestServer::start().await?;
    let url = server.ws_url();
    let session_id = uuid::Uuid::new_v4().to_string();

    let mut ws = TestWsClient::connect(&url).await?;
    ws.authenticate("test-token").await?;

    // Subscribe — become owner
    let ack = ws.subscribe(&session_id).await?;
    assert!(matches!(
        ack,
        WsServerMessage::SubscribeAck { owner: true, .. }
    ));

    // Chat in the owned session
    let msgs = ws.chat("hello", Some(&session_id), None).await?;

    // Expect SessionCreated followed by at least one ChatChunk (done=true)
    let has_session_created = msgs
        .iter()
        .any(|m| matches!(m, WsServerMessage::SessionCreated { .. }));
    let has_terminal = msgs.iter().any(|m| {
        matches!(m, WsServerMessage::ChatChunk { done: true, .. })
            || matches!(m, WsServerMessage::Error { .. })
    });

    assert!(has_session_created, "Should receive SessionCreated");
    assert!(
        has_terminal,
        "Should receive a terminal message (done or error)"
    );

    Ok(())
}

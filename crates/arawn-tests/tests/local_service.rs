//! Tests for LocalService — the ArawnService impl that wraps engine + store.

use std::sync::Arc;

use futures_util::StreamExt;
use tempfile::TempDir;

use arawn_core::Workstream;
use arawn_engine::{QueryEngineConfig, ThinkTool, ToolRegistry};
use arawn_llm::{MockLlmClient, MockResponse};
use arawn_service::{ArawnService, EngineEvent};
use arawn_storage::{Store, JsonlMessageStore};

fn setup_service(responses: Vec<MockResponse>) -> (TempDir, arawn_bin::LocalService) {
    let tmp = TempDir::new().unwrap();
    let store = Store::open(tmp.path()).unwrap();

    // Create scratch workstream
    let ws = Workstream::scratch(tmp.path());
    store.create_workstream(&ws).unwrap();

    let llm = Arc::new(MockLlmClient::new(responses));
    let registry = Arc::new(ToolRegistry::new());
    registry.register(Box::new(ThinkTool));

    let config = QueryEngineConfig {
        system_prompt: "Test".into(),
        ..Default::default()
    };

    let service =
        arawn_bin::LocalService::new(store, tmp.path().to_path_buf(), llm, registry, config);

    (tmp, service)
}

#[tokio::test]
async fn list_workstreams_returns_scratch() {
    let (_tmp, service) = setup_service(vec![]);
    let workstreams = service.list_workstreams().await.unwrap();
    assert!(!workstreams.is_empty());
    assert_eq!(workstreams[0].name, "scratch");
}

#[tokio::test]
async fn create_and_load_session_roundtrip() {
    let (_tmp, service) = setup_service(vec![]);

    let workstreams = service.list_workstreams().await.unwrap();
    let ws_id = workstreams[0].id;

    let session = service.create_session(Some(ws_id)).await.unwrap();
    assert_eq!(session.workstream_id, Some(ws_id));

    let loaded = service.load_session(session.id).await.unwrap();
    assert_eq!(loaded.id, session.id);
    assert!(loaded.messages.is_empty());
}

#[tokio::test]
async fn send_message_text_only_returns_complete() {
    let (_tmp, service) = setup_service(vec![MockResponse::text("Hello back!")]);

    let session = service.create_session(None).await.unwrap();

    let mut stream = service.send_message(session.id, "Hi".into()).await.unwrap();

    let mut got_complete = false;
    let mut final_text = String::new();

    while let Some(event) = stream.next().await {
        match event {
            EngineEvent::Complete { final_text: ft } => {
                got_complete = true;
                final_text = ft;
            }
            _ => {}
        }
    }

    assert!(got_complete, "should have received Complete event");
    assert_eq!(final_text, "Hello back!");
}

#[tokio::test]
async fn send_message_with_tool_call_returns_events() {
    let (_tmp, service) = setup_service(vec![
        MockResponse::tool_call("c1", "think", r#"{"thought":"reasoning"}"#),
        MockResponse::text("Done thinking."),
    ]);

    let session = service.create_session(None).await.unwrap();

    let mut stream = service
        .send_message(session.id, "Think about this".into())
        .await
        .unwrap();

    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event);
    }

    // Should have tool events + complete
    let has_tool_start = events
        .iter()
        .any(|e| matches!(e, EngineEvent::ToolCallStart { .. }));
    let has_tool_result = events
        .iter()
        .any(|e| matches!(e, EngineEvent::ToolCallResult { .. }));
    let has_complete = events
        .iter()
        .any(|e| matches!(e, EngineEvent::Complete { .. }));

    assert!(has_tool_start, "should have ToolCallStart event");
    assert!(has_tool_result, "should have ToolCallResult event");
    assert!(has_complete, "should have Complete event");
}

#[tokio::test]
async fn send_message_persists_to_jsonl() {
    let (_tmp, service) = setup_service(vec![MockResponse::text("Persisted reply")]);

    let workstreams = service.list_workstreams().await.unwrap();
    let ws_id = workstreams[0].id;
    let session = service.create_session(Some(ws_id)).await.unwrap();

    let mut stream = service
        .send_message(session.id, "Save this".into())
        .await
        .unwrap();

    // Drain stream
    while let Some(_) = stream.next().await {}

    // Load session — should have messages
    let loaded = service.load_session(session.id).await.unwrap();
    assert!(
        loaded.messages.len() >= 2,
        "should have at least user + assistant messages, got {}",
        loaded.messages.len()
    );
}

#[tokio::test]
async fn create_workstream_with_default_root_dir() {
    let (tmp, service) = setup_service(vec![]);

    let ws = service
        .create_workstream("test-project".into(), tmp.path().join("workstreams/test-project"))
        .await
        .unwrap();

    assert_eq!(ws.name, "test-project");

    // Directory should exist
    let ws_dir = tmp.path().join("workstreams/test-project");
    assert!(ws_dir.exists(), "workstream directory should be created");

    // Should appear in list
    let all = service.list_workstreams().await.unwrap();
    assert!(
        all.iter().any(|w| w.name == "test-project"),
        "new workstream should appear in list"
    );
}

#[tokio::test]
async fn promote_scratch_session_to_workstream() {
    let (tmp, service) = setup_service(vec![MockResponse::text("Reply in scratch")]);

    // Create a target workstream
    service
        .create_workstream("finances".into(), tmp.path().join("workstreams/finances"))
        .await
        .unwrap();

    // Create a scratch session and send a message
    let session = service.create_session(None).await.unwrap();
    let mut stream = service
        .send_message(session.id, "Track my expenses".into())
        .await
        .unwrap();
    while let Some(_) = stream.next().await {}

    // Verify message is in scratch
    let loaded_before = service.load_session(session.id).await.unwrap();
    assert!(
        loaded_before.messages.len() >= 2,
        "scratch session should have messages"
    );

    // Promote to finances workstream
    let result = service.promote_session(session.id, "finances").await.unwrap();
    assert_eq!(
        result.get("status").and_then(|s| s.as_str()),
        Some("promoted")
    );

    // Session should still load with its messages from the new location
    let loaded_after = service.load_session(session.id).await.unwrap();
    assert_eq!(
        loaded_before.messages.len(),
        loaded_after.messages.len(),
        "messages should survive promotion"
    );

    // JSONL should exist in the workstream dir, not scratch
    let ws_dir = arawn_storage::workstream_dir_name("finances",
        result.get("workstream_id").and_then(|s| s.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .unwrap()
    );
    let msg_store = JsonlMessageStore::new(tmp.path());
    let messages = msg_store.load(session.id, &ws_dir).await.unwrap();
    assert!(
        !messages.is_empty(),
        "JSONL should exist in workstream dir after promotion"
    );
}

#[tokio::test]
async fn promote_non_scratch_session_fails() {
    let (tmp, service) = setup_service(vec![]);

    // Create a workstream and a session bound to it
    let ws = service
        .create_workstream("project-a".into(), tmp.path().join("workstreams/project-a"))
        .await
        .unwrap();
    let session = service.create_session(Some(ws.id)).await.unwrap();

    // Create another workstream
    service
        .create_workstream("project-b".into(), tmp.path().join("workstreams/project-b"))
        .await
        .unwrap();

    // Promoting a non-scratch session should fail
    let result = service.promote_session(session.id, "project-b").await;
    assert!(result.is_err(), "promoting a non-scratch session should fail");
}

#[tokio::test]
async fn multi_turn_conversation_accumulates_history() {
    let (_tmp, service) = setup_service(vec![
        MockResponse::text("First reply"),
        MockResponse::text("Second reply"),
    ]);

    let session = service.create_session(None).await.unwrap();

    // First turn
    let mut stream = service
        .send_message(session.id, "Turn 1".into())
        .await
        .unwrap();
    while let Some(_) = stream.next().await {}

    // Second turn
    let mut stream = service
        .send_message(session.id, "Turn 2".into())
        .await
        .unwrap();
    while let Some(_) = stream.next().await {}

    // Load session — should have 4 messages: user1, assistant1, user2, assistant2
    let loaded = service.load_session(session.id).await.unwrap();
    assert!(
        loaded.messages.len() >= 4,
        "expected at least 4 messages for 2 turns, got {}",
        loaded.messages.len()
    );
}

#[tokio::test]
async fn list_sessions_returns_multiple() {
    let (_tmp, service) = setup_service(vec![]);

    let workstreams = service.list_workstreams().await.unwrap();
    let ws_id = workstreams[0].id;

    // Create 2 sessions
    let s1 = service.create_session(Some(ws_id)).await.unwrap();
    let s2 = service.create_session(Some(ws_id)).await.unwrap();

    let sessions = service.list_sessions(Some(ws_id)).await.unwrap();
    assert!(
        sessions.len() >= 2,
        "expected at least 2 sessions, got {}",
        sessions.len()
    );

    let ids: Vec<_> = sessions.iter().map(|s| s.id).collect();
    assert!(ids.contains(&s1.id), "should contain first session");
    assert!(ids.contains(&s2.id), "should contain second session");
}

#[tokio::test]
async fn engine_error_produces_error_event() {
    // Use MockResponse::error to simulate LLM failure
    let (_tmp, service) = setup_service(vec![MockResponse::error(
        arawn_llm::LlmError::Auth("invalid key".into()),
    )]);

    let session = service.create_session(None).await.unwrap();

    let mut stream = service
        .send_message(session.id, "trigger error".into())
        .await
        .unwrap();

    let mut got_error = false;
    while let Some(event) = stream.next().await {
        if matches!(event, EngineEvent::Error { .. }) {
            got_error = true;
        }
    }

    assert!(got_error, "should have received Error event for LLM failure");
}

#[tokio::test]
async fn multi_turn_with_tool_calls_accumulates_full_history() {
    let (_tmp, service) = setup_service(vec![
        // Turn 1: tool call + text
        MockResponse::tool_call("c1", "think", r#"{"thought":"turn 1 analysis"}"#),
        MockResponse::text("Turn 1 complete."),
        // Turn 2: tool call + text
        MockResponse::tool_call("c2", "think", r#"{"thought":"turn 2 analysis"}"#),
        MockResponse::text("Turn 2 complete."),
    ]);

    let session = service.create_session(None).await.unwrap();

    // Turn 1
    let mut stream = service
        .send_message(session.id, "First question".into())
        .await
        .unwrap();
    while let Some(_) = stream.next().await {}

    // Turn 2
    let mut stream = service
        .send_message(session.id, "Follow-up question".into())
        .await
        .unwrap();
    while let Some(_) = stream.next().await {}

    // Load and verify full history
    let loaded = service.load_session(session.id).await.unwrap();
    // Per turn: user + assistant(tool) + tool_result + assistant(text) = 4
    // Two turns = 8 messages minimum
    assert!(
        loaded.messages.len() >= 8,
        "expected 8+ messages for 2 tool-call turns, got {}",
        loaded.messages.len()
    );
}

#[tokio::test]
async fn session_isolation_separate_histories() {
    let (_tmp, service) = setup_service(vec![
        MockResponse::text("Reply to session A"),
        MockResponse::text("Reply to session B"),
    ]);

    let s_a = service.create_session(None).await.unwrap();
    let s_b = service.create_session(None).await.unwrap();

    // Send to A
    let mut stream = service
        .send_message(s_a.id, "Message for A".into())
        .await
        .unwrap();
    while let Some(_) = stream.next().await {}

    // Send to B
    let mut stream = service
        .send_message(s_b.id, "Message for B".into())
        .await
        .unwrap();
    while let Some(_) = stream.next().await {}

    // Verify isolation
    let loaded_a = service.load_session(s_a.id).await.unwrap();
    let loaded_b = service.load_session(s_b.id).await.unwrap();

    // A should not contain B's content and vice versa
    let a_text: String = loaded_a
        .messages
        .iter()
        .filter_map(|m| match m {
            arawn_core::Message::Assistant { content, .. } => Some(content.clone()),
            _ => None,
        })
        .collect();
    let b_text: String = loaded_b
        .messages
        .iter()
        .filter_map(|m| match m {
            arawn_core::Message::Assistant { content, .. } => Some(content.clone()),
            _ => None,
        })
        .collect();

    assert!(
        a_text.contains("session A"),
        "session A should have A's reply"
    );
    assert!(
        b_text.contains("session B"),
        "session B should have B's reply"
    );
    assert!(
        !a_text.contains("session B"),
        "session A should NOT have B's reply"
    );
    assert!(
        !b_text.contains("session A"),
        "session B should NOT have A's reply"
    );
}

#[tokio::test]
async fn large_conversation_five_turns_persisted() {
    let (_tmp, service) = setup_service(vec![
        MockResponse::text("Reply 1"),
        MockResponse::text("Reply 2"),
        MockResponse::text("Reply 3"),
        MockResponse::text("Reply 4"),
        MockResponse::text("Reply 5"),
    ]);

    let session = service.create_session(None).await.unwrap();

    for i in 1..=5 {
        let mut stream = service
            .send_message(session.id, format!("Turn {i}"))
            .await
            .unwrap();
        while let Some(_) = stream.next().await {}
    }

    let loaded = service.load_session(session.id).await.unwrap();
    // 5 turns * 2 messages (user + assistant) = 10
    assert!(
        loaded.messages.len() >= 10,
        "expected 10+ messages for 5 turns, got {}",
        loaded.messages.len()
    );
}

#[tokio::test]
async fn error_after_successful_first_turn_preserves_history() {
    let (_tmp, service) = setup_service(vec![
        MockResponse::text("First turn succeeded."),
        MockResponse::error(arawn_llm::LlmError::Auth("expired token".into())),
    ]);

    let session = service.create_session(None).await.unwrap();

    // Turn 1: success
    let mut stream = service
        .send_message(session.id, "First turn".into())
        .await
        .unwrap();
    let mut first_text = String::new();
    while let Some(event) = stream.next().await {
        if let EngineEvent::Complete { final_text } = event {
            first_text = final_text;
        }
    }
    assert_eq!(first_text, "First turn succeeded.");

    // Turn 2: LLM error
    let mut stream = service
        .send_message(session.id, "Second turn".into())
        .await
        .unwrap();
    let mut got_error = false;
    while let Some(event) = stream.next().await {
        if matches!(event, EngineEvent::Error { .. }) {
            got_error = true;
        }
    }
    assert!(got_error, "second turn should produce error event");

    // First turn's messages should still be persisted
    let loaded = service.load_session(session.id).await.unwrap();
    assert!(
        loaded.messages.len() >= 2,
        "first turn messages should survive second turn's error, got {}",
        loaded.messages.len()
    );
}

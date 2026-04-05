//! Tests for LocalService — the ArawnService impl that wraps engine + store.

use std::sync::Arc;

use futures_util::StreamExt;
use tempfile::TempDir;

use arawn_core::Workstream;
use arawn_engine::{QueryEngineConfig, ThinkTool, ToolRegistry};
use arawn_llm::{MockLlmClient, MockResponse};
use arawn_service::{ArawnService, EngineEvent};
use arawn_storage::Store;

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

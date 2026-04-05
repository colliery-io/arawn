//! Functional tests for context compaction.
//! Tests the full stack: compaction detection, LLM summarization, session mutation, persistence.

use std::sync::Arc;

use tempfile::TempDir;

use arawn_core::{Message, Session, Workstream};
use arawn_engine::{
    Compactor, ModelLimits, QueryEngine, QueryEngineConfig, ToolContext, ToolRegistry,
};
use arawn_llm::{MockLlmClient, MockResponse};
use arawn_storage::Store;

// --- Engine integration tests ---

#[tokio::test]
async fn engine_with_compactor_compacts_when_over_threshold() {
    // Create a session with many large messages that exceed a tiny context window
    let ws = Workstream::new("test", "/tmp/compact-test");
    let mut session = Session::new(ws.id);
    let ctx = ToolContext::new(&ws, session.id);

    let filler = "x".repeat(1000);
    for i in 0..20 {
        session.add_message(Message::User {
            content: format!("msg {i}: {filler}"),
        });
        session.add_message(Message::Assistant {
            content: format!("reply {i}: {filler}"),
            tool_uses: vec![],
        });
    }

    // Compaction LLM returns a summary, then the main LLM returns a text response
    let compaction_response = MockResponse::text(
        "<analysis>Analyzing conversation</analysis>\n<summary>\n1. Primary Request: User sent many messages.\n</summary>",
    );
    let main_response = MockResponse::text("Final answer after compaction.");

    let mock = Arc::new(MockLlmClient::new(vec![compaction_response, main_response]));
    let registry = Arc::new(ToolRegistry::new());

    let compactor = Compactor::with_keep_recent(mock.clone(), "test-model", 4);
    let config = QueryEngineConfig {
        model_limits: ModelLimits::new(500, 0.85), // tiny window forces compaction
        system_prompt: "Test".into(),
        ..Default::default()
    };

    let mut engine = QueryEngine::with_config(mock, registry, config).with_compactor(compactor);

    // Add one more user message to trigger the engine
    session.add_message(Message::User {
        content: "One more question".into(),
    });

    let result = engine.run(&mut session, &ctx).await.unwrap();
    assert_eq!(result, "Final answer after compaction.");

    // Session should have been compacted — first message should be Summary
    assert!(
        matches!(session.messages()[0], Message::Summary { .. }),
        "expected Summary as first message after compaction"
    );

    // Total message count should be much less than original 41
    assert!(
        session.messages().len() < 20,
        "expected fewer messages after compaction, got {}",
        session.messages().len()
    );
}

#[tokio::test]
async fn engine_without_compactor_no_compaction() {
    let ws = Workstream::new("test", "/tmp/no-compact");
    let mut session = Session::new(ws.id);
    let ctx = ToolContext::new(&ws, session.id);

    session.add_message(Message::User {
        content: "hello".into(),
    });

    let mock = Arc::new(MockLlmClient::new(vec![MockResponse::text("hi")]));
    let registry = Arc::new(ToolRegistry::new());
    let mut engine = QueryEngine::new(mock, registry); // No compactor

    let result = engine.run(&mut session, &ctx).await.unwrap();
    assert_eq!(result, "hi");
    assert_eq!(session.messages().len(), 2); // user + assistant, no Summary
}

#[tokio::test]
async fn engine_under_threshold_no_compaction() {
    let ws = Workstream::new("test", "/tmp/under-threshold");
    let mut session = Session::new(ws.id);
    let ctx = ToolContext::new(&ws, session.id);

    session.add_message(Message::User {
        content: "short message".into(),
    });

    let mock = Arc::new(MockLlmClient::new(vec![MockResponse::text("short reply")]));
    let registry = Arc::new(ToolRegistry::new());
    let compactor = Compactor::new(mock.clone(), "test-model");
    let config = QueryEngineConfig {
        model_limits: ModelLimits::new(1_000_000, 0.85), // huge window
        system_prompt: "Test".into(),
        ..Default::default()
    };

    let mut engine = QueryEngine::with_config(mock, registry, config).with_compactor(compactor);

    let result = engine.run(&mut session, &ctx).await.unwrap();
    assert_eq!(result, "short reply");
    // No compaction should have occurred
    assert!(
        !matches!(session.messages().first(), Some(Message::Summary { .. })),
        "should not have compacted under threshold"
    );
}

// --- Persistence integration tests ---

#[tokio::test]
async fn persistence_summary_survives_save_and_load() {
    let tmp = TempDir::new().unwrap();
    let store = Store::open(tmp.path()).unwrap();
    let ws = Workstream::new("ws", tmp.path().join("workspace"));
    store.create_workstream(&ws).unwrap();
    std::fs::create_dir_all(&ws.root_dir).unwrap();

    let session = Session::new(ws.id);
    store.create_session(&session).unwrap();

    // Append some messages then a Summary
    store
        .append_message(
            session.id,
            "ws",
            &Message::User {
                content: "old message 1".into(),
            },
        )
        .await
        .unwrap();
    store
        .append_message(
            session.id,
            "ws",
            &Message::User {
                content: "old message 2".into(),
            },
        )
        .await
        .unwrap();
    store
        .append_message(
            session.id,
            "ws",
            &Message::Summary {
                content: "Summary of old messages".into(),
                original_count: 2,
                estimated_tokens_saved: 50,
            },
        )
        .await
        .unwrap();
    store
        .append_message(
            session.id,
            "ws",
            &Message::User {
                content: "new message after compaction".into(),
            },
        )
        .await
        .unwrap();

    // Load session — should use compacted view
    let loaded = store.load_session(session.id).await.unwrap().unwrap();
    let msgs = loaded.messages();

    // Should start with Summary, then the new message — old messages skipped
    assert_eq!(msgs.len(), 2);
    assert!(
        matches!(&msgs[0], Message::Summary { content, .. } if content.contains("Summary of old"))
    );
    assert!(matches!(&msgs[1], Message::User { content } if content.contains("new message")));
}

#[tokio::test]
async fn persistence_no_summary_loads_all() {
    let tmp = TempDir::new().unwrap();
    let store = Store::open(tmp.path()).unwrap();
    let ws = Workstream::new("ws", tmp.path().join("workspace"));
    store.create_workstream(&ws).unwrap();
    std::fs::create_dir_all(&ws.root_dir).unwrap();

    let session = Session::new(ws.id);
    store.create_session(&session).unwrap();

    store
        .append_message(
            session.id,
            "ws",
            &Message::User {
                content: "msg 1".into(),
            },
        )
        .await
        .unwrap();
    store
        .append_message(
            session.id,
            "ws",
            &Message::User {
                content: "msg 2".into(),
            },
        )
        .await
        .unwrap();

    let loaded = store.load_session(session.id).await.unwrap().unwrap();
    assert_eq!(loaded.messages().len(), 2); // No Summary, all messages loaded
}

#[tokio::test]
async fn persistence_resume_after_compaction() {
    let tmp = TempDir::new().unwrap();
    let store = Store::open(tmp.path()).unwrap();
    let ws = Workstream::new("ws", tmp.path().join("workspace"));
    store.create_workstream(&ws).unwrap();
    std::fs::create_dir_all(&ws.root_dir).unwrap();

    let session = Session::new(ws.id);
    store.create_session(&session).unwrap();

    // Simulate a compacted session
    for i in 0..5 {
        store
            .append_message(
                session.id,
                "ws",
                &Message::User {
                    content: format!("old {i}"),
                },
            )
            .await
            .unwrap();
    }
    store
        .append_message(
            session.id,
            "ws",
            &Message::Summary {
                content: "Summary: user asked 5 questions".into(),
                original_count: 5,
                estimated_tokens_saved: 100,
            },
        )
        .await
        .unwrap();
    store
        .append_message(
            session.id,
            "ws",
            &Message::User {
                content: "follow-up question".into(),
            },
        )
        .await
        .unwrap();

    // Resume: load and run engine
    let mut loaded = store.load_session(session.id).await.unwrap().unwrap();
    assert_eq!(loaded.messages().len(), 2); // Summary + follow-up

    let mock = Arc::new(MockLlmClient::new(vec![MockResponse::text(
        "Answer to follow-up",
    )]));
    let registry = Arc::new(ToolRegistry::new());
    let mut engine = QueryEngine::new(mock, registry);
    let ctx = ToolContext::new(&ws, loaded.id);

    let result = engine.run(&mut loaded, &ctx).await.unwrap();
    assert_eq!(result, "Answer to follow-up");
    assert_eq!(loaded.messages().len(), 3); // Summary + follow-up + answer
}

//! Integration tests over the cross-crate seams the per-crate unit tests
//! don't reach (ARAWN-T-0483, I-0068 P2-8). These pin the data-integrity
//! behaviour the rest of Phase 2 relies on: session lifecycle incl.
//! promotion, JSONL corruption tolerance, and stream interruption mid
//! tool-call. (Atomicity-under-injected-failure for promotion/lens creation
//! is pinned by the storage-layer tests in `arawn-storage::store`; the
//! subsystem-absent startup path by the `status_*` tests in
//! `local_service.rs`.)

use std::sync::Arc;

use futures_util::StreamExt;
use tempfile::TempDir;

use arawn_core::{Lens, Message};
use arawn_engine::{QueryEngineConfig, ThinkTool, ToolRegistry};
use arawn_llm::{ChatChunk, LlmError, MockLlmClient, MockResponse};
use arawn_service::{ArawnService, EngineEvent};
use arawn_storage::Store;

fn setup_service(responses: Vec<MockResponse>) -> (TempDir, arawn_bin::LocalService) {
    let tmp = TempDir::new().unwrap();
    let store = Store::open(tmp.path()).unwrap();
    store.create_lens(&Lens::scratch(tmp.path())).unwrap();

    let llm: Arc<dyn arawn_llm::LlmClient> = Arc::new(MockLlmClient::new(responses));
    let registry = Arc::new(ToolRegistry::new());
    registry.register(Box::new(ThinkTool));
    let config = QueryEngineConfig {
        system_prompt: "Test".into(),
        ..Default::default()
    };
    let pool = Arc::new(arawn_bin::LlmClientPool::single(llm, config.model.clone()));
    let service =
        arawn_bin::LocalService::new(store, tmp.path().to_path_buf(), pool, registry, config);
    (tmp, service)
}

/// Seam: the full session lifecycle end-to-end — create scratch → send a
/// message (writes JSONL) → load → promote into a lens → load from the lens
/// with history intact.
#[tokio::test]
async fn session_lifecycle_create_append_load_promote() {
    let (tmp, service) = setup_service(vec![MockResponse::text("noted")]);

    let session = service.create_session(None).await.unwrap();
    let mut stream = service
        .send_message(session.id, "remember this".into())
        .await
        .unwrap();
    while stream.next().await.is_some() {}

    let loaded = service.load_session(session.id).await.unwrap();
    assert!(loaded.lens_id.is_none(), "starts in scratch");
    assert!(loaded.messages.len() >= 2, "user + assistant persisted");

    let lens = service
        .create_lens("keep".into(), tmp.path().join("lenses/keep"))
        .await
        .unwrap();
    service.promote_session(session.id, lens.id).await.unwrap();

    let after = service.load_session(session.id).await.unwrap();
    assert_eq!(after.lens_id, Some(lens.id), "now bound to the lens");
    assert!(
        after.messages.len() >= 2,
        "history survives promotion, got {}",
        after.messages.len()
    );
}

/// Seam: a malformed line in a session's JSONL is skipped with a warning —
/// the valid messages still load instead of the whole session erroring out.
#[tokio::test]
async fn jsonl_corruption_skips_bad_line_and_loads_rest() {
    let (tmp, service) = setup_service(vec![]);
    let session = service.create_session(None).await.unwrap();

    // Write the session file by hand: one valid message line, one garbage
    // line, another valid line.
    let dir = tmp
        .path()
        .join("lenses/scratch")
        .join(session.id.to_string());
    std::fs::create_dir_all(&dir).unwrap();
    let valid1 = serde_json::to_string(&Message::User {
        content: "first".into(),
    })
    .unwrap();
    let valid2 = serde_json::to_string(&Message::User {
        content: "second".into(),
    })
    .unwrap();
    let contents = format!("{valid1}\n{{ this is not valid json\n{valid2}\n");
    std::fs::write(dir.join("messages.jsonl"), contents).unwrap();

    // The service load tolerates the corrupt line and returns the valid ones.
    let loaded = service.load_session(session.id).await.unwrap();
    assert_eq!(
        loaded.messages.len(),
        2,
        "the two valid lines load; the malformed one is skipped"
    );
}

/// Seam: the LLM stream dies mid-tool-call. The turn must surface an error
/// (not panic or hang), and the session must remain usable for a later turn.
#[tokio::test]
async fn stream_interrupted_mid_tool_call_emits_error_and_recovers() {
    // First turn: a tool call starts streaming, then the stream errors before
    // the tool call completes. Second turn: a normal text reply succeeds.
    let interrupted = MockResponse::stream_error(
        vec![
            ChatChunk::ToolUseStart {
                index: 0,
                id: "c1".into(),
                name: "think".into(),
            },
            ChatChunk::ToolUseInputDelta {
                index: 0,
                json: "{\"thought\":\"par".into(), // truncated mid-arguments
            },
        ],
        LlmError::Api("connection reset mid-stream".into()),
    );
    let (_tmp, service) = setup_service(vec![interrupted, MockResponse::text("recovered")]);

    let session = service.create_session(None).await.unwrap();

    // Turn 1 — drain; expect an Error event and no panic/hang.
    let mut stream = service
        .send_message(session.id, "do a thing".into())
        .await
        .unwrap();
    let mut saw_error = false;
    while let Some(ev) = stream.next().await {
        if matches!(ev, EngineEvent::Error { .. }) {
            saw_error = true;
        }
    }
    assert!(
        saw_error,
        "a mid-tool-call stream failure must surface as an Error event"
    );
    drop(stream);

    // Turn 2 — the session is not bricked; a normal turn completes.
    let mut stream = service
        .send_message(session.id, "try again".into())
        .await
        .unwrap();
    let mut final_text = String::new();
    while let Some(ev) = stream.next().await {
        if let EngineEvent::Complete { final_text: ft } = ev {
            final_text = ft;
        }
    }
    assert_eq!(
        final_text, "recovered",
        "the session recovers and the next turn completes"
    );

    // History loads cleanly after the interruption + recovery.
    let loaded = service.load_session(session.id).await.unwrap();
    assert!(
        !loaded.messages.is_empty(),
        "history is readable after a mid-stream interruption"
    );
}

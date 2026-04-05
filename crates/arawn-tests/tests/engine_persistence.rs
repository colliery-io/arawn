//! Functional tests: engine loop + persistence working together.
//! These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.

use std::sync::Arc;

use tempfile::TempDir;

use arawn_core::{Message, Session, Workstream};
use arawn_engine::{
    FileReadTool, QueryEngine, QueryEngineConfig, ShellTool, ThinkTool, ToolContext, ToolRegistry,
};
use arawn_llm::{MockLlmClient, MockResponse};
use arawn_storage::Store;

/// Helper: set up a full stack with Store + Engine + MockLLM in a temp directory.
struct Fixture {
    _tmp: TempDir,
    store: Store,
    workstream: Workstream,
    ws_dir: String,
}

impl Fixture {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        let ws = Workstream::new("test-ws", tmp.path().join("workspace"));
        store.create_workstream(&ws).unwrap();
        std::fs::create_dir_all(&ws.root_dir).unwrap();
        Self {
            _tmp: tmp,
            store,
            ws_dir: "test-ws".to_string(),
            workstream: ws,
        }
    }

    fn new_session(&self) -> Session {
        let session = Session::new(self.workstream.id);
        self.store.create_session(&session).unwrap();
        session
    }

    fn scratch_session(&self) -> Session {
        let session = Session::scratch();
        self.store.create_session(&session).unwrap();
        session
    }

    fn context(&self, session: &Session) -> ToolContext {
        ToolContext::new(&self.workstream, session.id)
    }

    fn registry(&self) -> Arc<ToolRegistry> {
        let reg = Arc::new(ToolRegistry::new());
        reg.register(Box::new(ThinkTool));
        reg.register(Box::new(ShellTool::default()));
        reg.register(Box::new(FileReadTool));
        reg
    }

    fn engine(&self, mock: Arc<MockLlmClient>, registry: Arc<ToolRegistry>) -> QueryEngine {
        QueryEngine::with_config(
            mock,
            registry,
            QueryEngineConfig {
                system_prompt: "Test".into(),
                ..Default::default()
            },
        )
    }
}

#[tokio::test]
async fn engine_run_persists_all_messages() {
    let fix = Fixture::new();
    let mut session = fix.new_session();
    let ctx = fix.context(&session);
    let registry = fix.registry();

    let mock = Arc::new(MockLlmClient::new(vec![
        MockResponse::tool_call("c1", "think", r#"{"thought":"reasoning..."}"#),
        MockResponse::text("Here's my answer."),
    ]));

    let mut engine = fix.engine(mock, registry);

    let user_msg = Message::User {
        content: "Do the thing".into(),
    };
    session.add_message(user_msg.clone());
    fix.store
        .append_message(session.id, &fix.ws_dir, &user_msg)
        .await
        .unwrap();

    let msgs_before = session.messages().len();
    let result = engine.run(&mut session, &ctx).await.unwrap();
    assert_eq!(result, "Here's my answer.");

    for msg in &session.messages()[msgs_before..] {
        fix.store
            .append_message(session.id, &fix.ws_dir, msg)
            .await
            .unwrap();
    }

    let loaded = fix.store.load_session(session.id).await.unwrap().unwrap();
    assert_eq!(loaded.messages().len(), session.messages().len());

    let msgs = loaded.messages();
    assert!(matches!(&msgs[0], Message::User { .. }));
    assert!(matches!(&msgs[1], Message::Assistant { tool_uses, .. } if !tool_uses.is_empty()));
    assert!(matches!(&msgs[2], Message::ToolResult { .. }));
    assert!(matches!(&msgs[3], Message::Assistant { tool_uses, .. } if tool_uses.is_empty()));
}

#[tokio::test]
async fn session_resume_continues_conversation() {
    let fix = Fixture::new();
    let registry = fix.registry();

    // --- Turn 1 ---
    let mut session = fix.new_session();
    let ctx = fix.context(&session);

    let mock1 = Arc::new(MockLlmClient::new(vec![MockResponse::text(
        "First response.",
    )]));
    let mut engine1 = fix.engine(mock1, registry.clone());

    let user_msg = Message::User {
        content: "First question".into(),
    };
    session.add_message(user_msg.clone());
    fix.store
        .append_message(session.id, &fix.ws_dir, &user_msg)
        .await
        .unwrap();

    let msgs_before = session.messages().len();
    engine1.run(&mut session, &ctx).await.unwrap();

    for msg in &session.messages()[msgs_before..] {
        fix.store
            .append_message(session.id, &fix.ws_dir, msg)
            .await
            .unwrap();
    }

    let session_id = session.id;
    drop(session);

    // --- Turn 2: resume ---
    let mut resumed = fix.store.load_session(session_id).await.unwrap().unwrap();
    assert_eq!(resumed.messages().len(), 2);

    let mock2 = Arc::new(MockLlmClient::new(vec![MockResponse::text(
        "Second response with context.",
    )]));
    let mut engine2 = fix.engine(mock2, registry);
    let ctx2 = fix.context(&resumed);

    let user_msg2 = Message::User {
        content: "Follow-up question".into(),
    };
    resumed.add_message(user_msg2.clone());
    fix.store
        .append_message(session_id, &fix.ws_dir, &user_msg2)
        .await
        .unwrap();

    let msgs_before = resumed.messages().len();
    let result = engine2.run(&mut resumed, &ctx2).await.unwrap();
    assert_eq!(result, "Second response with context.");

    for msg in &resumed.messages()[msgs_before..] {
        fix.store
            .append_message(session_id, &fix.ws_dir, msg)
            .await
            .unwrap();
    }

    let final_session = fix.store.load_session(session_id).await.unwrap().unwrap();
    assert_eq!(final_session.messages().len(), 4);
}

#[tokio::test]
async fn tool_results_persisted_with_content() {
    let fix = Fixture::new();

    let test_file = fix.workstream.root_dir.join("data.txt");
    std::fs::write(&test_file, "important data\n").unwrap();

    let mut session = fix.new_session();
    let ctx = fix.context(&session);
    let registry = fix.registry();

    let mock = Arc::new(MockLlmClient::new(vec![
        MockResponse::tool_call("c1", "file_read", r#"{"path":"data.txt"}"#),
        MockResponse::text("The file says: important data"),
    ]));

    let mut engine = fix.engine(mock, registry);

    let user_msg = Message::User {
        content: "Read data.txt".into(),
    };
    session.add_message(user_msg.clone());
    fix.store
        .append_message(session.id, &fix.ws_dir, &user_msg)
        .await
        .unwrap();

    let msgs_before = session.messages().len();
    engine.run(&mut session, &ctx).await.unwrap();

    for msg in &session.messages()[msgs_before..] {
        fix.store
            .append_message(session.id, &fix.ws_dir, msg)
            .await
            .unwrap();
    }

    let loaded = fix.store.load_session(session.id).await.unwrap().unwrap();
    let tool_result = &loaded.messages()[2];
    match tool_result {
        Message::ToolResult {
            content, is_error, ..
        } => {
            assert!(!is_error);
            assert!(
                content.contains("important data"),
                "expected file content in tool result, got: {content}"
            );
        }
        _ => panic!("expected ToolResult at index 2"),
    }
}

#[tokio::test]
async fn scratch_session_promotion_preserves_messages() {
    let fix = Fixture::new();
    let registry = fix.registry();

    let mut session = fix.scratch_session();
    let ctx = fix.context(&session);

    let mock = Arc::new(MockLlmClient::new(vec![MockResponse::text(
        "Scratch answer.",
    )]));
    let mut engine = fix.engine(mock, registry);

    let user_msg = Message::User {
        content: "Scratch question".into(),
    };
    session.add_message(user_msg.clone());
    fix.store
        .append_message(session.id, "scratch", &user_msg)
        .await
        .unwrap();

    let msgs_before = session.messages().len();
    engine.run(&mut session, &ctx).await.unwrap();

    for msg in &session.messages()[msgs_before..] {
        fix.store
            .append_message(session.id, "scratch", msg)
            .await
            .unwrap();
    }

    let before_promote = fix
        .store
        .load_messages(session.id, "scratch")
        .await
        .unwrap();
    assert_eq!(before_promote.len(), 2);

    fix.store
        .promote_session(session.id, fix.workstream.id)
        .await
        .unwrap();

    let after_promote = fix
        .store
        .load_messages(session.id, &fix.ws_dir)
        .await
        .unwrap();
    assert_eq!(after_promote.len(), 2);

    let scratch_after = fix
        .store
        .load_messages(session.id, "scratch")
        .await
        .unwrap();
    assert!(scratch_after.is_empty());
}

#[tokio::test]
async fn multiple_sessions_isolated() {
    let fix = Fixture::new();
    let registry = fix.registry();

    // Session A
    let mut session_a = fix.new_session();
    let ctx_a = fix.context(&session_a);
    let mock_a = Arc::new(MockLlmClient::new(vec![MockResponse::text("Answer A")]));
    let mut engine_a = fix.engine(mock_a, registry.clone());

    let msg_a = Message::User {
        content: "Question A".into(),
    };
    session_a.add_message(msg_a.clone());
    fix.store
        .append_message(session_a.id, &fix.ws_dir, &msg_a)
        .await
        .unwrap();

    let before = session_a.messages().len();
    engine_a.run(&mut session_a, &ctx_a).await.unwrap();
    for msg in &session_a.messages()[before..] {
        fix.store
            .append_message(session_a.id, &fix.ws_dir, msg)
            .await
            .unwrap();
    }

    // Session B
    let mut session_b = fix.new_session();
    let ctx_b = fix.context(&session_b);
    let mock_b = Arc::new(MockLlmClient::new(vec![MockResponse::text("Answer B")]));
    let mut engine_b = fix.engine(mock_b, registry);

    let msg_b = Message::User {
        content: "Question B".into(),
    };
    session_b.add_message(msg_b.clone());
    fix.store
        .append_message(session_b.id, &fix.ws_dir, &msg_b)
        .await
        .unwrap();

    let before = session_b.messages().len();
    engine_b.run(&mut session_b, &ctx_b).await.unwrap();
    for msg in &session_b.messages()[before..] {
        fix.store
            .append_message(session_b.id, &fix.ws_dir, msg)
            .await
            .unwrap();
    }

    let loaded_a = fix.store.load_session(session_a.id).await.unwrap().unwrap();
    let loaded_b = fix.store.load_session(session_b.id).await.unwrap().unwrap();

    assert_eq!(loaded_a.messages().len(), 2);
    assert_eq!(loaded_b.messages().len(), 2);

    match &loaded_a.messages()[0] {
        Message::User { content } => assert_eq!(content, "Question A"),
        _ => panic!("wrong message"),
    }
    match &loaded_b.messages()[0] {
        Message::User { content } => assert_eq!(content, "Question B"),
        _ => panic!("wrong message"),
    }
}

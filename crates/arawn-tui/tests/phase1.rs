//! Phase 1 integration tests for arawn-tui.

use ratatui::{Terminal, backend::TestBackend};
use tokio::sync::mpsc;

use arawn_tui::app::App;
use arawn_tui::config::TuiConfig;
use arawn_tui::events::Event;
use arawn_tui::protocol::ServerMessage;

/// Helper: create an App wired to test channels.
fn make_test_app() -> (
    App,
    mpsc::UnboundedSender<Event>,
    mpsc::UnboundedReceiver<Event>,
    mpsc::UnboundedSender<ServerMessage>,
    mpsc::UnboundedReceiver<ServerMessage>,
) {
    let config = TuiConfig::new("http://127.0.0.1:9999");
    let (ws_tx, _ws_rx) = mpsc::unbounded_channel(); // outgoing WS messages (we discard)
    let app = App::new(&config, ws_tx);
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (server_tx, server_rx) = mpsc::unbounded_channel();
    (app, event_tx, event_rx, server_tx, server_rx)
}

#[tokio::test]
async fn test_headless_renders() {
    let (mut app, event_tx, event_rx, _server_tx, server_rx) = make_test_app();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Send a tick then close the channel
    event_tx.send(Event::Tick).unwrap();
    drop(event_tx);

    let result = app
        .run_headless(&mut terminal, event_rx, server_rx, 5)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_headless_key_input() {
    let (mut app, event_tx, event_rx, _server_tx, server_rx) = make_test_app();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Type "hi"
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    let key = |code: KeyCode| {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    };

    event_tx.send(key(KeyCode::Char('h'))).unwrap();
    event_tx.send(key(KeyCode::Char('i'))).unwrap();
    event_tx.send(Event::Tick).unwrap();
    drop(event_tx);

    app.run_headless(&mut terminal, event_rx, server_rx, 5)
        .await
        .unwrap();

    assert_eq!(app.input, "hi");
    assert_eq!(app.cursor_pos, 2);
}

#[tokio::test]
async fn test_headless_server_messages() {
    let (mut app, event_tx, event_rx, server_tx, server_rx) = make_test_app();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Simulate server sending session + chat
    server_tx
        .send(ServerMessage::SessionCreated {
            session_id: "test-session".into(),
        })
        .unwrap();
    server_tx
        .send(ServerMessage::ChatChunk {
            session_id: "test-session".into(),
            chunk: "Hello from Arawn".into(),
            done: true,
        })
        .unwrap();

    // Drive ticks to process
    for _ in 0..5 {
        event_tx.send(Event::Tick).unwrap();
    }
    drop(event_tx);

    app.run_headless(&mut terminal, event_rx, server_rx, 10)
        .await
        .unwrap();

    assert_eq!(app.session_id.as_deref(), Some("test-session"));
    assert_eq!(app.messages.len(), 1);
    assert_eq!(app.messages[0].content, "Hello from Arawn");
    assert!(!app.messages[0].is_user);
    assert!(!app.messages[0].streaming);
}

#[tokio::test]
async fn test_headless_chat_flow() {
    // Start a no-auth test server
    let server = arawn_test_utils::TestServer::builder()
        .with_auth(None)
        .with_text_responses(vec!["I am Arawn.".into()])
        .build()
        .await
        .unwrap();

    let config = TuiConfig::new(&server.base_url());
    let ws_url = config.ws_url();

    // Connect via our WS client
    let (ws_tx, mut ws_rx, _status_rx) = arawn_tui::ws::connect(ws_url);

    // Wait briefly for connection to establish
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut app = App::new(&config, ws_tx);

    // Set input text and send the message directly (bypassing event loop for sending)
    app.set_text("Hello");
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    app.handle_key_public(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });

    // Now wait for server messages to arrive via the ws_rx channel
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(std::time::Duration::from_millis(100), ws_rx.recv()).await {
            Ok(Some(msg)) => {
                let is_done = matches!(&msg, ServerMessage::ChatChunk { done: true, .. });
                app.handle_server_message_public(msg);
                if is_done {
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => {} // timeout, keep waiting
        }
    }

    // Verify render works with content
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| arawn_tui::render::draw(f, &app)).unwrap();

    // Should have user message + at least one assistant message
    assert!(
        app.messages.len() >= 2,
        "Expected at least 2 messages, got {}: {:?}",
        app.messages.len(),
        app.messages
            .iter()
            .map(|m| format!(
                "{}: {}",
                if m.is_user { "user" } else { "assistant" },
                &m.content
            ))
            .collect::<Vec<_>>()
    );
    assert!(app.messages[0].is_user);
    assert_eq!(app.messages[0].content, "Hello");

    // Find the assistant response
    let assistant_msg = app.messages.iter().find(|m| !m.is_user).unwrap();
    assert!(
        assistant_msg.content.contains("I am Arawn"),
        "Expected assistant to say 'I am Arawn', got: {}",
        assistant_msg.content
    );

    // Session should be set
    assert!(app.session_id.is_some());
}

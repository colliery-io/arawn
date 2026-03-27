//! Phase 4 integration tests for arawn-tui — polish features.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use tokio::sync::mpsc;

use arawn_tui::app::{App, ChatMessage, Focus, SidebarSection, WorkstreamInfo};
use arawn_tui::config::TuiConfig;
use arawn_tui::events::Event;
use arawn_tui::protocol::ServerMessage;
use arawn_tui::ws::ConnectionStatus;

/// Helper: create an App wired to test channels.
fn make_test_app() -> (
    App,
    mpsc::UnboundedSender<Event>,
    mpsc::UnboundedReceiver<Event>,
    mpsc::UnboundedSender<ServerMessage>,
    mpsc::UnboundedReceiver<ServerMessage>,
) {
    let config = TuiConfig::new("http://127.0.0.1:9999");
    let (ws_tx, _ws_rx) = mpsc::unbounded_channel();
    let app = App::new(&config, ws_tx);
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (server_tx, server_rx) = mpsc::unbounded_channel();
    (app, event_tx, event_rx, server_tx, server_rx)
}

/// Helper: make a key event.
fn key(code: KeyCode) -> crossterm::event::KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// Helper: extract rendered buffer text.
fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer().clone();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            text.push_str(cell.symbol());
        }
        text.push('\n');
    }
    text
}

/// Helper: create sample workstreams.
fn sample_workstreams() -> Vec<WorkstreamInfo> {
    vec![
        WorkstreamInfo {
            id: "ws-001".into(),
            title: "my-blog".into(),
            is_scratch: false,
        },
        WorkstreamInfo {
            id: "ws-002".into(),
            title: "code-review".into(),
            is_scratch: false,
        },
    ]
}

#[tokio::test]
async fn test_chat_scroll() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();

    // Add many messages to exceed visible area
    for i in 0..50 {
        app.messages.push(ChatMessage {
            is_user: i % 2 == 0,
            content: format!("Message number {}", i),
            streaming: false,
        });
    }

    // Auto-scroll is on by default
    assert!(app.auto_scroll);

    // Focus is on Input by default
    assert_eq!(app.focus, Focus::Input);

    // PageUp should disable auto-scroll and scroll up
    app.handle_key_public(key(KeyCode::PageUp));
    assert!(!app.auto_scroll);
    // chat_scroll was 0, saturating_sub(10) = 0
    assert_eq!(app.chat_scroll, 0);

    // Manually set chat_scroll higher, then PageUp
    app.chat_scroll = 30;
    app.handle_key_public(key(KeyCode::PageUp));
    assert_eq!(app.chat_scroll, 20);
    assert!(!app.auto_scroll);

    // PageDown should scroll down
    app.handle_key_public(key(KeyCode::PageDown));
    assert_eq!(app.chat_scroll, 30);

    // Home should go to top
    app.handle_key_public(key(KeyCode::Home));
    assert_eq!(app.chat_scroll, 0);
    assert!(!app.auto_scroll);

    // End should re-enable auto-scroll
    app.handle_key_public(key(KeyCode::End));
    assert!(app.auto_scroll);

    // Verify rendering with auto-scroll shows the latest messages
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| arawn_tui::render::draw(f, &app)).unwrap();
    let text = buffer_text(&terminal);
    // With auto-scroll, the latest message (49) should be visible
    assert!(
        text.contains("Message number 49"),
        "Auto-scroll should show latest message, got:\n{}",
        text
    );

    // Now disable auto-scroll and scroll to top
    app.auto_scroll = false;
    app.chat_scroll = 0;
    terminal.draw(|f| arawn_tui::render::draw(f, &app)).unwrap();
    let text = buffer_text(&terminal);
    // At scroll=0, the first message should be visible
    assert!(
        text.contains("Message number 0"),
        "Scroll top should show first message, got:\n{}",
        text
    );
}

#[tokio::test]
async fn test_status_bar_renders() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();

    // Set connection status and workstream info
    app.connection_status = ConnectionStatus::Connected;
    app.workstream_id = Some("ws-001".into());
    app.workstream = "my-blog".to_string();
    app.session_id = Some("abcdef1234567890".into());

    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| arawn_tui::render::draw(f, &app)).unwrap();

    let text = buffer_text(&terminal);

    assert!(
        text.contains("Connected"),
        "Status bar should show 'Connected', got:\n{}",
        text
    );
    assert!(
        text.contains("my-blog"),
        "Status bar should show workstream name, got:\n{}",
        text
    );
    assert!(
        text.contains("abcdef12"),
        "Status bar should show first 8 chars of session ID, got:\n{}",
        text
    );

    // Test with no session and no workstream
    app.session_id = None;
    app.workstream_id = None;
    app.connection_status = ConnectionStatus::Disconnected;
    terminal.draw(|f| arawn_tui::render::draw(f, &app)).unwrap();

    let text = buffer_text(&terminal);
    assert!(
        text.contains("Disconnected"),
        "Status bar should show 'Disconnected', got:\n{}",
        text
    );
    assert!(
        text.contains("scratch"),
        "Status bar should show 'scratch' for no workstream, got:\n{}",
        text
    );
    assert!(
        text.contains("no session"),
        "Status bar should show 'no session', got:\n{}",
        text
    );
}

#[tokio::test]
async fn test_create_workstream_input() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();
    app.workstreams = sample_workstreams();

    // Focus sidebar, workstreams section
    app.handle_key_public(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Sidebar);
    assert_eq!(app.sidebar_section, SidebarSection::Workstreams);

    // Initially not creating
    assert!(app.creating_workstream.is_none());

    // Press 'n' to start creating
    app.handle_key_public(key(KeyCode::Char('n')));
    assert!(app.creating_workstream.is_some());
    assert_eq!(app.creating_workstream.as_deref(), Some(""));

    // Type a name
    app.handle_key_public(key(KeyCode::Char('t')));
    app.handle_key_public(key(KeyCode::Char('e')));
    app.handle_key_public(key(KeyCode::Char('s')));
    app.handle_key_public(key(KeyCode::Char('t')));
    assert_eq!(app.creating_workstream.as_deref(), Some("test"));

    // Backspace removes a char
    app.handle_key_public(key(KeyCode::Backspace));
    assert_eq!(app.creating_workstream.as_deref(), Some("tes"));

    // Esc cancels
    app.handle_key_public(key(KeyCode::Esc));
    assert!(app.creating_workstream.is_none());

    // Press 'n' again, type, then Enter submits and clears
    app.handle_key_public(key(KeyCode::Char('n')));
    app.handle_key_public(key(KeyCode::Char('n')));
    app.handle_key_public(key(KeyCode::Char('e')));
    app.handle_key_public(key(KeyCode::Char('w')));
    assert_eq!(app.creating_workstream.as_deref(), Some("new"));

    // Enter should submit (clears creating_workstream)
    // Note: the actual HTTP call will fail since there's no server, but the state should clear
    app.handle_key_public(key(KeyCode::Enter));
    assert!(
        app.creating_workstream.is_none(),
        "creating_workstream should be None after Enter"
    );

    // Verify the creation input renders in the sidebar
    app.creating_workstream = Some("my-ws".into());
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| arawn_tui::render::draw(f, &app)).unwrap();

    let text = buffer_text(&terminal);
    assert!(
        text.contains("New: my-ws"),
        "Should show creation input in sidebar, got:\n{}",
        text
    );
}

#[tokio::test]
async fn test_status_message_overrides_bar() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();

    app.connection_status = ConnectionStatus::Connected;
    app.workstream_id = Some("ws-001".into());
    app.workstream = "my-blog".to_string();

    // Set a transient status
    app.handle_server_message_public(ServerMessage::Error {
        code: "test".into(),
        message: "Something went wrong".into(),
    });

    assert!(app.status.is_some());

    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| arawn_tui::render::draw(f, &app)).unwrap();

    let text = buffer_text(&terminal);

    // Status message should override the normal status bar
    assert!(
        text.contains("Something went wrong"),
        "Error status should appear in status bar, got:\n{}",
        text
    );

    // On user action, status should clear
    app.handle_key_public(key(KeyCode::Char('a')));
    assert!(app.status.is_none(), "Status should clear on user action");
}

#[tokio::test]
async fn test_auto_scroll_on_new_messages() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();

    // Add enough messages to fill the view
    for i in 0..30 {
        app.messages.push(ChatMessage {
            is_user: true,
            content: format!("Msg {}", i),
            streaming: false,
        });
    }

    // Disable auto-scroll
    app.auto_scroll = false;
    app.chat_scroll = 5;

    // Receiving a chat chunk re-enables auto-scroll
    app.handle_server_message_public(ServerMessage::ChatChunk {
        session_id: "s1".into(),
        chunk: "New response".into(),
        done: true,
    });

    assert!(
        app.auto_scroll,
        "Auto-scroll should re-enable when new messages arrive"
    );
}

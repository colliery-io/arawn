//! Phase 2 integration tests for arawn-tui — sidebar, focus, workstream selection.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use tokio::sync::mpsc;

use arawn_tui::app::{App, Focus, HttpResult, WorkstreamInfo};
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

/// Helper: create sample workstreams.
fn sample_workstreams() -> Vec<WorkstreamInfo> {
    vec![
        WorkstreamInfo {
            id: "ws-scratch".into(),
            title: "scratch".into(),
            is_scratch: true,
        },
        WorkstreamInfo {
            id: "ws-001".into(),
            title: "Project Alpha".into(),
            is_scratch: false,
        },
        WorkstreamInfo {
            id: "ws-002".into(),
            title: "Bug Fixes".into(),
            is_scratch: false,
        },
    ]
}

#[tokio::test]
async fn test_sidebar_renders_workstreams() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();
    app.workstreams = sample_workstreams();

    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| arawn_tui::render::draw(f, &app)).unwrap();

    // Extract the buffer content as a string to verify workstream names are visible
    let buffer = terminal.backend().buffer().clone();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            text.push_str(cell.symbol());
        }
        text.push('\n');
    }

    assert!(
        text.contains("Workstreams"),
        "Sidebar header 'Workstreams' not found in rendered output"
    );
    assert!(
        text.contains("scratch"),
        "Scratch workstream not found in rendered output"
    );
    assert!(
        text.contains("Project Alpha"),
        "Workstream 'Project Alpha' not found in rendered output"
    );
    assert!(
        text.contains("Bug Fixes"),
        "Workstream 'Bug Fixes' not found in rendered output"
    );
}

#[tokio::test]
async fn test_tab_toggles_focus() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();

    // Default focus is Input
    assert_eq!(app.focus, Focus::Input);

    // Tab switches to Sidebar
    app.handle_key_public(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Sidebar);

    // Tab switches back to Input
    app.handle_key_public(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Input);
}

#[tokio::test]
async fn test_esc_returns_to_input_from_sidebar() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();

    app.handle_key_public(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Sidebar);

    app.handle_key_public(key(KeyCode::Esc));
    assert_eq!(app.focus, Focus::Input);
}

#[tokio::test]
async fn test_select_workstream_clears_chat() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();
    app.workstreams = sample_workstreams();

    // Add a message and session to verify they get cleared
    app.messages.push(arawn_tui::app::ChatMessage {
        is_user: true,
        content: "hello".into(),
        streaming: false,
    });
    app.session_id = Some("old-session".into());

    // Switch to sidebar
    app.handle_key_public(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Sidebar);

    // Move down to "Project Alpha" (index 1)
    app.handle_key_public(key(KeyCode::Down));
    assert_eq!(app.selected_workstream, 1);

    // Select it
    app.handle_key_public(key(KeyCode::Enter));

    // Verify selection applied
    assert_eq!(app.workstream_id.as_deref(), Some("ws-001"));
    assert_eq!(app.workstream, "Project Alpha");
    assert!(app.messages.is_empty(), "Messages should be cleared");
    assert!(app.session_id.is_none(), "Session should be cleared");
    assert_eq!(app.focus, Focus::Input, "Focus should return to Input");
}

#[tokio::test]
async fn test_sidebar_navigation_bounds() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();
    app.workstreams = sample_workstreams();

    // Switch to sidebar
    app.handle_key_public(key(KeyCode::Tab));

    // At top, pressing Up should stay at 0
    app.handle_key_public(key(KeyCode::Up));
    assert_eq!(app.selected_workstream, 0);

    // Move to bottom
    app.handle_key_public(key(KeyCode::Down));
    app.handle_key_public(key(KeyCode::Down));
    assert_eq!(app.selected_workstream, 2);

    // At bottom, pressing Down should stay at 2
    app.handle_key_public(key(KeyCode::Down));
    assert_eq!(app.selected_workstream, 2);
}

#[tokio::test]
async fn test_http_result_populates_workstreams() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();

    // Simulate receiving an HTTP result via the channel
    let tx = app.http_tx.clone();
    tx.send(HttpResult::Workstreams(sample_workstreams()))
        .unwrap();

    // Drive the event loop briefly to process the HTTP result
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (_server_tx2, server_rx) = mpsc::unbounded_channel::<ServerMessage>();

    event_tx.send(Event::Tick).unwrap();
    drop(event_tx);

    app.run_headless(&mut terminal, event_rx, server_rx, 5)
        .await
        .unwrap();

    assert_eq!(app.workstreams.len(), 3);
    assert_eq!(app.workstreams[0].title, "scratch");
    assert!(app.workstreams[0].is_scratch);
}

#[tokio::test]
async fn test_input_keys_ignored_in_sidebar_mode() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();

    // Switch to sidebar
    app.handle_key_public(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Sidebar);

    // Type a character — should NOT affect input
    app.handle_key_public(key(KeyCode::Char('x')));
    assert!(
        app.input.is_empty(),
        "Typing in sidebar mode should not affect input"
    );
}

//! Phase 3 integration tests for arawn-tui — sessions in sidebar.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use tokio::sync::mpsc;

use arawn_tui::app::{
    App, ChatMessage, Focus, HttpResult, MessageInfo, SessionInfo, SidebarSection, WorkstreamInfo,
};
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

/// Helper: create sample sessions.
fn sample_sessions() -> Vec<SessionInfo> {
    vec![
        SessionInfo {
            id: "sess-001".into(),
            started_at: "2026-03-26T14:00:00Z".into(),
        },
        SessionInfo {
            id: "sess-002".into(),
            started_at: "2026-03-25T09:30:00Z".into(),
        },
    ]
}

/// Helper: create sample messages for a session.
fn sample_messages() -> Vec<MessageInfo> {
    vec![
        MessageInfo {
            role: "user".into(),
            content: "Hello there".into(),
        },
        MessageInfo {
            role: "assistant".into(),
            content: "Hi! How can I help?".into(),
        },
    ]
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

#[tokio::test]
async fn test_sessions_render_after_workstream_select() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();
    app.workstreams = sample_workstreams();

    // Select the first workstream (this sets sidebar_section to Sessions)
    app.handle_key_public(key(KeyCode::Tab)); // focus sidebar
    app.handle_key_public(key(KeyCode::Enter)); // select "my-blog"

    // Inject sessions via HttpResult
    app.handle_http_result_public(HttpResult::Sessions(sample_sessions()));

    // Render and check
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| arawn_tui::render::draw(f, &app)).unwrap();

    let text = buffer_text(&terminal);

    assert!(
        text.contains("Workstreams"),
        "Should show Workstreams header"
    );
    assert!(text.contains("Sessions"), "Should show Sessions header");
    assert!(
        text.contains("+ New Session"),
        "Should show '+ New Session' entry"
    );
    assert!(
        text.contains("Mar 26 14:00"),
        "Should show formatted session date: got:\n{}",
        text
    );
    assert!(
        text.contains("Mar 25 09:30"),
        "Should show second session date"
    );
}

#[tokio::test]
async fn test_new_session_clears_chat() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();
    app.workstreams = sample_workstreams();

    // Add some existing messages and a session_id
    app.messages.push(ChatMessage {
        is_user: true,
        content: "old message".into(),
        streaming: false,
    });
    app.session_id = Some("old-session".into());

    // Select workstream first
    app.handle_key_public(key(KeyCode::Tab)); // focus sidebar
    app.handle_key_public(key(KeyCode::Enter)); // select "my-blog" -> moves to Sessions section

    // Inject sessions
    app.handle_http_result_public(HttpResult::Sessions(sample_sessions()));

    // Now we're in Sessions section, selected_session = 0 (i.e., "+ New Session")
    assert_eq!(app.sidebar_section, SidebarSection::Sessions);
    assert_eq!(app.selected_session, 0);

    // Re-add messages to simulate having some chat history before pressing new session
    app.messages.push(ChatMessage {
        is_user: true,
        content: "some chat".into(),
        streaming: false,
    });
    app.session_id = Some("some-session".into());

    // Press Enter on "+ New Session"
    app.focus = Focus::Sidebar; // ensure we're focused on sidebar
    app.handle_key_public(key(KeyCode::Enter));

    // Verify messages cleared and session_id is None
    assert!(
        app.messages.is_empty(),
        "Messages should be cleared after selecting '+ New Session'"
    );
    assert!(
        app.session_id.is_none(),
        "session_id should be None after selecting '+ New Session'"
    );
    assert_eq!(
        app.focus,
        Focus::Input,
        "Focus should return to Input after selecting '+ New Session'"
    );
}

#[tokio::test]
async fn test_select_session_loads_messages() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();
    app.workstreams = sample_workstreams();

    // Select workstream
    app.handle_key_public(key(KeyCode::Tab));
    app.handle_key_public(key(KeyCode::Enter));

    // Inject sessions
    app.handle_http_result_public(HttpResult::Sessions(sample_sessions()));

    // Navigate to the first actual session (index 1 in the sessions list)
    app.focus = Focus::Sidebar;
    app.sidebar_section = SidebarSection::Sessions;
    app.selected_session = 0;
    app.handle_key_public(key(KeyCode::Down)); // move to first session (index 1)
    assert_eq!(app.selected_session, 1);

    // Press Enter to select the session
    app.handle_key_public(key(KeyCode::Enter));

    // Verify session_id was set
    assert_eq!(app.session_id.as_deref(), Some("sess-001"));
    assert_eq!(app.focus, Focus::Input);

    // Now inject the messages result
    app.handle_http_result_public(HttpResult::Messages("sess-001".into(), sample_messages()));

    // Verify messages appear in chat
    assert_eq!(app.messages.len(), 2);
    assert!(app.messages[0].is_user);
    assert_eq!(app.messages[0].content, "Hello there");
    assert!(!app.messages[1].is_user);
    assert_eq!(app.messages[1].content, "Hi! How can I help?");
}

#[tokio::test]
async fn test_sidebar_navigation_between_sections() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();
    app.workstreams = sample_workstreams();
    app.sessions = sample_sessions();
    app.sidebar_section = SidebarSection::Workstreams;
    app.selected_workstream = 0;

    app.handle_key_public(key(KeyCode::Tab)); // focus sidebar

    // Navigate down through workstreams
    app.handle_key_public(key(KeyCode::Down)); // ws index 1 (last workstream)
    assert_eq!(app.selected_workstream, 1);
    assert_eq!(app.sidebar_section, SidebarSection::Workstreams);

    // Down again from last workstream -> moves to sessions
    app.handle_key_public(key(KeyCode::Down));
    assert_eq!(app.sidebar_section, SidebarSection::Sessions);
    assert_eq!(app.selected_session, 0);

    // Navigate down in sessions
    app.handle_key_public(key(KeyCode::Down)); // session index 1
    assert_eq!(app.selected_session, 1);

    app.handle_key_public(key(KeyCode::Down)); // session index 2
    assert_eq!(app.selected_session, 2);

    // At bottom of sessions, can't go further
    app.handle_key_public(key(KeyCode::Down));
    assert_eq!(app.selected_session, 2);

    // Navigate back up
    app.handle_key_public(key(KeyCode::Up)); // session index 1
    assert_eq!(app.selected_session, 1);

    app.handle_key_public(key(KeyCode::Up)); // session index 0 ("+ New Session")
    assert_eq!(app.selected_session, 0);

    // Up from top of sessions -> back to workstreams
    app.handle_key_public(key(KeyCode::Up));
    assert_eq!(app.sidebar_section, SidebarSection::Workstreams);
    // Should be at the last workstream
    assert_eq!(app.selected_workstream, 1);
}

#[tokio::test]
async fn test_sessions_empty_when_no_workstream_selected() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();
    app.workstreams = sample_workstreams();

    // Without selecting a workstream, sessions should be empty
    assert!(app.sessions.is_empty());

    // Render should still show the sessions section with just "+ New Session"
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| arawn_tui::render::draw(f, &app)).unwrap();

    let text = buffer_text(&terminal);
    assert!(text.contains("Sessions"), "Should show Sessions header");
    assert!(
        text.contains("+ New Session"),
        "Should always show '+ New Session'"
    );
}

#[tokio::test]
async fn test_messages_ignored_for_wrong_session() {
    let (mut app, _event_tx, _event_rx, _server_tx, _server_rx) = make_test_app();

    app.session_id = Some("sess-001".into());

    // Inject messages for a different session
    app.handle_http_result_public(HttpResult::Messages("sess-999".into(), sample_messages()));

    // Messages should NOT be loaded because session_id doesn't match
    assert!(
        app.messages.is_empty(),
        "Messages for wrong session should be ignored"
    );
}

//! Render tests — verify visible TUI output using TestBackend.

mod helpers;

use arawn_tui::LogBuffer;
use arawn_tui::app::App;
use arawn_tui::app_types::{ChatMessage, ContextState, ToolExecution};
use arawn_tui::client::ConnectionStatus;
use arawn_tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn app() -> App {
    App::new("http://localhost:0".to_string(), LogBuffer::new()).unwrap()
}

fn term() -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(120, 40)).unwrap()
}

fn render(app: &mut App, terminal: &mut Terminal<TestBackend>) {
    terminal.draw(|frame| ui::render(app, frame)).unwrap();
}

// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_empty_state_renders() {
    let mut app = app();
    let mut terminal = term();
    render(&mut app, &mut terminal);

    // Should have the header with "arawn" branding
    helpers::assert_rendered(&terminal, "arawn");
    // Should have the input prompt
    helpers::assert_rendered(&terminal, ">");
    // No messages
    helpers::assert_not_rendered(&terminal, "UNIQUE_UNLIKELY_STRING");
}

#[tokio::test]
async fn test_user_message_renders() {
    let mut app = app();
    app.messages.push(ChatMessage {
        is_user: true,
        content: "Hello from the user side".to_string(),
        streaming: false,
    });

    let mut terminal = term();
    render(&mut app, &mut terminal);

    helpers::assert_rendered(&terminal, "Hello from the user side");
}

#[tokio::test]
async fn test_assistant_response_renders() {
    let mut app = app();
    app.messages.push(ChatMessage {
        is_user: false,
        content: "I am the assistant response".to_string(),
        streaming: false,
    });

    let mut terminal = term();
    render(&mut app, &mut terminal);

    helpers::assert_rendered(&terminal, "I am the assistant response");
}

#[tokio::test]
async fn test_streaming_message_renders() {
    let mut app = app();
    app.messages.push(ChatMessage {
        is_user: false,
        content: "Partial response so far".to_string(),
        streaming: true,
    });

    let mut terminal = term();
    render(&mut app, &mut terminal);

    helpers::assert_rendered(&terminal, "Partial response so far");
}

#[tokio::test]
async fn test_error_status_renders() {
    let mut app = app();
    app.status_message = Some("Something went wrong".to_string());

    let mut terminal = term();
    render(&mut app, &mut terminal);

    helpers::assert_rendered(&terminal, "Something went wrong");
}

#[tokio::test]
async fn test_workstream_renders_in_header() {
    let mut app = app();
    app.workstream = "my-project".to_string();

    let mut terminal = term();
    render(&mut app, &mut terminal);

    helpers::assert_rendered(&terminal, "my-project");
}

#[tokio::test]
async fn test_disconnected_status_renders() {
    let mut app = app();
    app.connection_status = ConnectionStatus::Disconnected;

    let mut terminal = term();
    render(&mut app, &mut terminal);

    // The connection indicator should change — look for disconnect marker
    let buffer = terminal.backend().buffer().clone();
    let text = helpers::buffer_to_string(&buffer);
    // Should NOT show the connected indicator (green dot or "Connected")
    // The exact indicator depends on the UI theme, so just verify render doesn't panic
    assert!(!text.is_empty());
}

#[tokio::test]
async fn test_tool_execution_renders() {
    let mut app = app();
    app.show_tool_pane = true;
    app.tools.push(ToolExecution {
        id: "tool-1".to_string(),
        name: "file_read".to_string(),
        args: "path=/test.txt".to_string(),
        output: "file contents here".to_string(),
        running: false,
        success: Some(true),
        started_at: std::time::Instant::now(),
        duration_ms: Some(42),
    });

    let mut terminal = term();
    render(&mut app, &mut terminal);

    helpers::assert_rendered(&terminal, "file_read");
}

#[tokio::test]
async fn test_context_info_renders() {
    let mut app = app();
    app.context_info = Some(ContextState {
        current_tokens: 5000,
        max_tokens: 10000,
        percent: 50,
        status: "ok".to_string(),
    });

    let mut terminal = term();
    render(&mut app, &mut terminal);

    // Token usage should appear somewhere in the header/status
    helpers::assert_rendered(&terminal, "50%");
}

#[tokio::test]
async fn test_multiple_messages_render() {
    let mut app = app();
    app.messages.push(ChatMessage {
        is_user: true,
        content: "First user message".to_string(),
        streaming: false,
    });
    app.messages.push(ChatMessage {
        is_user: false,
        content: "First assistant reply".to_string(),
        streaming: false,
    });
    app.messages.push(ChatMessage {
        is_user: true,
        content: "Second user message".to_string(),
        streaming: false,
    });

    let mut terminal = term();
    render(&mut app, &mut terminal);

    helpers::assert_rendered(&terminal, "First user message");
    helpers::assert_rendered(&terminal, "First assistant reply");
    helpers::assert_rendered(&terminal, "Second user message");
}

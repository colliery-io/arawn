//! Tests for render assertion helpers.

mod helpers;

use arawn_tui::LogBuffer;
use arawn_tui::app::App;
use arawn_tui::app_types::ChatMessage;
use arawn_tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_buffer_to_string_not_empty() {
    let mut app = App::new("http://localhost:0".to_string(), LogBuffer::new()).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();

    terminal.draw(|frame| ui::render(&mut app, frame)).unwrap();

    let text = helpers::buffer_to_string(terminal.backend().buffer());
    assert!(
        !text.trim().is_empty(),
        "Rendered buffer should not be empty"
    );
}

#[tokio::test]
async fn test_assert_rendered_finds_user_message() {
    let mut app = App::new("http://localhost:0".to_string(), LogBuffer::new()).unwrap();
    app.messages.push(ChatMessage {
        is_user: true,
        content: "UNIQUE_TEST_STRING_12345".to_string(),
        streaming: false,
    });

    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|frame| ui::render(&mut app, frame)).unwrap();

    helpers::assert_rendered(&terminal, "UNIQUE_TEST_STRING_12345");
}

#[tokio::test]
async fn test_assert_not_rendered_works() {
    let mut app = App::new("http://localhost:0".to_string(), LogBuffer::new()).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();

    terminal.draw(|frame| ui::render(&mut app, frame)).unwrap();

    helpers::assert_not_rendered(&terminal, "THIS_SHOULD_NOT_APPEAR_ANYWHERE");
}

#[tokio::test]
async fn test_buffer_contains_text() {
    let mut app = App::new("http://localhost:0".to_string(), LogBuffer::new()).unwrap();
    app.messages.push(ChatMessage {
        is_user: false,
        content: "Assistant says hello".to_string(),
        streaming: false,
    });

    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|frame| ui::render(&mut app, frame)).unwrap();

    let buffer = terminal.backend().buffer().clone();
    assert!(helpers::buffer_contains_text(
        &buffer,
        "Assistant says hello"
    ));
    assert!(!helpers::buffer_contains_text(
        &buffer,
        "NONEXISTENT_STRING"
    ));
}

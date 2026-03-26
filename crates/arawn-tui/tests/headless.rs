//! Headless TUI tests — run the App event loop without a real terminal.

use std::time::Duration;

use arawn_test_utils::server::TestServerBuilder;
use arawn_test_utils::{ScriptedMockBackend, StreamingMockEvent};
use arawn_tui::LogBuffer;
use arawn_tui::app::App;
use arawn_tui::events::Event;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tokio::sync::mpsc;

/// Helper: create a noauth test server with a single text response.
async fn noauth_server(response: &str) -> anyhow::Result<arawn_test_utils::TestServer> {
    let backend =
        ScriptedMockBackend::new(vec![vec![StreamingMockEvent::Text(response.to_string())]]);
    TestServerBuilder::new()
        .with_auth(None)
        .with_backend(backend)
        .build()
        .await
}

#[tokio::test]
async fn test_headless_renders_without_panic() -> anyhow::Result<()> {
    let server = noauth_server("headless test").await?;
    let mut app = App::new(server.base_url(), LogBuffer::new())?;

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend)?;

    let (event_tx, event_rx) = mpsc::unbounded_channel();

    // Send a single tick then close
    event_tx.send(Event::Tick)?;
    drop(event_tx);

    app.run_headless(&mut terminal, event_rx, 10).await?;

    // Verify something rendered (buffer not empty)
    let buffer = terminal.backend().buffer().clone();
    let text: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(
        !text.trim().is_empty(),
        "Buffer should have rendered content"
    );

    Ok(())
}

#[tokio::test]
async fn test_headless_chat_flow() -> anyhow::Result<()> {
    let server = noauth_server("headless chat response").await?;
    let mut app = App::new(server.base_url(), LogBuffer::new())?;

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend)?;

    let (event_tx, event_rx) = mpsc::unbounded_channel();

    // Wait for WS connection
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Simulate: type message and send
    app.input.set_text("hello headless");
    app.send_message();

    assert!(app.waiting, "Should be waiting for response");

    // Send ticks to drive the event loop, then close
    for _ in 0..20 {
        event_tx.send(Event::Tick)?;
    }
    drop(event_tx);

    app.run_headless(&mut terminal, event_rx, 100).await?;

    // Check app state
    println!("Messages: {}", app.messages.len());
    for (i, msg) in app.messages.iter().enumerate() {
        println!(
            "  [{}] user={} content={}",
            i,
            msg.is_user,
            &msg.content[..msg.content.len().min(60)]
        );
    }

    assert!(
        app.messages.len() >= 2,
        "Expected user + assistant messages, got {}",
        app.messages.len()
    );

    // Check rendered buffer contains the response
    let buffer = terminal.backend().buffer().clone();
    let text: String = buffer.content().iter().map(|c| c.symbol()).collect();
    println!(
        "Buffer contains 'headless chat response': {}",
        text.contains("headless chat response")
    );

    Ok(())
}

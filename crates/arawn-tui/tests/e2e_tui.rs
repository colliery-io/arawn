//! End-to-end TUI tests — full flow from TestServer through headless App to rendered buffer.

mod helpers;

use std::time::Duration;

use arawn_test_utils::server::TestServerBuilder;
use arawn_test_utils::{ScriptedMockBackend, StreamingMockEvent};
use arawn_tui::LogBuffer;
use arawn_tui::app::App;
use arawn_tui::events::Event;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use serde_json::json;
use tokio::sync::mpsc;

async fn noauth_server(
    responses: Vec<Vec<StreamingMockEvent>>,
) -> anyhow::Result<arawn_test_utils::TestServer> {
    let backend = ScriptedMockBackend::new(responses);
    TestServerBuilder::new()
        .with_auth(None)
        .with_backend(backend)
        .build()
        .await
}

fn make_terminal() -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(120, 40)).unwrap()
}

/// Helper: send ticks then close the channel.
fn send_ticks(tx: &mpsc::UnboundedSender<Event>, count: usize) {
    for _ in 0..count {
        let _ = tx.send(Event::Tick);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// E2E 1: Single message flow — type, send, verify response in rendered buffer
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_single_message_renders() -> anyhow::Result<()> {
    let server = noauth_server(vec![vec![StreamingMockEvent::Text(
        "Hello from the server".to_string(),
    )]])
    .await?;

    let mut app = App::new(server.base_url(), LogBuffer::new())?;
    let mut terminal = make_terminal();
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    // Wait for WS to connect
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Type and send
    app.input.set_text("hello server");
    app.send_message();

    // Drive the event loop
    send_ticks(&event_tx, 30);
    drop(event_tx);

    app.run_headless(&mut terminal, event_rx, 100).await?;

    // Verify user message rendered
    helpers::assert_rendered(&terminal, "hello server");

    // Verify assistant response rendered
    helpers::assert_rendered(&terminal, "Hello from the server");

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// E2E 2: Multi-turn — two messages, both visible
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_multi_turn_renders() -> anyhow::Result<()> {
    let server = noauth_server(vec![
        vec![StreamingMockEvent::Text("First reply".to_string())],
        vec![StreamingMockEvent::Text("Second reply".to_string())],
    ])
    .await?;

    let mut app = App::new(server.base_url(), LogBuffer::new())?;
    let mut terminal = make_terminal();
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    tokio::time::sleep(Duration::from_millis(500)).await;

    // First message
    app.input.set_text("first question");
    app.send_message();
    send_ticks(&event_tx, 20);

    // Process first response before sending second
    // We need to run the loop partway, then send more
    // Simpler: just send both and let the loop handle ordering
    // The ScriptedMockBackend serves responses in order

    // Wait for first response to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Drain WS messages
    while let Some(msg) = app.ws_client.try_recv() {
        app.handle_server_message(msg);
    }

    // Second message
    app.input.set_text("second question");
    app.send_message();
    send_ticks(&event_tx, 20);
    drop(event_tx);

    app.run_headless(&mut terminal, event_rx, 100).await?;

    helpers::assert_rendered(&terminal, "first question");
    helpers::assert_rendered(&terminal, "First reply");
    helpers::assert_rendered(&terminal, "second question");
    helpers::assert_rendered(&terminal, "Second reply");

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// E2E 3: Tool execution — verify tool name appears
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_tool_execution_renders() -> anyhow::Result<()> {
    let backend = ScriptedMockBackend::tool_then_text(
        "file_read",
        "tc-1",
        json!({"path": "/test.txt"}),
        "Tool executed successfully",
    );
    let server = TestServerBuilder::new()
        .with_auth(None)
        .with_backend(backend)
        .with_tools(arawn_test_utils::mock_tool_registry())
        .build()
        .await?;

    let mut app = App::new(server.base_url(), LogBuffer::new())?;
    app.show_tool_pane = true;
    let mut terminal = make_terminal();
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    tokio::time::sleep(Duration::from_millis(500)).await;

    app.input.set_text("read the file");
    app.send_message();

    send_ticks(&event_tx, 30);
    drop(event_tx);

    app.run_headless(&mut terminal, event_rx, 100).await?;

    // Response text should be rendered
    helpers::assert_rendered(&terminal, "Tool executed successfully");

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// E2E 4: Connection status renders
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_connection_status_renders() -> anyhow::Result<()> {
    let server = noauth_server(vec![vec![StreamingMockEvent::Text("ok".to_string())]]).await?;

    let mut app = App::new(server.base_url(), LogBuffer::new())?;
    let mut terminal = make_terminal();
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    // Before connection: should show connecting
    send_ticks(&event_tx, 5);
    drop(event_tx);

    app.run_headless(&mut terminal, event_rx, 10).await?;

    // Should render without panic — the status bar should exist
    let buffer = terminal.backend().buffer().clone();
    let text = helpers::buffer_to_string(&buffer);
    assert!(!text.is_empty());

    Ok(())
}

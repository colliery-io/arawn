//! Reproduction test for the TUI hang bug.
//!
//! User reports: TUI connects, sends message, no response appears.
//! Server logs show the turn completed but TUI never renders it.

mod helpers;

use std::time::Duration;

use arawn_test_utils::server::TestServerBuilder;
use arawn_test_utils::{ScriptedMockBackend, StreamingMockEvent};
use arawn_tui::LogBuffer;
use arawn_tui::app::App;
use arawn_tui::events::Event;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tokio::sync::mpsc;

/// Reproduce: user creates a workstream via the REST API, TUI shows it,
/// user sends a chat message in that workstream context.
#[tokio::test]
async fn test_reproduce_hang_with_workstream() -> anyhow::Result<()> {
    let server = {
        let backend = ScriptedMockBackend::new(vec![vec![StreamingMockEvent::Text(
            "workstream response".to_string(),
        )]]);
        TestServerBuilder::new()
            .with_auth(None)
            .with_backend(backend)
            .with_workstreams()
            .build()
            .await?
    };

    let mut app = App::new(server.base_url(), LogBuffer::new())?;
    let mut terminal = Terminal::new(TestBackend::new(120, 40))?;
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    // Wait for WS to connect
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create a workstream via REST (like the sidebar does)
    let resp = server
        .client
        .post(format!("{}/api/v1/workstreams", server.base_url()))
        .json(&serde_json::json!({"title": "test-ws", "tags": ["test"]}))
        .send()
        .await?;
    let ws_body: serde_json::Value = resp.json().await?;
    let ws_id = ws_body["id"].as_str().unwrap().to_string();
    println!("Created workstream: {}", ws_id);

    // Simulate TUI switching to this workstream
    app.workstream = "test-ws".to_string();
    app.workstream_id = Some(ws_id.clone());
    app.session_id = None; // New workstream, no session yet

    // Send a message in this workstream context
    app.input.set_text("hello in workstream");
    app.send_message();

    assert!(app.waiting, "Should be waiting after send");
    println!("Sent message, waiting={}", app.waiting);

    // Drive the headless loop
    send_ticks(&event_tx, 30);
    drop(event_tx);

    app.run_headless(&mut terminal, event_rx, 100).await?;

    println!("Messages after run: {}", app.messages.len());
    for (i, msg) in app.messages.iter().enumerate() {
        println!(
            "  [{}] user={} streaming={} len={}",
            i,
            msg.is_user,
            msg.streaming,
            msg.content.len()
        );
    }
    println!("waiting={}", app.waiting);

    // This is the bug: does the response appear?
    assert!(
        app.messages.len() >= 2,
        "Expected user + assistant messages, got {}. BUG: TUI hang reproduced.",
        app.messages.len()
    );

    helpers::assert_rendered(&terminal, "workstream response");

    Ok(())
}

/// Reproduce: user is in the default scratch workstream (no explicit workstream_id).
#[tokio::test]
async fn test_chat_in_scratch_workstream() -> anyhow::Result<()> {
    let server = {
        let backend = ScriptedMockBackend::new(vec![vec![StreamingMockEvent::Text(
            "scratch response".to_string(),
        )]]);
        TestServerBuilder::new()
            .with_auth(None)
            .with_backend(backend)
            .build()
            .await?
    };

    let mut app = App::new(server.base_url(), LogBuffer::new())?;
    let mut terminal = Terminal::new(TestBackend::new(120, 40))?;
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Default state: scratch workstream, no workstream_id set
    app.input.set_text("hello scratch");
    app.send_message();

    send_ticks(&event_tx, 30);
    drop(event_tx);

    app.run_headless(&mut terminal, event_rx, 100).await?;

    assert!(
        app.messages.len() >= 2,
        "Expected user + assistant messages in scratch, got {}",
        app.messages.len()
    );

    helpers::assert_rendered(&terminal, "scratch response");

    Ok(())
}

fn send_ticks(tx: &mpsc::UnboundedSender<Event>, count: usize) {
    for _ in 0..count {
        let _ = tx.send(Event::Tick);
    }
}

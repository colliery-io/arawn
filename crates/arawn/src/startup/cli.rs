//! CLI client path — connects to the local arawn server over WebSocket
//! and streams a single prompt's response. Used when the binary is run
//! without `serve`/`tui` but with a prompt argument.

use anyhow::Result;
use uuid::Uuid;

/// Run a CLI prompt by connecting to the running server via WebSocket.
pub async fn run_cli_via_server(url: &str, prompt: &str, session_id: Option<Uuid>) -> Result<()> {
    use arawn_tui::ws_client::{EventUpdate, WsClient, engine_event_to_update, parse_engine_event};

    let mut client = WsClient::connect(url).await.map_err(|e| {
        anyhow::anyhow!(
            "Could not connect to arawn server at {url}: {e}\n\
             Start the server first: arawn serve"
        )
    })?;

    // Create or resume session
    let session_uuid = match session_id {
        Some(id) => {
            eprintln!("Resuming session {id}");
            id
        }
        None => {
            let s = client
                .create_session(None)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create session: {e}"))?;
            eprintln!("Session: {}", s.id);
            s.id
        }
    };

    // Send the prompt
    if prompt.is_empty() {
        eprintln!("No prompt provided");
        std::process::exit(1);
    }

    // request_response awaits the JSON-RPC ack via the dedicated reader
    // task — it can fail synchronously if the server rejected the request.
    let ack = client
        .request_response(
            "send_message",
            serde_json::json!({
                "session_id": session_uuid.to_string(),
                "content": prompt,
            }),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send message: {e}"))?;
    if let Some(err) = ack.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        eprintln!("Server error: {msg}");
        std::process::exit(1);
    }

    eprintln!("Thinking...\n");

    let mut events = client
        .events_take()
        .ok_or_else(|| anyhow::anyhow!("ws events channel already taken"))?;

    // Stream events until Complete or Error
    let final_text = 'stream: loop {
        let ev = events.recv().await;
        match ev {
            Some(arawn_tui::ws_client::WsEvent::Text(text)) => {
                if let Some(event) = parse_engine_event(&text) {
                    match engine_event_to_update(event) {
                        EventUpdate::AddToolCall { name, .. } => {
                            eprintln!("  [{name}]");
                        }
                        EventUpdate::AddToolResult { is_error, .. } => {
                            if is_error {
                                eprintln!("  [error]");
                            }
                        }
                        EventUpdate::Complete(text) => {
                            break 'stream text;
                        }
                        EventUpdate::Error(message) => {
                            eprintln!("Error: {message}");
                            std::process::exit(1);
                        }
                        _ => {}
                    }
                }
            }
            Some(arawn_tui::ws_client::WsEvent::Closed) => {
                eprintln!("Server closed connection");
                std::process::exit(1);
            }
            Some(arawn_tui::ws_client::WsEvent::Error(e)) => {
                eprintln!("WebSocket error: {e}");
                std::process::exit(1);
            }
            None => {
                eprintln!("Connection lost");
                std::process::exit(1);
            }
        }
    };

    println!("{final_text}");
    Ok(())
}

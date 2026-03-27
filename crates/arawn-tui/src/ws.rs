//! WebSocket client with background connection loop and reconnection.

use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;

use crate::protocol::{ClientMessage, ServerMessage};

/// Connection status reported via watch channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
    Failed(String),
}

/// Connect to the WebSocket server and return channels for communication.
///
/// Spawns a background tokio task that manages the connection lifecycle
/// including automatic reconnection on disconnect.
pub fn connect(
    url: String,
) -> (
    mpsc::UnboundedSender<ClientMessage>,
    mpsc::UnboundedReceiver<ServerMessage>,
    watch::Receiver<ConnectionStatus>,
) {
    let (client_tx, mut client_rx) = mpsc::unbounded_channel::<ClientMessage>();
    let (server_tx, server_rx) = mpsc::unbounded_channel::<ServerMessage>();
    let (status_tx, status_rx) = watch::channel(ConnectionStatus::Connecting);

    tokio::spawn(async move {
        let mut retry_delay = Duration::from_millis(500);
        let max_retry_delay = Duration::from_secs(10);

        loop {
            let _ = status_tx.send(ConnectionStatus::Connecting);

            let ws_stream = match tokio_tungstenite::connect_async(&url).await {
                Ok((stream, _)) => {
                    let _ = status_tx.send(ConnectionStatus::Connected);
                    retry_delay = Duration::from_millis(500);
                    stream
                }
                Err(e) => {
                    let _ = status_tx.send(ConnectionStatus::Failed(e.to_string()));
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(max_retry_delay);
                    continue;
                }
            };

            let (mut ws_sink, mut ws_source) = ws_stream.split();

            loop {
                tokio::select! {
                    // Forward client messages to the WebSocket
                    msg = client_rx.recv() => {
                        match msg {
                            Some(client_msg) => {
                                let json = match serde_json::to_string(&client_msg) {
                                    Ok(j) => j,
                                    Err(_) => continue,
                                };
                                if ws_sink.send(Message::Text(json.into())).await.is_err() {
                                    break; // Connection lost, reconnect
                                }
                            }
                            None => return, // Client channel closed, shut down
                        }
                    }
                    // Forward WebSocket messages to the client
                    msg = ws_source.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                                    if server_tx.send(server_msg).is_err() {
                                        return; // Receiver dropped, shut down
                                    }
                                }
                            }
                            Some(Ok(Message::Ping(data))) => {
                                let _ = ws_sink.send(Message::Pong(data)).await;
                            }
                            Some(Ok(Message::Close(_))) | None => break, // Disconnected
                            Some(Err(_)) => break, // Error, reconnect
                            _ => {} // Ignore other message types
                        }
                    }
                }
            }

            let _ = status_tx.send(ConnectionStatus::Disconnected);
            tokio::time::sleep(retry_delay).await;
            retry_delay = (retry_delay * 2).min(max_retry_delay);
        }
    });

    (client_tx, server_rx, status_rx)
}

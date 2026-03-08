//! TestWsClient — WebSocket test client for E2E testing.

use std::time::Duration;

use anyhow::{Result, bail};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// A WebSocket test client for interacting with the Arawn server.
pub struct TestWsClient {
    ws: WsStream,
    /// Default timeout for receive operations.
    pub recv_timeout: Duration,
}

/// Server message received over WebSocket (subset for test assertions).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsServerMessage {
    AuthResult {
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    SessionCreated {
        session_id: String,
    },
    ChatChunk {
        session_id: String,
        chunk: String,
        done: bool,
    },
    ToolStart {
        session_id: String,
        tool_id: String,
        tool_name: String,
    },
    ToolOutput {
        session_id: String,
        tool_id: String,
        content: String,
    },
    ToolEnd {
        session_id: String,
        tool_id: String,
        success: bool,
    },
    Error {
        code: String,
        message: String,
    },
    Pong,
    SubscribeAck {
        session_id: String,
        owner: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reconnect_token: Option<String>,
    },
    ContextInfo {
        session_id: String,
        current_tokens: usize,
        max_tokens: usize,
        percent: u8,
        status: String,
    },
    #[serde(other)]
    Unknown,
}

impl TestWsClient {
    /// Connect to a WebSocket server at the given URL.
    pub async fn connect(url: &str) -> Result<Self> {
        let (ws, _) = tokio_tungstenite::connect_async(url).await?;
        Ok(Self {
            ws,
            recv_timeout: Duration::from_secs(10),
        })
    }

    /// Set the default receive timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.recv_timeout = timeout;
        self
    }

    /// Send an authentication message.
    pub async fn authenticate(&mut self, token: &str) -> Result<WsServerMessage> {
        let msg = serde_json::json!({
            "type": "auth",
            "token": token
        });
        self.send_json(&msg).await?;
        self.recv().await
    }

    /// Subscribe to a session.
    pub async fn subscribe(&mut self, session_id: &str) -> Result<WsServerMessage> {
        let msg = serde_json::json!({
            "type": "subscribe",
            "session_id": session_id
        });
        self.send_json(&msg).await?;
        self.recv().await
    }

    /// Subscribe with a reconnect token.
    pub async fn subscribe_with_token(
        &mut self,
        session_id: &str,
        token: &str,
    ) -> Result<WsServerMessage> {
        let msg = serde_json::json!({
            "type": "subscribe",
            "session_id": session_id,
            "reconnect_token": token
        });
        self.send_json(&msg).await?;
        self.recv().await
    }

    /// Send a chat message and collect all response messages until done.
    pub async fn chat(
        &mut self,
        message: &str,
        session_id: Option<&str>,
        workstream_id: Option<&str>,
    ) -> Result<Vec<WsServerMessage>> {
        let mut msg = serde_json::json!({
            "type": "chat",
            "message": message,
        });
        if let Some(sid) = session_id {
            msg["session_id"] = serde_json::Value::String(sid.to_string());
        }
        if let Some(ws_id) = workstream_id {
            msg["workstream_id"] = serde_json::Value::String(ws_id.to_string());
        }
        self.send_json(&msg).await?;
        self.collect_until_done().await
    }

    /// Send a ping and wait for pong.
    pub async fn ping(&mut self) -> Result<()> {
        let msg = serde_json::json!({"type": "ping"});
        self.send_json(&msg).await?;
        let resp = self.recv().await?;
        match resp {
            WsServerMessage::Pong => Ok(()),
            other => bail!("Expected Pong, got: {:?}", other),
        }
    }

    /// Send a cancel message.
    pub async fn cancel(&mut self, session_id: &str) -> Result<()> {
        let msg = serde_json::json!({
            "type": "cancel",
            "session_id": session_id
        });
        self.send_json(&msg).await
    }

    /// Send a raw JSON message.
    pub async fn send_json(&mut self, msg: &serde_json::Value) -> Result<()> {
        let text = serde_json::to_string(msg)?;
        self.ws.send(Message::Text(text.into())).await?;
        Ok(())
    }

    /// Receive and parse the next server message.
    pub async fn recv(&mut self) -> Result<WsServerMessage> {
        let result = timeout(self.recv_timeout, async {
            loop {
                match self.ws.next().await {
                    Some(Ok(Message::Text(text))) => {
                        let msg: WsServerMessage = serde_json::from_str(&text)?;
                        return Ok(msg);
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = self.ws.send(Message::Pong(data)).await;
                        continue;
                    }
                    Some(Ok(Message::Pong(_))) => continue,
                    Some(Ok(Message::Close(_))) => bail!("WebSocket closed"),
                    Some(Err(e)) => bail!("WebSocket error: {}", e),
                    None => bail!("WebSocket stream ended"),
                    _ => continue,
                }
            }
        })
        .await;

        match result {
            Ok(msg) => msg,
            Err(_) => bail!("Timeout waiting for WebSocket message"),
        }
    }

    /// Try to receive a message with a short timeout. Returns None on timeout.
    pub async fn try_recv(&mut self, wait: Duration) -> Result<Option<WsServerMessage>> {
        match timeout(wait, self.recv()).await {
            Ok(Ok(msg)) => Ok(Some(msg)),
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(None),
        }
    }

    /// Collect messages until a ChatChunk with `done: true` is received.
    async fn collect_until_done(&mut self) -> Result<Vec<WsServerMessage>> {
        let mut messages = Vec::new();
        let deadline = Duration::from_secs(30);

        let result = timeout(deadline, async {
            loop {
                let msg = self.recv().await?;
                let is_done = matches!(&msg, WsServerMessage::ChatChunk { done: true, .. });
                let is_error = matches!(&msg, WsServerMessage::Error { .. });
                messages.push(msg);
                if is_done || is_error {
                    break;
                }
            }
            Ok::<(), anyhow::Error>(())
        })
        .await;

        match result {
            Ok(Ok(())) => Ok(messages),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Return what we collected so far
                Ok(messages)
            }
        }
    }

    /// Close the WebSocket connection.
    pub async fn close(mut self) -> Result<()> {
        self.ws.close(None).await?;
        Ok(())
    }
}

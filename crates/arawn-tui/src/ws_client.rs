use std::sync::atomic::{AtomicU64, Ordering};

use arawn_service::{EngineEvent, SessionInfo, WorkstreamInfo};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, warn};

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

/// A WebSocket connection to the Arawn server.
pub struct WsClient {
    write: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        WsMessage,
    >,
    pub read: futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
}

impl WsClient {
    pub async fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Append auth token from ~/.arawn/server.token if available
        let authed_url = if let Some(token) = Self::read_server_token() {
            let separator = if url.contains('?') { "&" } else { "?" };
            format!("{url}{separator}token={token}")
        } else {
            url.to_string()
        };

        debug!(url = %authed_url, "ws_client connecting");
        let (ws_stream, resp) = connect_async(&authed_url).await?;
        debug!(status = ?resp.status(), "ws_client connected");
        let (write, read) = ws_stream.split();
        Ok(Self { write, read })
    }

    /// Read the server auth token from {data_dir}/server.token.
    /// Checks ARAWN_DATA_DIR env var first, falls back to ~/.arawn.
    fn read_server_token() -> Option<String> {
        let data_dir = std::env::var("ARAWN_DATA_DIR")
            .ok()
            .or_else(|| {
                std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .ok()
                    .map(|h| format!("{h}/.arawn"))
            })?;
        let token_path = std::path::PathBuf::from(data_dir).join("server.token");
        std::fs::read_to_string(token_path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub async fn send_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let id = next_id();
        let request = json!({
            "id": id,
            "method": method,
            "params": params,
        });
        debug!(id, method, "ws_client sending request");
        self.write
            .send(WsMessage::Text(request.to_string().into()))
            .await?;
        debug!(id, method, "ws_client request sent");
        Ok(id)
    }

    pub async fn list_workstreams(
        &mut self,
    ) -> Result<Vec<WorkstreamInfo>, Box<dyn std::error::Error>> {
        self.send_request("list_workstreams", json!({})).await?;
        let resp = self.read_response().await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(serde_json::from_value(result.clone())?)
    }

    pub async fn list_workflows(
        &mut self,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        self.send_request("list_workflows", json!({})).await?;
        let resp = self.read_response().await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(serde_json::from_value(result.clone())?)
    }

    pub async fn get_permission_mode(
        &mut self,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.send_request("get_permission_mode", json!({})).await?;
        let resp = self.read_response().await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(result["mode"].as_str().unwrap_or("default").to_string())
    }

    pub async fn set_permission_mode(
        &mut self,
        mode: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.send_request("set_permission_mode", json!({"mode": mode})).await?;
        let resp = self.read_response().await?;
        if let Some(err) = resp.get("error") {
            return Err(err["message"].as_str().unwrap_or("unknown error").into());
        }
        let result = resp.get("result").ok_or("no result")?;
        Ok(result["mode"].as_str().unwrap_or(mode).to_string())
    }

    pub async fn list_sessions(
        &mut self,
        ws_id: Option<uuid::Uuid>,
    ) -> Result<Vec<SessionInfo>, Box<dyn std::error::Error>> {
        let params = match ws_id {
            Some(id) => json!({"workstream_id": id.to_string()}),
            None => json!({"workstream_id": null}),
        };
        self.send_request("list_sessions", params).await?;
        let resp = self.read_response().await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(serde_json::from_value(result.clone())?)
    }

    pub async fn create_session(
        &mut self,
        ws_id: Option<uuid::Uuid>,
    ) -> Result<SessionInfo, Box<dyn std::error::Error>> {
        let params = match ws_id {
            Some(id) => json!({"workstream_id": id.to_string()}),
            None => json!({"workstream_id": null}),
        };
        self.send_request("create_session", params).await?;
        let resp = self.read_response().await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(serde_json::from_value(result.clone())?)
    }

    pub async fn load_session(
        &mut self,
        session_id: uuid::Uuid,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        self.send_request("load_session", json!({"session_id": session_id.to_string()})).await?;
        let resp = self.read_response().await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(result.clone())
    }

    pub async fn send_message(
        &mut self,
        session_id: uuid::Uuid,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.send_request(
            "send_message",
            json!({
                "session_id": session_id.to_string(),
                "content": content,
            }),
        )
        .await?;
        // The ack response comes first, then streaming events
        let _ack = self.read_response().await?;
        Ok(())
    }

    /// Read the next JSON response from the server (public for sidebar).
    pub async fn read_response_raw(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        self.read_response().await
    }

    /// Read the next JSON response from the server.
    async fn read_response(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        while let Some(msg) = self.read.next().await {
            let msg = msg?;
            match &msg {
                WsMessage::Text(text) => {
                    let value: Value = serde_json::from_str(text)?;
                    let id = value.get("id").and_then(|v| v.as_u64());
                    let has_error = value.get("error").is_some();
                    debug!(id = ?id, has_error, "ws_client recv response");
                    return Ok(value);
                }
                WsMessage::Close(frame) => {
                    warn!(frame = ?frame, "ws_client recv close frame");
                }
                WsMessage::Ping(_) => {
                    debug!("ws_client recv ping");
                }
                _ => {
                    debug!(kind = ?std::mem::discriminant(&msg), "ws_client recv non-text frame");
                }
            }
        }
        warn!("ws_client connection closed unexpectedly");
        Err("connection closed".into())
    }
}

/// Parse a WS message as an EngineEvent. Returns None if it's not an event (e.g., a response).
pub fn parse_engine_event(text: &str) -> Option<EngineEvent> {
    let value: Value = serde_json::from_str(text).ok()?;

    // EngineEvent uses tagged serde: {"event": "...", "data": {...}}
    if value.get("event").is_some() {
        let event_type = value.get("event").and_then(|e| e.as_str()).unwrap_or("?").to_string();
        match serde_json::from_value::<EngineEvent>(value) {
            Ok(event) => {
                debug!(event_type = %event_type, "parsed engine event");
                Some(event)
            }
            Err(e) => {
                warn!(event_type = %event_type, error = %e, "failed to deserialize engine event");
                None
            }
        }
    } else {
        debug!("ws message is not an engine event (no 'event' field)");
        None
    }
}

/// Convert an EngineEvent into App state updates. Returns messages to add and streaming text to append.
pub enum EventUpdate {
    AppendStreamingText(String),
    AddToolCall {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    AddToolResult {
        id: String,
        content: String,
        is_error: bool,
    },
    Complete(String),
    Error(String),
    Warning(String),
    Compaction(usize),
    /// Token usage update.
    Usage { input_tokens: u64, output_tokens: u64 },
    /// Server requests user input via a modal dialog.
    UserInputRequest {
        request_id: String,
        title: String,
        subtitle: Option<String>,
        options: Vec<arawn_service::ModalPromptOption>,
    },
    /// Server signals that the client should render now.
    Flush,
}

pub fn engine_event_to_update(event: EngineEvent) -> EventUpdate {
    match event {
        EngineEvent::StreamingText { text } => EventUpdate::AppendStreamingText(text),
        EngineEvent::ToolCallStart { id, name, input } => {
            EventUpdate::AddToolCall { id, name, input }
        }
        EngineEvent::ToolCallResult {
            id,
            content,
            is_error,
        } => EventUpdate::AddToolResult {
            id,
            content,
            is_error,
        },
        EngineEvent::Complete { final_text } => EventUpdate::Complete(final_text),
        EngineEvent::Error { message } => EventUpdate::Error(message),
        EngineEvent::Warning { message } => EventUpdate::Warning(message),
        EngineEvent::CompactionOccurred {
            messages_summarized,
        } => EventUpdate::Compaction(messages_summarized),
        EngineEvent::Usage { input_tokens, output_tokens } => EventUpdate::Usage { input_tokens, output_tokens },
        EngineEvent::UserInputRequest { request_id, title, subtitle, options } => {
            EventUpdate::UserInputRequest { request_id, title, subtitle, options }
        }
        EngineEvent::Flush => EventUpdate::Flush,
    }
}

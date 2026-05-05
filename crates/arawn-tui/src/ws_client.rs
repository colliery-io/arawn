use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use arawn_service::{EngineEvent, SessionInfo, WorkstreamInfo};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, warn};

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

/// A frame from the reader task. Either a raw text payload (engine events,
/// system notices, anything not tagged with an `id`) or an end-of-stream
/// signal. Synchronous JSON-RPC responses (which carry `id`) are routed
/// directly to their `send_request` caller via the pending-oneshot map and
/// never appear here.
#[derive(Debug)]
pub enum WsEvent {
    Text(String),
    Closed,
    Error(String),
}

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>;

/// A WebSocket connection to the Arawn server.
///
/// The reader half runs on its own task — it never shares the main task's
/// time budget with rendering. Synchronous request/response is implemented
/// by registering a oneshot keyed on the request id; the reader fans
/// responses to the right oneshot and pushes everything else into the
/// event channel returned by [`Self::events_take`].
pub struct WsClient {
    write: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        WsMessage,
    >,
    pending: Pending,
    events: Option<mpsc::Receiver<WsEvent>>,
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

        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let (events_tx, events_rx) = mpsc::channel::<WsEvent>(256);
        spawn_reader(read, Arc::clone(&pending), events_tx);

        Ok(Self {
            write,
            pending,
            events: Some(events_rx),
        })
    }

    /// Take ownership of the event receiver. The main loop selects on this
    /// instead of polling the read half directly. Returns `None` if already
    /// taken (only meaningful to call once after `connect`).
    pub fn events_take(&mut self) -> Option<mpsc::Receiver<WsEvent>> {
        self.events.take()
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

    /// Send a request and await its response via the pending-oneshot map.
    /// Replaces the old send-then-read-next pattern, which conflicted with
    /// the dedicated reader task owning the stream.
    pub async fn request_response(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let id = next_id();
        let request = json!({"id": id, "method": method, "params": params});
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        debug!(id, method, "ws_client sending request");
        if let Err(e) = self
            .write
            .send(WsMessage::Text(request.to_string().into()))
            .await
        {
            self.pending.lock().await.remove(&id);
            return Err(e.into());
        }
        match rx.await {
            Ok(value) => Ok(value),
            Err(_) => Err("connection closed before response".into()),
        }
    }

    pub async fn list_workstreams(
        &mut self,
    ) -> Result<Vec<WorkstreamInfo>, Box<dyn std::error::Error>> {
        let resp = self.request_response("list_workstreams", json!({})).await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(serde_json::from_value(result.clone())?)
    }

    pub async fn list_workflows(
        &mut self,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        let resp = self.request_response("list_workflows", json!({})).await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(serde_json::from_value(result.clone())?)
    }

    /// Fetch server runtime capabilities. Used on connect to surface
    /// degraded-feature warnings (e.g. embeddings unavailable → memory
    /// falls back to keyword search).
    pub async fn get_capabilities(
        &mut self,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let resp = self.request_response("get_capabilities", json!({})).await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(result.clone())
    }

    /// Fetch permission rules + recent audit. Backs the `/permissions` TUI command.
    pub async fn get_permissions_status(
        &mut self,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let resp = self.request_response("get_permissions_status", json!({})).await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(result.clone())
    }

    /// List registered integrations and their connection state. Backs `/integrations`.
    pub async fn list_integrations(
        &mut self,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        let resp = self.request_response("list_integrations", json!({})).await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(serde_json::from_value(result.clone())?)
    }

    /// Begin the OAuth flow for a service. Returns the auth URL the user
    /// should open. The rest of the flow runs server-side; completion is
    /// announced via a `ServerNotice` with category="integration".
    pub async fn start_oauth_flow(
        &mut self,
        service: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let resp = self
            .request_response("start_oauth_flow", json!({"service": service}))
            .await?;
        if let Some(err) = resp.get("error") {
            return Err(err["message"].as_str().unwrap_or("unknown error").into());
        }
        let result = resp.get("result").ok_or("no result")?;
        Ok(result.clone())
    }

    /// Drop stored credentials for a service.
    pub async fn disconnect_integration(
        &mut self,
        service: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let resp = self
            .request_response("disconnect_integration", json!({"service": service}))
            .await?;
        if let Some(err) = resp.get("error") {
            return Err(err["message"].as_str().unwrap_or("unknown error").into());
        }
        Ok(())
    }

    pub async fn get_permission_mode(
        &mut self,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let resp = self.request_response("get_permission_mode", json!({})).await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(result["mode"].as_str().unwrap_or("default").to_string())
    }

    pub async fn set_permission_mode(
        &mut self,
        mode: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let resp = self
            .request_response("set_permission_mode", json!({"mode": mode}))
            .await?;
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
        let resp = self.request_response("list_sessions", params).await?;
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
        let resp = self.request_response("create_session", params).await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(serde_json::from_value(result.clone())?)
    }

    pub async fn load_session(
        &mut self,
        session_id: uuid::Uuid,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let resp = self
            .request_response(
                "load_session",
                json!({"session_id": session_id.to_string()}),
            )
            .await?;
        let result = resp.get("result").ok_or("no result")?;
        Ok(result.clone())
    }

    /// Rewind a session back to before the Nth user message. Returns the
    /// full truncated session detail (same shape as `load_session`). Used
    /// by the "branch from a prior prompt" flow.
    pub async fn truncate_session_at_user_message(
        &mut self,
        session_id: uuid::Uuid,
        user_message_index: usize,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let resp = self
            .request_response(
                "truncate_session_at_user_message",
                json!({
                    "session_id": session_id.to_string(),
                    "user_message_index": user_message_index,
                }),
            )
            .await?;
        if let Some(err) = resp.get("error") {
            return Err(err["message"].as_str().unwrap_or("unknown error").into());
        }
        let result = resp.get("result").ok_or("no result")?;
        Ok(result.clone())
    }

    pub async fn send_message(
        &mut self,
        session_id: uuid::Uuid,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _ack = self
            .request_response(
                "send_message",
                json!({
                    "session_id": session_id.to_string(),
                    "content": content,
                }),
            )
            .await?;
        Ok(())
    }
}

/// Spawn the reader task. Owns the read half of the stream forever; on
/// disconnect, sends `WsEvent::Closed` (or `Error`) once and exits.
fn spawn_reader(
    mut read: futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    pending: Pending,
    events_tx: mpsc::Sender<WsEvent>,
) {
    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    let _ = events_tx.send(WsEvent::Error(e.to_string())).await;
                    return;
                }
            };
            match msg {
                WsMessage::Text(text) => {
                    let s = text.to_string();
                    // Try to route as a JSON-RPC response first (has `id`).
                    if let Ok(value) = serde_json::from_str::<Value>(&s)
                        && let Some(id) = value.get("id").and_then(|v| v.as_u64())
                    {
                        let mut map = pending.lock().await;
                        if let Some(tx) = map.remove(&id) {
                            let _ = tx.send(value);
                            continue;
                        }
                        // Else fall through to event channel — the engine
                        // sometimes carries `id` on streaming frames too.
                    }
                    if events_tx.send(WsEvent::Text(s)).await.is_err() {
                        return; // receiver dropped — main loop exited
                    }
                }
                WsMessage::Close(frame) => {
                    warn!(frame = ?frame, "ws reader: close frame");
                    let _ = events_tx.send(WsEvent::Closed).await;
                    return;
                }
                WsMessage::Ping(_) => debug!("ws reader: ping"),
                _ => debug!(kind = ?std::mem::discriminant(&msg), "ws reader: non-text"),
            }
        }
        let _ = events_tx.send(WsEvent::Closed).await;
    });
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

/// Parse a server-wide notice (plugin/config hot-reload) from a raw WS text
/// message. Returns `None` if the message isn't a `SystemNotice` envelope.
/// Distinct from `parse_engine_event` because notices are server-scoped, not
/// per-conversation-turn.
pub fn parse_system_notice(text: &str) -> Option<arawn_service::ServerNotice> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    if v.get("event")?.as_str()? != "SystemNotice" {
        return None;
    }
    serde_json::from_value(v.get("data")?.clone()).ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    // T-0199: parse_system_notice must accept the on-the-wire shape and
    // reject unrelated messages without panicking.

    #[test]
    fn parses_well_formed_system_notice() {
        let raw = serde_json::json!({
            "event": "SystemNotice",
            "data": {
                "level": "info",
                "category": "plugin_reload",
                "message": "plugins reloaded: 2 plugin(s), 5 skill(s), 1 agent(s)",
                "timestamp": "2026-05-02T15:00:00Z"
            }
        })
        .to_string();
        let notice = parse_system_notice(&raw).expect("expected SystemNotice");
        assert_eq!(notice.level, "info");
        assert_eq!(notice.category, "plugin_reload");
        assert!(notice.message.contains("reloaded"));
    }

    #[test]
    fn rejects_engine_event_envelope() {
        let raw = serde_json::json!({
            "event": "StreamingText",
            "data": {"text": "hi"}
        })
        .to_string();
        assert!(parse_system_notice(&raw).is_none());
    }

    #[test]
    fn rejects_response_envelope() {
        let raw = serde_json::json!({"id": 1, "result": {"ok": true}}).to_string();
        assert!(parse_system_notice(&raw).is_none());
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(parse_system_notice("not json at all").is_none());
        assert!(parse_system_notice("").is_none());
    }
}

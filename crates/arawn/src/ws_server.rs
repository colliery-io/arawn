use std::sync::Arc;

use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message as WsMessage, WebSocket},
    },
    response::IntoResponse,
    routing::get,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tracing::{debug, info, warn};

use arawn_service::ArawnService;

use crate::local_service::LocalService;

/// JSON-RPC style request from client.
#[derive(Debug, Deserialize)]
struct Request {
    id: u64,
    method: String,
    #[serde(default)]
    params: Value,
}

/// JSON-RPC style response to client.
#[derive(Debug, Serialize)]
struct Response {
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorBody>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

impl Response {
    fn success(id: u64, result: Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: u64, code: &str, message: String) -> Self {
        Self {
            id,
            result: None,
            error: Some(ErrorBody {
                code: code.to_string(),
                message,
            }),
        }
    }
}

/// Start the WebSocket server on the given port.
pub async fn run_server(service: LocalService, port: u16) -> anyhow::Result<()> {
    let service = Arc::new(service);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(service);

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    info!(addr = %addr, "WebSocket server listening");
    eprintln!("Arawn server listening on ws://{addr}/ws");
    eprintln!("Press Ctrl-C to stop.");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(service): State<Arc<LocalService>>,
) -> impl IntoResponse {
    debug!("ws_handler: upgrade request received");
    ws.on_upgrade(move |socket| handle_connection(socket, service))
}

/// Handle a single WebSocket connection. Public for integration tests.
pub async fn handle_connection_public(socket: WebSocket, service: Arc<LocalService>) {
    handle_connection(socket, service).await;
}

async fn handle_connection(socket: WebSocket, service: Arc<LocalService>) {
    let (mut sender, mut receiver) = socket.split();
    info!("WebSocket client connected");

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(WsMessage::Text(text)) => text,
            Ok(WsMessage::Close(frame)) => {
                info!(frame = ?frame, "WebSocket client disconnected (close frame)");
                break;
            }
            Ok(WsMessage::Ping(_)) => {
                debug!("recv ping");
                continue;
            }
            Ok(WsMessage::Pong(_)) => {
                debug!("recv pong");
                continue;
            }
            Ok(_) => continue,
            Err(e) => {
                warn!(error = %e, "WebSocket receive error");
                break;
            }
        };

        let request: Request = match serde_json::from_str(&msg) {
            Ok(r) => r,
            Err(e) => {
                warn!(raw = %msg, error = %e, "failed to parse request JSON");
                let resp = Response::error(0, "parse_error", format!("Invalid JSON: {e}"));
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
                continue;
            }
        };

        let id = request.id;
        debug!(id, method = %request.method, "dispatching RPC");

        match request.method.as_str() {
            "list_workstreams" => {
                let resp = match service.list_workstreams().await {
                    Ok(ws) => {
                        debug!(id, count = ws.len(), "list_workstreams ok");
                        Response::success(id, serde_json::to_value(&ws).unwrap())
                    }
                    Err(e) => {
                        warn!(id, error = %e, "list_workstreams failed");
                        Response::error(id, "service_error", e.to_string())
                    }
                };
                if sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await
                    .is_err()
                {
                    warn!(id, "send failed, client gone");
                    break;
                }
            }

            "create_workstream" => {
                let name = request
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let root_dir = request
                    .params
                    .get("root_dir")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .into();
                debug!(id, %name, "create_workstream");
                let resp = match service.create_workstream(name, root_dir).await {
                    Ok(ws) => {
                        debug!(id, ws_id = %ws.id, "create_workstream ok");
                        Response::success(id, serde_json::to_value(&ws).unwrap())
                    }
                    Err(e) => {
                        warn!(id, error = %e, "create_workstream failed");
                        Response::error(id, "service_error", e.to_string())
                    }
                };
                if sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await
                    .is_err()
                {
                    warn!(id, "send failed, client gone");
                    break;
                }
            }

            "list_sessions" => {
                let ws_id = request
                    .params
                    .get("workstream_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                debug!(id, ws_id = ?ws_id, "list_sessions");
                let resp = match service.list_sessions(ws_id).await {
                    Ok(sessions) => {
                        debug!(id, count = sessions.len(), "list_sessions ok");
                        Response::success(id, serde_json::to_value(&sessions).unwrap())
                    }
                    Err(e) => {
                        warn!(id, error = %e, "list_sessions failed");
                        Response::error(id, "service_error", e.to_string())
                    }
                };
                if sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await
                    .is_err()
                {
                    warn!(id, "send failed, client gone");
                    break;
                }
            }

            "create_session" => {
                let ws_id = request
                    .params
                    .get("workstream_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                debug!(id, ws_id = ?ws_id, "create_session");
                let resp = match service.create_session(ws_id).await {
                    Ok(session) => {
                        debug!(id, session_id = %session.id, "create_session ok");
                        Response::success(id, serde_json::to_value(&session).unwrap())
                    }
                    Err(e) => {
                        warn!(id, error = %e, "create_session failed");
                        Response::error(id, "service_error", e.to_string())
                    }
                };
                if sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await
                    .is_err()
                {
                    warn!(id, "send failed, client gone");
                    break;
                }
            }

            "load_session" => {
                let session_id = request
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                debug!(id, session_id = ?session_id, "load_session");
                let resp = match session_id {
                    Some(sid) => match service.load_session(sid).await {
                        Ok(detail) => {
                            debug!(id, messages = detail.messages.len(), "load_session ok");
                            Response::success(id, serde_json::to_value(&detail).unwrap())
                        }
                        Err(e) => {
                            warn!(id, error = %e, "load_session failed");
                            Response::error(id, "service_error", e.to_string())
                        }
                    },
                    None => {
                        warn!(id, "load_session missing session_id");
                        Response::error(id, "invalid_params", "missing session_id".into())
                    }
                };
                if sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await
                    .is_err()
                {
                    warn!(id, "send failed, client gone");
                    break;
                }
            }

            "send_message" => {
                let session_id = request
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                let content = request
                    .params
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let session_id = match session_id {
                    Some(sid) => sid,
                    None => {
                        warn!(id, "send_message missing session_id");
                        let resp =
                            Response::error(id, "invalid_params", "missing session_id".into());
                        let _ = sender
                            .send(WsMessage::Text(
                                serde_json::to_string(&resp).unwrap().into(),
                            ))
                            .await;
                        continue;
                    }
                };

                debug!(id, %session_id, content_len = content.len(), "send_message starting stream");

                // Ack the request
                let ack = Response::success(id, json!({"status": "streaming"}));
                if sender
                    .send(WsMessage::Text(serde_json::to_string(&ack).unwrap().into()))
                    .await
                    .is_err()
                {
                    warn!(id, "send_message ack failed, client gone");
                    break;
                }
                debug!(id, "send_message ack sent");

                // Stream events while also handling incoming messages
                // (needed for user_input_response during modal prompts)
                match service.send_message(session_id, content).await {
                    Ok(mut stream) => {
                        let mut event_count: u64 = 0;
                        loop {
                            tokio::select! {
                                // Engine events → forward to client
                                event = stream.next() => {
                                    match event {
                                        Some(engine_event) => {
                                            event_count += 1;
                                            debug!(id, event_count, event = ?std::mem::discriminant(&engine_event), "forwarding engine event");
                                            let event_json = serde_json::to_string(&engine_event).unwrap();
                                            if sender
                                                .send(WsMessage::Text(event_json.into()))
                                                .await
                                                .is_err()
                                            {
                                                warn!(id, event_count, "stream send failed, client disconnected mid-stream");
                                                break;
                                            }
                                        }
                                        None => {
                                            debug!(id, event_count, "engine stream ended");
                                            break;
                                        }
                                    }
                                }
                                // Client messages (e.g., user_input_response) → handle inline
                                msg = receiver.next() => {
                                    match msg {
                                        Some(Ok(WsMessage::Text(text))) => {
                                            if let Ok(req) = serde_json::from_str::<Request>(&text) {
                                                debug!(id, inline_method = %req.method, "recv inline message during stream");
                                                if req.method == "user_input_response" {
                                                    let request_id = req.params
                                                        .get("request_id")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("")
                                                        .to_string();
                                                    let selected_index = req.params
                                                        .get("selected_index")
                                                        .and_then(|v| v.as_u64())
                                                        .map(|n| n as usize);

                                                    debug!(id, %request_id, ?selected_index, "routing modal response (inline)");
                                                    {
                                                        let mut pending = service.pending_modals.lock().unwrap();
                                                        if let Some(tx) = pending.remove(&request_id) {
                                                            let _ = tx.send(selected_index);
                                                            debug!(id, %request_id, "modal response delivered (inline)");
                                                        } else {
                                                            warn!(id, %request_id, "no pending modal found (inline)");
                                                        }
                                                    }

                                                    let resp = Response::success(req.id, json!({"status": "delivered"}));
                                                    let _ = sender
                                                        .send(WsMessage::Text(
                                                            serde_json::to_string(&resp).unwrap().into(),
                                                        ))
                                                        .await;
                                                }
                                                // Other methods during streaming are ignored
                                            }
                                        }
                                        Some(Ok(WsMessage::Close(frame))) => {
                                            info!(id, frame = ?frame, "client closed during stream");
                                            break;
                                        }
                                        None => {
                                            info!(id, "client connection dropped during stream");
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        debug!(id, event_count, "send_message stream complete");
                    }
                    Err(e) => {
                        warn!(id, error = %e, "send_message service error");
                        let resp = Response::error(id, "service_error", e.to_string());
                        let _ = sender
                            .send(WsMessage::Text(
                                serde_json::to_string(&resp).unwrap().into(),
                            ))
                            .await;
                    }
                }
            }

            "user_input_response" => {
                let request_id = request
                    .params
                    .get("request_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let selected_index = request
                    .params
                    .get("selected_index")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);

                debug!(id, %request_id, ?selected_index, "routing modal response");

                // Route the response to the waiting tool
                let resolved = {
                    let mut pending = service.pending_modals.lock().unwrap();
                    if let Some(tx) = pending.remove(&request_id) {
                        let _ = tx.send(selected_index);
                        true
                    } else {
                        false
                    }
                };

                if resolved {
                    debug!(id, %request_id, "modal response delivered");
                } else {
                    warn!(id, %request_id, "no pending modal found");
                }

                let resp = if resolved {
                    Response::success(id, json!({"status": "delivered"}))
                } else {
                    Response::error(id, "not_found", format!("no pending modal for {request_id}"))
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "cancel" => {
                let session_id = request
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                debug!(id, session_id = ?session_id, "cancel");
                let resp = match session_id {
                    Some(sid) => match service.cancel(sid).await {
                        Ok(()) => {
                            debug!(id, "cancel ok");
                            Response::success(id, json!({"status": "cancelled"}))
                        }
                        Err(e) => {
                            warn!(id, error = %e, "cancel failed");
                            Response::error(id, "service_error", e.to_string())
                        }
                    },
                    None => {
                        warn!(id, "cancel missing session_id");
                        Response::error(id, "invalid_params", "missing session_id".into())
                    }
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "query_inventory" => {
                let kind = request
                    .params
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                debug!(id, %kind, "query_inventory");
                let result = service.query_inventory(kind);
                let resp = Response::success(id, result);
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "remember_fact" => {
                debug!(id, "remember_fact");
                let text = request
                    .params
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let result = service.remember_fact(text);
                let resp = Response::success(id, result);
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "memory_summary" => {
                debug!(id, "memory_summary");
                let result = service.memory_summary();
                let resp = Response::success(id, result);
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "forget_entity" => {
                debug!(id, "forget_entity");
                let query = request
                    .params
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let result = service.forget_entity(query);
                let resp = Response::success(id, result);
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "list_available_commands" => {
                debug!(id, "list_available_commands");
                let result = service.list_available_commands();
                let resp = Response::success(id, result);
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            unknown => {
                warn!(id, method = %unknown, "unknown RPC method");
                let resp =
                    Response::error(id, "method_not_found", format!("unknown method: {unknown}"));
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }
        }
    }
    info!("WebSocket connection handler exiting");
}

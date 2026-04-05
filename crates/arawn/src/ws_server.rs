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
use tracing::{info, warn};

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
            Ok(WsMessage::Close(_)) => {
                info!("WebSocket client disconnected");
                break;
            }
            Ok(_) => continue, // Ignore binary, ping, pong
            Err(e) => {
                warn!(error = %e, "WebSocket receive error");
                break;
            }
        };

        let request: Request = match serde_json::from_str(&msg) {
            Ok(r) => r,
            Err(e) => {
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

        match request.method.as_str() {
            "list_workstreams" => {
                let resp = match service.list_workstreams().await {
                    Ok(ws) => Response::success(id, serde_json::to_value(&ws).unwrap()),
                    Err(e) => Response::error(id, "service_error", e.to_string()),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
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
                let resp = match service.create_workstream(name, root_dir).await {
                    Ok(ws) => Response::success(id, serde_json::to_value(&ws).unwrap()),
                    Err(e) => Response::error(id, "service_error", e.to_string()),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "list_sessions" => {
                let ws_id = request
                    .params
                    .get("workstream_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                let resp = match service.list_sessions(ws_id).await {
                    Ok(sessions) => Response::success(id, serde_json::to_value(&sessions).unwrap()),
                    Err(e) => Response::error(id, "service_error", e.to_string()),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "create_session" => {
                let ws_id = request
                    .params
                    .get("workstream_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                let resp = match service.create_session(ws_id).await {
                    Ok(session) => Response::success(id, serde_json::to_value(&session).unwrap()),
                    Err(e) => Response::error(id, "service_error", e.to_string()),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "load_session" => {
                let session_id = request
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                let resp = match session_id {
                    Some(sid) => match service.load_session(sid).await {
                        Ok(detail) => Response::success(id, serde_json::to_value(&detail).unwrap()),
                        Err(e) => Response::error(id, "service_error", e.to_string()),
                    },
                    None => Response::error(id, "invalid_params", "missing session_id".into()),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
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

                // Ack the request
                let ack = Response::success(id, json!({"status": "streaming"}));
                let _ = sender
                    .send(WsMessage::Text(serde_json::to_string(&ack).unwrap().into()))
                    .await;

                // Stream events while also handling incoming messages
                // (needed for user_input_response during modal prompts)
                match service.send_message(session_id, content).await {
                    Ok(mut stream) => {
                        loop {
                            tokio::select! {
                                // Engine events → forward to client
                                event = stream.next() => {
                                    match event {
                                        Some(engine_event) => {
                                            let event_json = serde_json::to_string(&engine_event).unwrap();
                                            if sender
                                                .send(WsMessage::Text(event_json.into()))
                                                .await
                                                .is_err()
                                            {
                                                break; // Client disconnected
                                            }
                                        }
                                        None => break, // Stream ended
                                    }
                                }
                                // Client messages (e.g., user_input_response) → handle inline
                                msg = receiver.next() => {
                                    match msg {
                                        Some(Ok(WsMessage::Text(text))) => {
                                            if let Ok(req) = serde_json::from_str::<Request>(&text) {
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

                                                    {
                                                        let mut pending = service.pending_modals.lock().unwrap();
                                                        if let Some(tx) = pending.remove(&request_id) {
                                                            let _ = tx.send(selected_index);
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
                                        Some(Ok(WsMessage::Close(_))) | None => break,
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
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
                let resp = match session_id {
                    Some(sid) => match service.cancel(sid).await {
                        Ok(()) => Response::success(id, json!({"status": "cancelled"})),
                        Err(e) => Response::error(id, "service_error", e.to_string()),
                    },
                    None => Response::error(id, "invalid_params", "missing session_id".into()),
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
                let result = service.query_inventory(kind);
                let resp = Response::success(id, result);
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "remember_fact" => {
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
                let result = service.memory_summary();
                let resp = Response::success(id, result);
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "forget_entity" => {
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
                let result = service.list_available_commands();
                let resp = Response::success(id, result);
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            unknown => {
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
}

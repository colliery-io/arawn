use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message as WsMessage, WebSocket},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tracing::{debug, info, warn};

use arawn_service::ArawnService;

use crate::local_service::LocalService;

/// Protocol version reported by the `hello` handshake.
const PROTOCOL_VERSION: &str = "0.1.0";

/// Canonical RPC method names (returned by `hello`).
const RPC_METHODS: &[&str] = &[
    "hello",
    "list_workstreams",
    "create_workstream",
    "list_sessions",
    "create_session",
    "load_session",
    "truncate_session_at_user_message",
    "send_message",
    "cancel",
    "user_input_response",
    "promote_session",
    "query_inventory",
    "list_commands",
    "list_workflows",
    "store_memory",
    "get_memory_summary",
    "delete_memory",
    "get_permission_mode",
    "set_permission_mode",
    "get_capabilities",
    "get_permissions_status",
    "list_integrations",
    "start_oauth_flow",
    "disconnect_integration",
    "feed_register",
    "feed_list",
];

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
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
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
                details: None,
            }),
        }
    }

    /// Build an error response from a [`ServiceError`]. Preserves the
    /// stable code, the display message, and — for variants that wrap
    /// typed sources — a structured `details` object with the inner
    /// error's `kind` so clients can dispatch on it.
    fn from_service_error(id: u64, e: &arawn_service::ServiceError) -> Self {
        Self {
            id,
            result: None,
            error: Some(ErrorBody {
                code: e.error_code().to_string(),
                message: e.to_string(),
                details: e.details(),
            }),
        }
    }
}

/// Shared app state for the WebSocket server.
#[derive(Clone)]
struct AppState {
    service: Arc<LocalService>,
    /// Authentication token required for WebSocket connections.
    /// If None, authentication is disabled (dev mode).
    auth_token: Option<String>,
}

/// Generate a random auth token for WebSocket connections.
fn generate_auth_token() -> String {
    // Use two UUIDs concatenated for sufficient entropy (256 bits)
    format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
}

/// Write the auth token to {data_dir}/server.token for clients to read.
fn write_token_file(data_dir: &std::path::Path, token: &str) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(data_dir)?;
    let token_path = data_dir.join("server.token");
    std::fs::write(&token_path, token)?;
    Ok(token_path)
}

/// Read the auth token from {data_dir}/server.token.
/// Falls back to ARAWN_DATA_DIR env var, then ~/.arawn.
pub fn read_token_file() -> Option<String> {
    let data_dir = std::env::var("ARAWN_DATA_DIR")
        .ok()
        .or_else(|| {
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .ok()
                .map(|h| format!("{h}/.arawn"))
        })?;
    let token_path = std::path::PathBuf::from(data_dir).join("server.token");
    std::fs::read_to_string(token_path).ok().map(|s| s.trim().to_string())
}

/// Start the WebSocket server on the given port.
pub async fn run_server(service: LocalService, port: u16) -> anyhow::Result<()> {
    let data_dir = service.data_dir.clone();

    // Generate auth token and write to disk for clients
    let auth_token = generate_auth_token();
    match write_token_file(&data_dir, &auth_token) {
        Ok(path) => info!(?path, "auth token written"),
        Err(e) => warn!("failed to write auth token file: {e} — authentication disabled"),
    }

    let state = AppState {
        service: Arc::new(service),
        auth_token: Some(auth_token),
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/decision", post(decision_handler))
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    info!(addr = %addr, "WebSocket server listening");
    eprintln!("Arawn server listening on ws://{addr}/ws");
    eprintln!("Press Ctrl-C to stop.");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("server shutting down gracefully");
    // Clean up token file on shutdown
    let token_path = data_dir.join("server.token");
    let _ = std::fs::remove_file(token_path);
    Ok(())
}

/// Wait for a shutdown signal (Ctrl-C / SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("received Ctrl-C, initiating shutdown"),
        _ = terminate => info!("received SIGTERM, initiating shutdown"),
    }
}

/// HTTP endpoint for workflow decision tasks.
/// Decision tasks in cloacina pipelines call POST /api/decision to run
/// the QueryEngine and get agent-powered responses.
async fn decision_handler(
    State(AppState { service, .. }): State<AppState>,
    Json(req): Json<arawn_workflow::agent_executor::DecisionRequest>,
) -> impl IntoResponse {
    let decision_service = arawn_workflow::DecisionService::new(
        service.shared_store(),
        service.shared_llm(),
        service.shared_registry(),
        service.engine_config().clone(),
    );

    match decision_service.execute(req).await {
        Ok(resp) => (StatusCode::OK, Json(json!(resp))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Query parameters for WebSocket connection.
#[derive(Debug, Deserialize)]
struct WsQueryParams {
    token: Option<String>,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    debug!("ws_handler: upgrade request received");

    // Validate auth token if configured
    if let Some(ref expected_token) = state.auth_token {
        match params.token {
            Some(ref provided) if provided == expected_token => {
                debug!("ws_handler: auth token valid");
            }
            Some(_) => {
                warn!("ws_handler: invalid auth token");
                return (StatusCode::UNAUTHORIZED, "Invalid auth token").into_response();
            }
            None => {
                warn!("ws_handler: missing auth token");
                return (StatusCode::UNAUTHORIZED, "Auth token required. Connect with /ws?token=<token>").into_response();
            }
        }
    }

    ws.on_upgrade(move |socket| handle_connection(socket, state.service))
        .into_response()
}

/// Handle a single WebSocket connection. Public for integration tests.
pub async fn handle_connection_public(socket: WebSocket, service: Arc<LocalService>) {
    handle_connection(socket, service).await;
}

async fn handle_connection(socket: WebSocket, service: Arc<LocalService>) {
    let (mut sender, mut receiver) = socket.split();
    info!("WebSocket client connected");

    // Subscribe to server-wide notices (plugin/config hot-reload) for the
    // lifetime of this connection. Notices arrive as JSON messages with
    // `event: "SystemNotice"` interleaved with normal RPC responses.
    let mut notice_rx = service.subscribe_notices();

    loop {
        let msg = tokio::select! {
            biased;
            // Forward server-wide notices to this client. RecvError::Lagged
            // means the broadcast buffer overflowed — log and keep going;
            // missing a hot-reload notice is not fatal.
            notice = notice_rx.recv() => {
                match notice {
                    Ok(notice) => {
                        let payload = serde_json::json!({
                            "event": "SystemNotice",
                            "data": notice,
                        });
                        let _ = sender
                            .send(WsMessage::Text(payload.to_string().into()))
                            .await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "client lagged on notice broadcast");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // Channel closed — service is shutting down. Fall
                        // through; the next socket recv will see Close.
                    }
                }
                continue;
            }
            recv = receiver.next() => recv,
        };
        let msg = match msg {
            Some(Ok(WsMessage::Text(text))) => text,
            Some(Ok(WsMessage::Close(frame))) => {
                info!(frame = ?frame, "WebSocket client disconnected (close frame)");
                break;
            }
            Some(Ok(WsMessage::Ping(_))) => {
                debug!("recv ping");
                continue;
            }
            Some(Ok(WsMessage::Pong(_))) => {
                debug!("recv pong");
                continue;
            }
            Some(Ok(_)) => continue,
            Some(Err(e)) => {
                warn!(error = %e, "WebSocket receive error");
                break;
            }
            None => {
                info!("WebSocket receiver closed");
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
            "hello" => {
                debug!(id, "hello handshake");
                let resp = Response::success(
                    id,
                    json!({
                        "version": PROTOCOL_VERSION,
                        "methods": RPC_METHODS,
                    }),
                );
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "list_workstreams" => {
                let resp = match service.list_workstreams().await {
                    Ok(ws) => {
                        debug!(id, count = ws.len(), "list_workstreams ok");
                        Response::success(id, serde_json::to_value(&ws).unwrap())
                    }
                    Err(e) => {
                        warn!(id, error = %e, "list_workstreams failed");
                        Response::from_service_error(id, &e)
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
                // Default root_dir to {data_dir}/workstreams/{name}/ when not provided
                let root_dir: std::path::PathBuf = request
                    .params
                    .get("root_dir")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| service.data_dir.join("workstreams").join(&name));
                debug!(id, %name, root_dir = %root_dir.display(), "create_workstream");
                let resp = match service.create_workstream(name, root_dir).await {
                    Ok(ws) => {
                        debug!(id, ws_id = %ws.id, "create_workstream ok");
                        Response::success(id, serde_json::to_value(&ws).unwrap())
                    }
                    Err(e) => {
                        warn!(id, error = %e, "create_workstream failed");
                        Response::from_service_error(id, &e)
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
                        Response::from_service_error(id, &e)
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
                        Response::from_service_error(id, &e)
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
                            Response::from_service_error(id, &e)
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

            "truncate_session_at_user_message" => {
                let session_id = request
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                let user_message_index = request
                    .params
                    .get("user_message_index")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);
                debug!(
                    id,
                    session_id = ?session_id,
                    user_message_index,
                    "truncate_session_at_user_message"
                );
                let resp = match (session_id, user_message_index) {
                    (Some(sid), Some(idx)) => match service
                        .truncate_session_at_user_message(sid, idx)
                        .await
                    {
                        Ok(detail) => {
                            debug!(
                                id,
                                messages = detail.messages.len(),
                                "truncate_session_at_user_message ok"
                            );
                            Response::success(id, serde_json::to_value(&detail).unwrap())
                        }
                        Err(e) => {
                            warn!(id, error = %e, "truncate_session_at_user_message failed");
                            Response::from_service_error(id, &e)
                        }
                    },
                    _ => Response::error(
                        id,
                        "invalid_params",
                        "missing session_id or user_message_index".into(),
                    ),
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
                                                    let resp = match service
                                                        .resolve_user_input(&request_id, selected_index)
                                                        .await
                                                    {
                                                        Ok(()) => {
                                                            debug!(id, %request_id, "modal response delivered (inline)");
                                                            Response::success(req.id, json!({"status": "delivered"}))
                                                        }
                                                        Err(e) => {
                                                            warn!(id, %request_id, "no pending modal found (inline)");
                                                            Response::from_service_error(req.id, &e)
                                                        }
                                                    };
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
                        let resp = Response::from_service_error(id, &e);
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

                let resp = match service
                    .resolve_user_input(&request_id, selected_index)
                    .await
                {
                    Ok(()) => {
                        debug!(id, %request_id, "modal response delivered");
                        Response::success(id, json!({"status": "delivered"}))
                    }
                    Err(e) => {
                        warn!(id, %request_id, "no pending modal found");
                        Response::from_service_error(id, &e)
                    }
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
                            Response::from_service_error(id, &e)
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

            "promote_session" => {
                let session_id = request
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                let workstream_name = request
                    .params
                    .get("workstream_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                debug!(id, session_id = ?session_id, %workstream_name, "promote_session");

                let resp = match session_id {
                    Some(sid) => match service.promote_session(sid, &workstream_name).await {
                        Ok(result) => {
                            debug!(id, "promote_session ok");
                            Response::success(id, serde_json::to_value(&result).unwrap())
                        }
                        Err(e) => {
                            warn!(id, error = %e, "promote_session failed");
                            Response::from_service_error(id, &e)
                        }
                    },
                    None => {
                        warn!(id, "promote_session missing session_id");
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
                let resp = match service.query_inventory(kind).await {
                    Ok(items) => Response::success(id, serde_json::to_value(&items).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "store_memory" => {
                debug!(id, "store_memory");
                let text = request
                    .params
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let resp = match service.remember_fact(text).await {
                    Ok(result) => Response::success(id, serde_json::to_value(&result).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "get_memory_summary" => {
                debug!(id, "get_memory_summary");
                let resp = match service.memory_summary().await {
                    Ok(summary) => Response::success(id, serde_json::to_value(&summary).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "delete_memory" => {
                debug!(id, "delete_memory");
                let query = request
                    .params
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let resp = match service.forget_entity(query).await {
                    Ok(result) => Response::success(id, serde_json::to_value(&result).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "list_commands" => {
                debug!(id, "list_commands");
                let resp = match service.list_available_commands().await {
                    Ok(commands) => Response::success(id, serde_json::to_value(&commands).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "list_workflows" => {
                debug!(id, "list_workflows");
                let resp = match service.list_workflows().await {
                    Ok(workflows) => {
                        Response::success(id, serde_json::to_value(&workflows).unwrap())
                    }
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "get_permission_mode" => {
                debug!(id, "get_permission_mode");
                let resp = match service.get_permission_mode().await {
                    Ok(info) => Response::success(id, serde_json::to_value(&info).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "set_permission_mode" => {
                let mode_str = request
                    .params
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                debug!(id, %mode_str, "set_permission_mode");
                let resp = match service.set_permission_mode(mode_str).await {
                    Ok(info) => Response::success(id, serde_json::to_value(&info).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "get_capabilities" => {
                debug!(id, "get_capabilities");
                let resp = match service.get_capabilities().await {
                    Ok(caps) => Response::success(id, serde_json::to_value(&caps).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "get_permissions_status" => {
                debug!(id, "get_permissions_status");
                let resp = match service.get_permissions_status().await {
                    Ok(status) => Response::success(id, serde_json::to_value(&status).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "list_integrations" => {
                debug!(id, "list_integrations");
                let resp = match service.list_integrations().await {
                    Ok(list) => Response::success(id, serde_json::to_value(&list).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "start_oauth_flow" => {
                let svc = request
                    .params
                    .get("service")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                debug!(id, %svc, "start_oauth_flow");
                let resp = match service.start_oauth_flow(svc).await {
                    Ok(started) => Response::success(id, serde_json::to_value(&started).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "disconnect_integration" => {
                let svc = request
                    .params
                    .get("service")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                debug!(id, %svc, "disconnect_integration");
                let resp = match service.disconnect_integration(svc).await {
                    Ok(()) => Response::success(id, serde_json::json!({"ok": true})),
                    Err(e) => Response::from_service_error(id, &e),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "feed_register" => {
                debug!(id, "feed_register");
                let resp = match serde_json::from_value::<arawn_service::FeedRegisterSpec>(
                    request.params.clone(),
                ) {
                    Ok(spec) => match service.feed_register(spec).await {
                        Ok(dto) => {
                            Response::success(id, serde_json::to_value(&dto).unwrap())
                        }
                        Err(e) => Response::from_service_error(id, &e),
                    },
                    Err(e) => Response::error(
                        id,
                        "invalid_params",
                        format!("feed_register params: {e}"),
                    ),
                };
                let _ = sender
                    .send(WsMessage::Text(
                        serde_json::to_string(&resp).unwrap().into(),
                    ))
                    .await;
            }

            "feed_list" => {
                debug!(id, "feed_list");
                let resp = match service.feed_list().await {
                    Ok(list) => Response::success(id, serde_json::to_value(&list).unwrap()),
                    Err(e) => Response::from_service_error(id, &e),
                };
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

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_service::ServiceError;

    /// Typed Storage error should round-trip through the wire payload with
    /// structured `details` (kind tag) alongside the code + message.
    #[test]
    fn from_service_error_preserves_structured_detail_for_typed_variants() {
        let storage_err = arawn_storage::StorageError::NotFound("session 42".into());
        let service_err: ServiceError = storage_err.into();
        let resp = Response::from_service_error(7, &service_err);

        let body = resp.error.unwrap();
        assert_eq!(body.code, "storage_error");
        assert!(body.message.contains("not found"), "message: {}", body.message);
        let details = body.details.unwrap();
        assert_eq!(details["kind"], "not_found");
    }

    /// String-only variants (NotFound, InvalidOperation, Internal) keep
    /// code + message but have no `details` payload — the serialized wire
    /// form must omit the field entirely, not emit null.
    #[test]
    fn from_service_error_omits_details_for_string_only_variants() {
        let service_err = ServiceError::NotFound("session 99".into());
        let resp = Response::from_service_error(1, &service_err);

        let body = resp.error.as_ref().unwrap();
        assert_eq!(body.code, "not_found");
        assert!(body.details.is_none());

        // Wire check: `details: null` should not appear in the JSON.
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("\"details\":null"), "json: {json}");
    }

    /// Engine errors surface a `kind` that identifies the inner variant —
    /// tool_not_found here — so clients can dispatch without string-parsing
    /// the message.
    #[test]
    fn from_service_error_preserves_engine_error_kind() {
        let engine_err = arawn_engine::EngineError::ToolNotFound("make_coffee".into());
        let service_err: ServiceError = engine_err.into();
        let resp = Response::from_service_error(3, &service_err);

        let body = resp.error.unwrap();
        assert_eq!(body.code, "engine_error");
        let details = body.details.unwrap();
        assert_eq!(details["kind"], "tool_not_found");
    }
}

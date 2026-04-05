//! Integration tests for the WebSocket server.
//! Spins up the server on a random port, connects a WS client, exercises the JSON protocol.

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

use arawn_core::Workstream;
use arawn_engine::{FileReadTool, QueryEngineConfig, ShellTool, ThinkTool, ToolRegistry};
use arawn_llm::{MockLlmClient, MockResponse};
use arawn_storage::Store;

/// Spin up a test server on a random port and return the WS URL.
async fn start_test_server(mock_responses: Vec<MockResponse>) -> (String, TempDir) {
    let tmp = TempDir::new().unwrap();
    let store = Store::open(tmp.path()).unwrap();

    // Create scratch workstream
    let ws = Workstream::scratch(tmp.path());
    store.create_workstream(&ws).unwrap();

    let llm = Arc::new(MockLlmClient::new(mock_responses));
    let registry = Arc::new(ToolRegistry::new());
    registry.register(Box::new(ThinkTool));
    registry.register(Box::new(ShellTool::default()));
    registry.register(Box::new(FileReadTool));

    let config = QueryEngineConfig {
        system_prompt: "Test".into(),
        ..Default::default()
    };

    let service =
        arawn_bin::LocalService::new(store, tmp.path().to_path_buf(), llm, registry, config);

    // Bind to random port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{addr}/ws");

    let service = Arc::new(service);
    let app = axum::Router::new()
        .route(
            "/ws",
            axum::routing::get(
                move |ws: axum::extract::WebSocketUpgrade,
                      state: axum::extract::State<Arc<arawn_bin::LocalService>>| async move {
                    ws.on_upgrade(move |socket| {
                        arawn_bin::ws_server::handle_connection_public(socket, state.0)
                    })
                },
            ),
        )
        .with_state(service);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    (url, tmp)
}

/// Helper: send a JSON request and get the response.
async fn send_request(
    write: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        TungsteniteMessage,
    >,
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    request: Value,
) -> Value {
    write
        .send(TungsteniteMessage::Text(request.to_string().into()))
        .await
        .unwrap();

    let msg = read.next().await.unwrap().unwrap();
    let text = msg.into_text().unwrap();
    serde_json::from_str(&text).unwrap()
}

#[tokio::test]
async fn list_workstreams_returns_scratch() {
    let (url, _tmp) = start_test_server(vec![]).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let resp = send_request(
        &mut write,
        &mut read,
        json!({"id": 1, "method": "list_workstreams"}),
    )
    .await;

    assert!(resp["result"].is_array());
    let workstreams = resp["result"].as_array().unwrap();
    assert!(!workstreams.is_empty());
    assert_eq!(workstreams[0]["name"], "scratch");
}

#[tokio::test]
async fn create_and_load_session() {
    let (url, _tmp) = start_test_server(vec![]).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Create session
    let resp = send_request(
        &mut write,
        &mut read,
        json!({"id": 1, "method": "create_session", "params": {"workstream_id": null}}),
    )
    .await;

    assert!(resp["result"]["id"].is_string());
    let session_id = resp["result"]["id"].as_str().unwrap();

    // Load session
    let resp = send_request(
        &mut write,
        &mut read,
        json!({"id": 2, "method": "load_session", "params": {"session_id": session_id}}),
    )
    .await;

    assert_eq!(resp["result"]["id"], session_id);
    assert!(resp["result"]["messages"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn unknown_method_returns_error() {
    let (url, _tmp) = start_test_server(vec![]).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let resp = send_request(
        &mut write,
        &mut read,
        json!({"id": 1, "method": "nonexistent"}),
    )
    .await;

    assert!(resp["error"].is_object());
    assert_eq!(resp["error"]["code"], "method_not_found");
}

#[tokio::test]
async fn malformed_json_returns_error() {
    let (url, _tmp) = start_test_server(vec![]).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    write
        .send(TungsteniteMessage::Text("not valid json".into()))
        .await
        .unwrap();

    let msg = read.next().await.unwrap().unwrap();
    let resp: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
    assert!(resp["error"].is_object());
    assert_eq!(resp["error"]["code"], "parse_error");
}

// --- Streaming tests ---

#[tokio::test]
async fn send_message_streams_complete_event() {
    let (url, _tmp) = start_test_server(vec![MockResponse::text("Streamed reply")]).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Create session first
    let resp = send_request(
        &mut write,
        &mut read,
        json!({"id": 1, "method": "create_session", "params": {"workstream_id": null}}),
    )
    .await;
    let session_id = resp["result"]["id"].as_str().unwrap().to_string();

    // Send message
    write
        .send(TungsteniteMessage::Text(
            json!({
                "id": 2,
                "method": "send_message",
                "params": {"session_id": session_id, "content": "hello"}
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    // Read ack response
    let _ack = read.next().await.unwrap().unwrap();

    // Read streaming events until Complete
    let mut got_complete = false;
    let mut final_text = String::new();
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(Ok(msg)) = read.next().await {
            if let TungsteniteMessage::Text(text) = msg {
                let value: Value = serde_json::from_str(&text).unwrap();
                if value.get("event").and_then(|e| e.as_str()) == Some("Complete") {
                    got_complete = true;
                    final_text = value["data"]["final_text"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    break;
                }
            }
        }
    });

    timeout.await.expect("timed out waiting for Complete event");
    assert!(got_complete);
    assert_eq!(final_text, "Streamed reply");
}

#[tokio::test]
async fn send_message_with_tool_call_streams_events() {
    let (url, _tmp) = start_test_server(vec![
        MockResponse::tool_call("c1", "think", r#"{"thought":"reasoning"}"#),
        MockResponse::text("Done."),
    ])
    .await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Create session
    let resp = send_request(
        &mut write,
        &mut read,
        json!({"id": 1, "method": "create_session", "params": {"workstream_id": null}}),
    )
    .await;
    let session_id = resp["result"]["id"].as_str().unwrap().to_string();

    // Send message
    write
        .send(TungsteniteMessage::Text(
            json!({
                "id": 2,
                "method": "send_message",
                "params": {"session_id": session_id, "content": "think about this"}
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let _ack = read.next().await.unwrap().unwrap();

    // Collect events
    let mut events: Vec<String> = Vec::new();
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(Ok(msg)) = read.next().await {
            if let TungsteniteMessage::Text(text) = msg {
                let value: Value = serde_json::from_str(&text).unwrap();
                if let Some(event_type) = value.get("event").and_then(|e| e.as_str()) {
                    events.push(event_type.to_string());
                    if event_type == "Complete" {
                        break;
                    }
                }
            }
        }
    });

    timeout.await.expect("timed out waiting for events");

    assert!(
        events.contains(&"ToolCallStart".to_string()),
        "should have ToolCallStart, got: {events:?}"
    );
    assert!(
        events.contains(&"ToolCallResult".to_string()),
        "should have ToolCallResult, got: {events:?}"
    );
    assert!(
        events.contains(&"Complete".to_string()),
        "should have Complete, got: {events:?}"
    );

    // Complete should be last
    assert_eq!(events.last().unwrap(), "Complete");
}

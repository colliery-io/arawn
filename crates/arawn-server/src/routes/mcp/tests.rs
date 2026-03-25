use super::*;
use crate::auth::auth_middleware;
use crate::config::ServerConfig;
use arawn_domain::McpManager;
use arawn_domain::{Agent, ToolRegistry};
use arawn_llm::MockBackend;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::{delete, get, post},
};
use tower::ServiceExt;

fn create_test_state_with_mcp() -> AppState {
    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();

    AppState::new(agent, ServerConfig::new(Some("test-token".to_string())))
        .with_mcp_manager(McpManager::new())
}

fn create_test_state_without_mcp() -> AppState {
    let backend = MockBackend::with_text("Test");
    let agent = Agent::builder()
        .with_backend(backend)
        .with_tools(ToolRegistry::new())
        .build()
        .unwrap();

    AppState::new(agent, ServerConfig::new(Some("test-token".to_string())))
}

fn create_test_router(state: AppState) -> Router {
    Router::new()
        .route(
            "/mcp/servers",
            post(add_server_handler).get(list_servers_handler),
        )
        .route("/mcp/servers/{name}", delete(remove_server_handler))
        .route("/mcp/servers/{name}/tools", get(list_server_tools_handler))
        .route("/mcp/servers/{name}/connect", post(connect_server_handler))
        .route(
            "/mcp/servers/{name}/disconnect",
            post(disconnect_server_handler),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

#[tokio::test]
async fn test_list_servers_empty() {
    let state = create_test_state_with_mcp();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/mcp/servers")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: ListServersResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.total, 0);
    assert_eq!(result.connected, 0);
    assert!(result.servers.is_empty());
}

#[tokio::test]
async fn test_list_servers_mcp_disabled() {
    let state = create_test_state_without_mcp();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/mcp/servers")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_add_server_missing_name() {
    let state = create_test_state_with_mcp();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"name": "", "command": "some-cmd"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_add_server_missing_command() {
    let state = create_test_state_with_mcp();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"name": "test", "command": "", "connect": false}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_add_server_success_no_connect() {
    let state = create_test_state_with_mcp();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"name": "test-server", "command": "some-cmd", "connect": false}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: AddServerResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.name, "test-server");
    assert!(!result.connected);
    assert!(result.error.is_none());
}

#[tokio::test]
async fn test_add_server_duplicate() {
    let state = create_test_state_with_mcp();

    // Add a server first
    {
        let mut manager = state.mcp_manager().as_ref().unwrap().write().await;
        manager.add_server(McpServerConfig::new("existing", "cmd"));
    }

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"name": "existing", "command": "another-cmd", "connect": false}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_remove_server_not_found() {
    let state = create_test_state_with_mcp();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/mcp/servers/nonexistent")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_remove_server_success() {
    let state = create_test_state_with_mcp();

    // Add a server first
    {
        let mut manager = state.mcp_manager().as_ref().unwrap().write().await;
        manager.add_server(McpServerConfig::new("to-remove", "cmd"));
    }

    let app = create_test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/mcp/servers/to-remove")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify removed
    let manager = state.mcp_manager().as_ref().unwrap().read().await;
    assert!(!manager.has_server("to-remove"));
}

#[tokio::test]
async fn test_list_server_tools_not_found() {
    let state = create_test_state_with_mcp();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/mcp/servers/nonexistent/tools")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_server_tools_not_connected() {
    let state = create_test_state_with_mcp();

    // Add a server but don't connect
    {
        let mut manager = state.mcp_manager().as_ref().unwrap().write().await;
        manager.add_server(McpServerConfig::new("not-connected", "cmd"));
    }

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/mcp/servers/not-connected/tools")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_connect_server_not_found() {
    let state = create_test_state_with_mcp();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers/nonexistent/connect")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_disconnect_server_not_found() {
    let state = create_test_state_with_mcp();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers/nonexistent/disconnect")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_disconnect_server_success() {
    let state = create_test_state_with_mcp();

    // Add a server (not connected but it should still succeed)
    {
        let mut manager = state.mcp_manager().as_ref().unwrap().write().await;
        manager.add_server(McpServerConfig::new("to-disconnect", "cmd"));
    }

    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers/to-disconnect/disconnect")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_add_http_server() {
    let state = create_test_state_with_mcp();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{
                            "name": "http-server",
                            "transport": "http",
                            "url": "http://localhost:8080/mcp",
                            "connect": false
                        }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: AddServerResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.name, "http-server");
    assert!(!result.connected);
}

#[tokio::test]
async fn test_add_http_server_missing_url() {
    let state = create_test_state_with_mcp();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp/servers")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{
                            "name": "http-server",
                            "transport": "http",
                            "connect": false
                        }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

use arawn_client::ArawnClient;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_client(uri: &str) -> ArawnClient {
    ArawnClient::builder()
        .base_url(uri)
        .auth_token("test-token")
        .build()
        .unwrap()
}

// ─────────────────────────────────────────────────────────────────────────────
// Health API
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_health_check() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
            "version": "0.1.0"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.health().check().await.unwrap();

    assert_eq!(resp.status, "ok");
    assert_eq!(resp.version.as_deref(), Some("0.1.0"));
}

#[tokio::test]
async fn test_health_check_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.health().check().await.unwrap_err();

    assert!(matches!(err, arawn_client::Error::Api { status: 500, .. }));
}

#[tokio::test]
async fn test_is_healthy_true() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
            "version": "0.1.0"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    assert!(client.health().is_healthy().await);
}

#[tokio::test]
async fn test_is_healthy_false() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    assert!(!client.health().is_healthy().await);
}

// ─────────────────────────────────────────────────────────────────────────────
// Config API
// ─────────────────────────────────────────────────────────────────────────────

fn config_json() -> serde_json::Value {
    serde_json::json!({
        "version": "0.1.0",
        "api_version": "v1",
        "features": {
            "workstreams_enabled": true,
            "memory_enabled": true,
            "mcp_enabled": false,
            "rate_limiting": false,
            "request_logging": true
        },
        "limits": {
            "max_concurrent_requests": 100
        },
        "bind_address": "127.0.0.1:8080",
        "auth_required": true
    })
}

#[tokio::test]
async fn test_config_get() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(config_json()))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.config().get().await.unwrap();

    assert_eq!(resp.version, "0.1.0");
    assert_eq!(resp.api_version.as_deref(), Some("v1"));
    assert!(resp.features.workstreams_enabled);
    assert!(resp.features.memory_enabled);
    assert!(!resp.features.mcp_enabled);
    assert!(!resp.features.rate_limiting);
    assert!(resp.features.request_logging);
    assert_eq!(resp.limits.max_concurrent_requests, Some(100));
    assert_eq!(resp.bind_address, "127.0.0.1:8080");
    assert!(resp.auth_required);
}

#[tokio::test]
async fn test_config_get_auth_header() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/config"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(config_json()))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.config().get().await.unwrap();

    assert_eq!(resp.version, "0.1.0");
}

#[tokio::test]
async fn test_config_get_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/config"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "code": "not_found",
            "message": "Resource not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.config().get().await.unwrap_err();

    assert!(matches!(err, arawn_client::Error::NotFound(_)));
    assert!(err.is_not_found());
}

#[tokio::test]
async fn test_config_get_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/config"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.config().get().await.unwrap_err();

    assert!(err.is_server_error());
    assert!(matches!(
        err,
        arawn_client::Error::Api {
            status: 500,
            ref code,
            ref message,
        } if code == "internal_error" && message == "Internal server error"
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// Agents API
// ─────────────────────────────────────────────────────────────────────────────

fn agent_detail_json() -> serde_json::Value {
    serde_json::json!({
        "id": "main",
        "name": "default",
        "is_default": true,
        "tools": [
            {
                "name": "shell",
                "description": "Run commands"
            }
        ],
        "capabilities": {
            "streaming": true,
            "tool_use": true,
            "max_context_length": 100000
        }
    })
}

fn agents_list_json() -> serde_json::Value {
    serde_json::json!({
        "agents": [
            {
                "id": "main",
                "name": "default",
                "is_default": true,
                "tool_count": 1
            }
        ],
        "total": 1
    })
}

#[tokio::test]
async fn test_agents_list() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/agents"))
        .respond_with(ResponseTemplate::new(200).set_body_json(agents_list_json()))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.agents().list().await.unwrap();

    assert_eq!(resp.total, 1);
    assert_eq!(resp.agents.len(), 1);
    assert_eq!(resp.agents[0].id, "main");
    assert_eq!(resp.agents[0].name, "default");
    assert!(resp.agents[0].is_default);
    assert_eq!(resp.agents[0].tool_count, 1);
}

#[tokio::test]
async fn test_agents_list_auth_header() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/agents"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(agents_list_json()))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.agents().list().await.unwrap();

    assert_eq!(resp.total, 1);
}

#[tokio::test]
async fn test_agents_get() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/agents/main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(agent_detail_json()))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.agents().get("main").await.unwrap();

    assert_eq!(resp.id, "main");
    assert_eq!(resp.name, "default");
    assert!(resp.is_default);
    assert_eq!(resp.tools.len(), 1);
    assert_eq!(resp.tools[0].name, "shell");
    assert_eq!(resp.tools[0].description, "Run commands");
    assert!(resp.capabilities.streaming);
    assert!(resp.capabilities.tool_use);
    assert_eq!(resp.capabilities.max_context_length, Some(100000));
}

#[tokio::test]
async fn test_agents_get_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/agents/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "code": "not_found",
            "message": "Resource not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.agents().get("nonexistent").await.unwrap_err();

    assert!(matches!(err, arawn_client::Error::NotFound(_)));
    assert!(err.is_not_found());
}

#[tokio::test]
async fn test_agents_main() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/agents/main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(agent_detail_json()))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.agents().main().await.unwrap();

    assert_eq!(resp.id, "main");
    assert_eq!(resp.name, "default");
    assert!(resp.is_default);
}

#[tokio::test]
async fn test_agents_list_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/agents"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "code": "internal_error",
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.agents().list().await.unwrap_err();

    assert!(err.is_server_error());
    assert!(matches!(err, arawn_client::Error::Api { status: 500, .. }));
}

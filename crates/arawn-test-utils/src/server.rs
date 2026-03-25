//! TestServer — configurable test server harness.
//!
//! Extracted from `arawn-server/tests/common/mod.rs` and generalized
//! with a builder pattern for flexible configuration.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use reqwest::Client;
use tempfile::TempDir;
use tokio::task::JoinHandle;
use tokio::time::timeout;

use arawn_domain::{Agent, MemoryStore, ToolRegistry};
use arawn_llm::{
    CompletionResponse, ContentBlock, LlmBackend, MockBackend, MockResponse, StopReason, Usage,
};
use arawn_server::{AppState, Server, ServerConfig};

use crate::StreamingMockBackend;

/// A test server that runs in the background with configurable options.
pub struct TestServer {
    /// The server's bound address.
    pub addr: SocketAddr,
    /// The auth token (if configured).
    pub token: Option<String>,
    /// HTTP client configured for this server.
    pub client: Client,
    /// Handle to the server task.
    _handle: JoinHandle<()>,
    /// Temporary directory for test data.
    pub temp_dir: TempDir,
}

impl TestServer {
    /// Start a test server with default configuration.
    pub async fn start() -> Result<Self> {
        TestServerBuilder::new().build().await
    }

    /// Start a test server with pre-configured text responses.
    pub async fn start_with_responses(responses: Vec<String>) -> Result<Self> {
        TestServerBuilder::new()
            .with_text_responses(responses)
            .build()
            .await
    }

    /// Create a builder for more control over server configuration.
    pub fn builder() -> TestServerBuilder {
        TestServerBuilder::new()
    }

    /// Get the base URL for the server.
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Get the WebSocket URL for the server.
    pub fn ws_url(&self) -> String {
        format!("ws://{}/ws", self.addr)
    }

    /// Get an authenticated GET request builder.
    pub fn get(&self, path: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.get(format!("{}{}", self.base_url(), path));
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        req
    }

    /// Get an authenticated POST request builder.
    pub fn post(&self, path: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.post(format!("{}{}", self.base_url(), path));
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        req
    }

    /// Get an authenticated DELETE request builder.
    pub fn delete(&self, path: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.delete(format!("{}{}", self.base_url(), path));
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        req
    }

    /// Get an authenticated PUT request builder.
    pub fn put(&self, path: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.put(format!("{}{}", self.base_url(), path));
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        req
    }

    /// Get an authenticated PATCH request builder.
    pub fn patch(&self, path: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.patch(format!("{}{}", self.base_url(), path));
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        req
    }

    /// Check if the server is healthy.
    pub async fn health(&self) -> Result<bool> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url()))
            .send()
            .await?;
        Ok(resp.status().is_success())
    }
}

/// Builder for configuring a TestServer.
pub struct TestServerBuilder {
    token: Option<String>,
    responses: Vec<MockResponse>,
    streaming_backend: Option<StreamingMockBackend>,
    /// Generic LLM backend (takes priority over streaming_backend and responses).
    generic_backend: Option<Box<dyn LlmBackend>>,
    tools: Option<ToolRegistry>,
    with_memory: bool,
    with_workstreams: bool,
    rate_limiting: bool,
    api_rpm: Option<u32>,
    trust_proxy: bool,
    request_logging: bool,
}

impl TestServerBuilder {
    /// Create a new builder with sensible defaults.
    pub fn new() -> Self {
        Self {
            token: Some("test-token".to_string()),
            api_rpm: None,
            streaming_backend: None,
            generic_backend: None,
            tools: None,
            responses: vec![MockResponse::Success(CompletionResponse::new(
                "mock_msg_0",
                "mock-model",
                vec![ContentBlock::Text {
                    text: "Test response".to_string(),
                    cache_control: None,
                }],
                StopReason::EndTurn,
                Usage::new(10, 20),
            ))],
            with_memory: true,
            with_workstreams: false,
            rate_limiting: false,
            trust_proxy: false,
            request_logging: false,
        }
    }

    /// Set the auth token. Pass `None` for no-auth (localhost) mode.
    pub fn with_auth(mut self, token: Option<&str>) -> Self {
        self.token = token.map(|t| t.to_string());
        self
    }

    /// Set text responses for the mock backend.
    pub fn with_text_responses(mut self, responses: Vec<String>) -> Self {
        self.responses = responses
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                MockResponse::Success(CompletionResponse::new(
                    format!("mock_msg_{}", i),
                    "mock-model",
                    vec![ContentBlock::Text {
                        text,
                        cache_control: None,
                    }],
                    StopReason::EndTurn,
                    Usage::new(10, 20),
                ))
            })
            .collect();
        self
    }

    /// Set raw mock responses (for tool_use, errors, etc.).
    pub fn with_mock_responses(mut self, responses: Vec<MockResponse>) -> Self {
        self.responses = responses;
        self
    }

    /// Set a streaming mock backend directly.
    pub fn with_streaming_backend(mut self, backend: StreamingMockBackend) -> Self {
        self.streaming_backend = Some(backend);
        self
    }

    /// Enable in-memory workstream manager.
    pub fn with_workstreams(mut self) -> Self {
        self.with_workstreams = true;
        self
    }

    /// Enable rate limiting.
    pub fn with_rate_limiting(mut self, enabled: bool) -> Self {
        self.rate_limiting = enabled;
        self
    }

    /// Set the API rate limit (requests per minute).
    pub fn with_api_rpm(mut self, rpm: u32) -> Self {
        self.api_rpm = Some(rpm);
        self
    }

    /// Trust proxy headers (X-Forwarded-For) for client IP extraction.
    pub fn with_trust_proxy(mut self, trust: bool) -> Self {
        self.trust_proxy = trust;
        self
    }

    /// Set a generic LLM backend (e.g., `ScriptedMockBackend`).
    ///
    /// Takes priority over `with_streaming_backend` and `with_text_responses`.
    pub fn with_backend(mut self, backend: impl LlmBackend + 'static) -> Self {
        self.generic_backend = Some(Box::new(backend));
        self
    }

    /// Set the tool registry for the agent.
    ///
    /// By default the agent has an empty `ToolRegistry`. Use this to inject
    /// mock tools so the agent's tool-execution pipeline is exercised.
    pub fn with_tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Disable the in-memory store.
    pub fn without_memory(mut self) -> Self {
        self.with_memory = false;
        self
    }

    /// Build and start the test server.
    pub async fn build(self) -> Result<TestServer> {
        let temp_dir = TempDir::new()?;
        let addr = find_available_port().await?;

        let tools = self.tools.unwrap_or_default();

        let agent = if let Some(backend) = self.generic_backend {
            Agent::builder()
                .with_shared_backend(Arc::from(backend))
                .with_tools(tools)
                .build()?
        } else if let Some(streaming) = self.streaming_backend {
            Agent::builder()
                .with_backend(streaming)
                .with_tools(tools)
                .build()?
        } else {
            let completion_responses: Vec<CompletionResponse> = self
                .responses
                .into_iter()
                .filter_map(|r| match r {
                    MockResponse::Success(resp) => Some(resp),
                    MockResponse::Error(_) => None,
                })
                .collect();

            let backend = MockBackend::new(completion_responses);

            Agent::builder()
                .with_backend(backend)
                .with_tools(tools)
                .build()?
        };

        let mut config = ServerConfig::new(self.token.clone())
            .with_bind_address(addr)
            .with_rate_limiting(self.rate_limiting)
            .with_trust_proxy(self.trust_proxy)
            .with_request_logging(self.request_logging);

        if let Some(rpm) = self.api_rpm {
            config = config.with_api_rpm(rpm);
        }

        let mut state = AppState::new(agent, config);

        if self.with_memory {
            let memory_store =
                Arc::new(MemoryStore::open_in_memory().expect("Failed to open in-memory store"));
            state.services.memory_store = Some(memory_store);
        }

        if self.with_workstreams {
            let ws_config = arawn_workstream::WorkstreamConfig {
                db_path: temp_dir.path().join("workstreams.db"),
                data_dir: temp_dir.path().join("workstreams"),
                session_timeout_minutes: 30,
            };
            if let Ok(mgr) = arawn_workstream::WorkstreamManager::new(&ws_config) {
                state = state.with_workstreams(mgr);
            }
        }

        let server = Server::from_state(state);
        let handle = tokio::spawn(async move {
            let _ = server.run_on(addr).await;
        });

        let client = Client::new();
        wait_for_server(&client, addr).await?;

        Ok(TestServer {
            addr,
            token: self.token,
            client,
            _handle: handle,
            temp_dir,
        })
    }
}

impl Default for TestServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Find an available port for the test server.
pub async fn find_available_port() -> Result<SocketAddr> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    drop(listener);
    Ok(addr)
}

/// Wait for a server to become ready by polling its health endpoint.
pub async fn wait_for_server(client: &Client, addr: SocketAddr) -> Result<()> {
    let url = format!("http://{}/health", addr);

    let result = timeout(Duration::from_secs(5), async {
        loop {
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => return Ok(()),
                _ => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => anyhow::bail!("Timeout waiting for server to start at {}", url),
    }
}

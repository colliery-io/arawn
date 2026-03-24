//! OpenAI-compatible API backend implementation.
//!
//! This module provides `OpenAiBackend` which connects to OpenAI's API
//! or any OpenAI-compatible service (Groq, Ollama, local LLMs, etc.).

use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::{Client, Response, header};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use crate::api_key::ApiKeyProvider;
use crate::backend::{ContentDelta, LlmBackend, ResponseStream, StreamEvent, with_retry};
use crate::error::{LlmError, Result};
use crate::types::{
    CompletionRequest, CompletionResponse, ContentBlock, Role, StopReason, ToolResultContent, Usage,
};

/// Default OpenAI API base URL.
const DEFAULT_OPENAI_BASE: &str = "https://api.openai.com/v1";

/// Default timeout for requests.
const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Default maximum retries for transient errors.
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default initial backoff between retries.
const DEFAULT_RETRY_BACKOFF_MS: u64 = 500;

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for the OpenAI-compatible backend.
///
/// # Examples
///
/// ```rust,ignore
/// use arawn_llm::OpenAiConfig;
///
/// // OpenAI
/// let openai = OpenAiConfig::openai("sk-...").with_model("gpt-4o");
///
/// // Groq
/// let groq = OpenAiConfig::groq("gsk-...");
///
/// // Local Ollama
/// let ollama = OpenAiConfig::ollama().with_model("llama3.1");
/// ```
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    /// API key for authentication. Supports hot-loading via `ApiKeyProvider::Dynamic`.
    pub api_key: ApiKeyProvider,

    /// Base URL for the API.
    pub base_url: String,

    /// Model to use (can be overridden per request).
    pub model: Option<String>,

    /// Request timeout.
    pub timeout: Duration,

    /// Maximum retries for transient errors.
    pub max_retries: u32,

    /// Initial backoff duration for retries.
    pub retry_backoff: Duration,

    /// Name for this backend instance.
    pub name: String,
}

impl OpenAiConfig {
    /// Create a new config for OpenAI.
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self {
            api_key: ApiKeyProvider::from_static(api_key),
            base_url: DEFAULT_OPENAI_BASE.to_string(),
            model: None,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_retries: DEFAULT_MAX_RETRIES,
            retry_backoff: Duration::from_millis(DEFAULT_RETRY_BACKOFF_MS),
            name: "openai".to_string(),
        }
    }

    /// Create a new config for Groq.
    pub fn groq(api_key: impl Into<String>) -> Self {
        Self {
            api_key: ApiKeyProvider::from_static(api_key),
            base_url: "https://api.groq.com/openai/v1".to_string(),
            model: Some("llama-3.1-70b-versatile".to_string()),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_retries: DEFAULT_MAX_RETRIES,
            retry_backoff: Duration::from_millis(DEFAULT_RETRY_BACKOFF_MS),
            name: "groq".to_string(),
        }
    }

    /// Create a new config for Ollama (local).
    pub fn ollama() -> Self {
        Self {
            api_key: ApiKeyProvider::None,
            base_url: "http://localhost:11434/v1".to_string(),
            model: None,
            timeout: Duration::from_secs(600), // Longer timeout for local inference
            max_retries: DEFAULT_MAX_RETRIES,
            retry_backoff: Duration::from_millis(DEFAULT_RETRY_BACKOFF_MS),
            name: "ollama".to_string(),
        }
    }

    /// Create config from environment for OpenAI.
    pub fn openai_from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            LlmError::Config("OPENAI_API_KEY environment variable not set".to_string())
        })?;
        Ok(Self::openai(api_key))
    }

    /// Create config from environment for Groq.
    pub fn groq_from_env() -> Result<Self> {
        let api_key = std::env::var("GROQ_API_KEY").map_err(|_| {
            LlmError::Config("GROQ_API_KEY environment variable not set".to_string())
        })?;
        Ok(Self::groq(api_key))
    }

    /// Set a custom base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the default model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the backend name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set max retries.
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set retry backoff.
    pub fn with_retry_backoff(mut self, backoff: Duration) -> Self {
        self.retry_backoff = backoff;
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// OpenAI Backend
// ─────────────────────────────────────────────────────────────────────────────

/// OpenAI-compatible API backend.
pub struct OpenAiBackend {
    client: Client,
    config: OpenAiConfig,
}

impl OpenAiBackend {
    /// Create a new OpenAI-compatible backend with the given configuration.
    pub fn new(config: OpenAiConfig) -> Result<Self> {
        let client = crate::common::build_http_client(config.timeout)?;

        Ok(Self { client, config })
    }

    /// Create an OpenAI backend from environment.
    pub fn openai_from_env() -> Result<Self> {
        Self::new(OpenAiConfig::openai_from_env()?)
    }

    /// Create a Groq backend from environment.
    pub fn groq_from_env() -> Result<Self> {
        Self::new(OpenAiConfig::groq_from_env()?)
    }

    /// Create an Ollama backend with default local settings.
    pub fn ollama() -> Result<Self> {
        Self::new(OpenAiConfig::ollama())
    }

    /// Build the chat completions endpoint URL.
    fn completions_url(&self) -> String {
        format!("{}/chat/completions", self.config.base_url)
    }

    /// Add authentication headers to a request.
    fn add_headers(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let builder = builder.header(header::CONTENT_TYPE, "application/json");

        if let Some(api_key) = self.config.api_key.resolve() {
            builder.header(header::AUTHORIZATION, format!("Bearer {}", api_key))
        } else {
            builder
        }
    }

    /// Convert our CompletionRequest to OpenAI-compatible format.
    fn to_openai_request(&self, request: &CompletionRequest) -> OpenAiChatRequest {
        let mut messages: Vec<OpenAiMessage> = Vec::new();

        // Add system message if present
        if let Some(ref system) = request.system {
            messages.push(OpenAiMessage {
                role: "system".to_string(),
                content: Some(OpenAiContent::Text(system.to_text())),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Add conversation messages
        for m in &request.messages {
            let blocks = m.content.blocks();

            // Check for tool calls in assistant messages
            let tool_calls: Vec<_> = blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::ToolUse {
                        id, name, input, ..
                    } => Some(OpenAiToolCall {
                        id: id.clone(),
                        call_type: "function".to_string(),
                        function: OpenAiFunctionCall {
                            name: name.clone(),
                            arguments: serde_json::to_string(input).unwrap_or_default(),
                        },
                    }),
                    _ => None,
                })
                .collect();

            // Check for tool results in user messages
            let tool_results: Vec<_> = blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } => {
                        let text = match content {
                            Some(ToolResultContent::Text(t)) => t.clone(),
                            Some(ToolResultContent::Blocks(blocks)) => blocks
                                .iter()
                                .filter_map(|b| {
                                    if let serde_json::Value::Object(obj) = b {
                                        obj.get("text").and_then(|v| v.as_str()).map(String::from)
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\n"),
                            None => String::new(),
                        };
                        Some((tool_use_id.clone(), text))
                    }
                    _ => None,
                })
                .collect();

            // Get text content
            let text_content: String = blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");

            if !tool_results.is_empty() {
                // Add tool results as separate "tool" role messages
                for (tool_id, result_text) in tool_results {
                    messages.push(OpenAiMessage {
                        role: "tool".to_string(),
                        content: Some(OpenAiContent::Text(result_text)),
                        tool_calls: None,
                        tool_call_id: Some(tool_id),
                    });
                }
            } else if !tool_calls.is_empty() {
                // Assistant message with tool calls
                messages.push(OpenAiMessage {
                    role: "assistant".to_string(),
                    content: if text_content.is_empty() {
                        None
                    } else {
                        Some(OpenAiContent::Text(text_content))
                    },
                    tool_calls: Some(tool_calls),
                    tool_call_id: None,
                });
            } else {
                // Regular text message
                messages.push(OpenAiMessage {
                    role: match m.role {
                        Role::User => "user".to_string(),
                        Role::Assistant => "assistant".to_string(),
                    },
                    content: Some(OpenAiContent::Text(text_content)),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
        }

        // Convert tools
        let tools: Option<Vec<OpenAiTool>> = if request.tools.is_empty() {
            None
        } else {
            Some(
                request
                    .tools
                    .iter()
                    .map(|t| OpenAiTool {
                        tool_type: "function".to_string(),
                        function: OpenAiFunction {
                            name: t.name.clone(),
                            description: Some(t.description.clone()),
                            parameters: t.input_schema.clone(),
                        },
                    })
                    .collect(),
            )
        };

        let stop = if request.stop_sequences.is_empty() {
            None
        } else {
            Some(request.stop_sequences.clone())
        };

        // Use config model if set, otherwise use request model
        let model = self
            .config
            .model
            .clone()
            .unwrap_or_else(|| request.model.clone());

        OpenAiChatRequest {
            model,
            messages,
            max_tokens: Some(request.max_tokens),
            temperature: request.temperature,
            top_p: request.top_p,
            stream: Some(request.stream),
            tools,
            stop,
        }
    }

    /// Handle a successful response.
    async fn handle_response(response: Response) -> Result<CompletionResponse> {
        if !response.status().is_success() {
            return Err(Self::handle_error_response(response).await);
        }

        let body = response.text().await?;
        let parsed: OpenAiChatResponse =
            serde_json::from_str(&body).map_err(|e| LlmError::Serialization(e.to_string()))?;

        Ok(parsed.into())
    }

    /// Handle an error response.
    async fn handle_error_response(response: Response) -> LlmError {
        let status = response.status();

        use crate::common::{ProviderErrorResponse, extract_retry_after, map_error_response, map_raw_error};

        let retry_after = extract_retry_after(response.headers());
        let body = response.text().await.unwrap_or_default();

        if let Ok(error) = serde_json::from_str::<ProviderErrorResponse>(&body) {
            map_error_response(
                status.as_u16(),
                &error.error.message,
                retry_after.as_deref(),
                true, // OpenAI backend also handles Groq-style retry
            )
        } else {
            map_raw_error(status, &body)
        }
    }
}

#[async_trait]
impl LlmBackend for OpenAiBackend {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let mut request = request;
        request.stream = false;

        let openai_request = self.to_openai_request(&request);

        tracing::debug!(
            backend = %self.config.name,
            model = %openai_request.model,
            messages = %openai_request.messages.len(),
            tools = %openai_request.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            "Sending OpenAI-compatible request"
        );

        with_retry(
            self.config.max_retries,
            self.config.retry_backoff,
            &self.config.name,
            || async {
                let response = self
                    .add_headers(self.client.post(self.completions_url()))
                    .json(&openai_request)
                    .send()
                    .await?;

                Self::handle_response(response).await
            },
        )
        .await
    }

    async fn complete_stream(&self, request: CompletionRequest) -> Result<ResponseStream> {
        let mut request = request;
        request.stream = true;

        let openai_request = self.to_openai_request(&request);

        let response = self
            .add_headers(self.client.post(self.completions_url()))
            .json(&openai_request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Self::handle_error_response(response).await);
        }

        Ok(parse_openai_sse_stream(response.bytes_stream()))
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn supports_native_tools(&self) -> bool {
        true
    }
}

/// Create a shared OpenAI-compatible backend.
pub fn create_shared_backend(config: OpenAiConfig) -> Result<Arc<dyn LlmBackend>> {
    Ok(Arc::new(OpenAiBackend::new(config)?))
}

// ─────────────────────────────────────────────────────────────────────────────
// OpenAI API Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(Debug, serde::Serialize)]
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<OpenAiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
enum OpenAiContent {
    Text(String),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct OpenAiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAiFunction,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct OpenAiFunction {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    parameters: serde_json::Value,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAiFunctionCall,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiChatResponse {
    id: String,
    choices: Vec<OpenAiChoice>,
    model: String,
    usage: Option<OpenAiUsage>,
}

impl From<OpenAiChatResponse> for CompletionResponse {
    fn from(resp: OpenAiChatResponse) -> Self {
        let choice = resp.choices.into_iter().next();

        let (content, stop_reason) = if let Some(c) = choice {
            let mut blocks = Vec::new();

            // Add text content if present
            if let Some(text) = c.message.content
                && !text.is_empty()
            {
                blocks.push(ContentBlock::Text {
                    text,
                    cache_control: None,
                });
            }

            // Add tool calls if present
            if let Some(tool_calls) = c.message.tool_calls {
                for tc in tool_calls {
                    let input: serde_json::Value =
                        serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                    blocks.push(ContentBlock::ToolUse {
                        id: tc.id,
                        name: tc.function.name,
                        input,
                        cache_control: None,
                    });
                }
            }

            let stop = Some(c.finish_reason.as_deref()
                .map(crate::common::map_stop_reason)
                .unwrap_or(StopReason::EndTurn));

            (blocks, stop)
        } else {
            (vec![], Some(StopReason::EndTurn))
        };

        let usage = resp.usage.unwrap_or(OpenAiUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
        });

        CompletionResponse {
            id: resp.id,
            response_type: "message".to_string(),
            role: Role::Assistant,
            content,
            model: resp.model,
            stop_reason,
            usage: Usage {
                input_tokens: usage.prompt_tokens,
                output_tokens: usage.completion_tokens,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            },
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiErrorResponse {
    error: OpenAiError,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiError {
    message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// SSE Streaming
// ─────────────────────────────────────────────────────────────────────────────

fn parse_openai_sse_stream(
    byte_stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
) -> ResponseStream {
    Box::pin(futures::stream::unfold(
        OpenAiSseState {
            byte_stream: Box::pin(byte_stream),
            buffer: String::new(),
            done: false,
            message_id: None,
            model: None,
            started: false,
        },
        |mut state| async move {
            if state.done {
                return None;
            }

            loop {
                // Process lines in buffer
                while let Some(line_end) = state.buffer.find('\n') {
                    let line = state.buffer[..line_end].trim().to_string();
                    state.buffer = state.buffer[line_end + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            state.done = true;
                            return Some((Ok(StreamEvent::MessageStop), state));
                        }

                        if let Ok(chunk) = serde_json::from_str::<OpenAiStreamChunk>(data) {
                            // Emit MessageStart on first chunk
                            if !state.started {
                                state.started = true;
                                state.message_id = Some(chunk.id.clone());
                                state.model = Some(chunk.model.clone());
                                return Some((
                                    Ok(StreamEvent::MessageStart {
                                        id: chunk.id,
                                        model: chunk.model,
                                    }),
                                    state,
                                ));
                            }

                            // Process choices
                            if let Some(choice) = chunk.choices.into_iter().next() {
                                if let Some(delta) = choice.delta {
                                    // Text content
                                    if let Some(content) = delta.content
                                        && !content.is_empty()
                                    {
                                        return Some((
                                            Ok(StreamEvent::ContentBlockDelta {
                                                index: 0,
                                                delta: ContentDelta::TextDelta(content),
                                            }),
                                            state,
                                        ));
                                    }

                                    // Tool calls (streamed as partial JSON)
                                    if let Some(tool_calls) = delta.tool_calls {
                                        for tc in tool_calls {
                                            if let Some(func) = tc.function
                                                && let Some(args) = func.arguments
                                            {
                                                return Some((
                                                    Ok(StreamEvent::ContentBlockDelta {
                                                        index: tc.index.unwrap_or(0),
                                                        delta: ContentDelta::InputJsonDelta(args),
                                                    }),
                                                    state,
                                                ));
                                            }
                                        }
                                    }
                                }

                                // Check for finish
                                if let Some(reason) = choice.finish_reason {
                                    let stop_reason = crate::common::map_stop_reason(reason.as_str());
                                    return Some((
                                        Ok(StreamEvent::MessageDelta {
                                            stop_reason,
                                            usage: Usage::new(0, 0),
                                        }),
                                        state,
                                    ));
                                }
                            }
                        }
                    }
                }

                // Need more data
                match state.byte_stream.next().await {
                    Some(Ok(bytes)) => {
                        let text = String::from_utf8_lossy(&bytes);
                        state.buffer.push_str(&text);
                    }
                    Some(Err(e)) => {
                        state.done = true;
                        return Some((Err(LlmError::Network(e.to_string())), state));
                    }
                    None => {
                        // Stream exhausted - state is dropped
                        return None;
                    }
                }
            }
        },
    ))
}

struct OpenAiSseState {
    byte_stream: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
    buffer: String,
    done: bool,
    message_id: Option<String>,
    model: Option<String>,
    started: bool,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiStreamChunk {
    id: String,
    model: String,
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiStreamChoice {
    delta: Option<OpenAiStreamDelta>,
    finish_reason: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiStreamDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiStreamToolCall>>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiStreamToolCall {
    index: Option<usize>,
    function: Option<OpenAiStreamFunction>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenAiStreamFunction {
    arguments: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;

    #[test]
    fn test_openai_config() {
        let config = OpenAiConfig::openai("test-key");
        assert_eq!(config.api_key.resolve(), Some("test-key".to_string()));
        assert_eq!(config.base_url, DEFAULT_OPENAI_BASE);
        assert_eq!(config.name, "openai");
    }

    #[test]
    fn test_groq_config() {
        let config = OpenAiConfig::groq("test-key");
        assert_eq!(config.api_key.resolve(), Some("test-key".to_string()));
        assert!(config.base_url.contains("groq.com"));
        assert_eq!(config.name, "groq");
        assert!(config.model.is_some());
    }

    #[test]
    fn test_ollama_config() {
        let config = OpenAiConfig::ollama();
        assert!(config.api_key.resolve().is_none());
        assert!(config.base_url.contains("localhost"));
        assert_eq!(config.name, "ollama");
        assert_eq!(config.timeout, Duration::from_secs(600));
    }

    #[test]
    fn test_config_builder() {
        let config = OpenAiConfig::openai("key")
            .with_base_url("http://custom.api")
            .with_model("gpt-4")
            .with_name("custom")
            .with_timeout(Duration::from_secs(60));

        assert_eq!(config.base_url, "http://custom.api");
        assert_eq!(config.model, Some("gpt-4".to_string()));
        assert_eq!(config.name, "custom");
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_completions_url() {
        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();
        assert_eq!(
            backend.completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_groq_completions_url() {
        let config = OpenAiConfig::groq("key");
        let backend = OpenAiBackend::new(config).unwrap();
        assert_eq!(
            backend.completions_url(),
            "https://api.groq.com/openai/v1/chat/completions"
        );
    }

    #[test]
    fn test_ollama_completions_url() {
        let config = OpenAiConfig::ollama();
        let backend = OpenAiBackend::new(config).unwrap();
        assert_eq!(
            backend.completions_url(),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn test_backend_name() {
        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();
        assert_eq!(backend.name(), "openai");
    }

    #[test]
    fn test_supports_native_tools() {
        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();
        assert!(backend.supports_native_tools());
    }

    #[test]
    fn test_openai_response_conversion() {
        let openai_resp = OpenAiChatResponse {
            id: "chatcmpl-123".to_string(),
            choices: vec![OpenAiChoice {
                message: OpenAiResponseMessage {
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            model: "gpt-4".to_string(),
            usage: Some(OpenAiUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
            }),
        };

        let response: CompletionResponse = openai_resp.into();
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.text(), "Hello!");
        assert_eq!(response.stop_reason, Some(StopReason::EndTurn));
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 5);
    }

    #[test]
    fn test_openai_response_with_tool_calls() {
        let openai_resp = OpenAiChatResponse {
            id: "chatcmpl-456".to_string(),
            choices: vec![OpenAiChoice {
                message: OpenAiResponseMessage {
                    content: Some("Let me check.".to_string()),
                    tool_calls: Some(vec![OpenAiToolCall {
                        id: "call_123".to_string(),
                        call_type: "function".to_string(),
                        function: OpenAiFunctionCall {
                            name: "read_file".to_string(),
                            arguments: r#"{"path": "/foo.rs"}"#.to_string(),
                        },
                    }]),
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            model: "gpt-4".to_string(),
            usage: Some(OpenAiUsage {
                prompt_tokens: 50,
                completion_tokens: 30,
            }),
        };

        let response: CompletionResponse = openai_resp.into();
        assert!(response.has_tool_use());
        assert_eq!(response.stop_reason, Some(StopReason::ToolUse));

        let tool_uses = response.tool_uses();
        assert_eq!(tool_uses.len(), 1);
        assert_eq!(tool_uses[0].name, "read_file");
    }

    #[test]
    fn test_add_headers_static_key() {
        let config = OpenAiConfig::groq("gsk_test_key_123");
        let backend = OpenAiBackend::new(config).unwrap();
        let req = backend
            .add_headers(backend.client.get("https://example.com"))
            .build()
            .unwrap();
        let auth = req
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(auth, "Bearer gsk_test_key_123");
    }

    #[test]
    fn test_add_headers_dynamic_provider() {
        let mut config = OpenAiConfig::groq("placeholder");
        config.api_key = ApiKeyProvider::dynamic(|| Some("gsk_dynamic_key".to_string()));
        let backend = OpenAiBackend::new(config).unwrap();
        let req = backend
            .add_headers(backend.client.get("https://example.com"))
            .build()
            .unwrap();
        let auth = req
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(auth, "Bearer gsk_dynamic_key");
    }

    #[test]
    fn test_add_headers_no_key() {
        let config = OpenAiConfig::ollama();
        let backend = OpenAiBackend::new(config).unwrap();
        let req = backend
            .add_headers(backend.client.get("https://example.com"))
            .build()
            .unwrap();
        assert!(req.headers().get("authorization").is_none());
    }

    #[test]
    fn test_add_headers_preserves_special_chars() {
        let key = "gsk+test/key=with+special/chars==";
        let config = OpenAiConfig::groq(key);
        let backend = OpenAiBackend::new(config).unwrap();
        let req = backend
            .add_headers(backend.client.get("https://example.com"))
            .build()
            .unwrap();
        let auth = req
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(auth, format!("Bearer {}", key));
    }

    #[test]
    fn test_add_headers_real_groq_key_format() {
        // Exact format of a real 56-char Groq key
        let key = "gsk_test00000000000000000000000000000000000000000000fake";
        let mut config = OpenAiConfig::groq("placeholder");
        config.api_key = ApiKeyProvider::dynamic(move || Some(key.to_string()));
        let backend = OpenAiBackend::new(config).unwrap();
        let req = backend
            .add_headers(backend.client.get("https://example.com"))
            .build()
            .unwrap();
        let auth = req
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(auth, format!("Bearer {}", key));
        // Verify no truncation or padding
        let bearer_value = auth.strip_prefix("Bearer ").unwrap();
        assert_eq!(bearer_value.len(), 56);
        assert_eq!(bearer_value, key);
    }

    #[test]
    fn test_to_openai_request() {
        let config = OpenAiConfig::openai("key").with_model("gpt-4");
        let backend = OpenAiBackend::new(config).unwrap();

        let request = CompletionRequest::new("gpt-3.5-turbo", vec![Message::user("Hello")], 100);

        let openai_req = backend.to_openai_request(&request);
        // Should use config model, not request model
        assert_eq!(openai_req.model, "gpt-4");
        assert_eq!(openai_req.messages.len(), 1);
        assert_eq!(openai_req.messages[0].role, "user");
        assert_eq!(openai_req.max_tokens, Some(100));
    }

    #[test]
    fn test_to_openai_request_with_system() {
        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();

        let request = CompletionRequest::new("gpt-4", vec![Message::user("Hi")], 100)
            .with_system("Be helpful.");

        let openai_req = backend.to_openai_request(&request);
        assert_eq!(openai_req.messages.len(), 2);
        assert_eq!(openai_req.messages[0].role, "system");
        if let Some(OpenAiContent::Text(ref t)) = openai_req.messages[0].content {
            assert_eq!(t, "Be helpful.");
        } else {
            panic!("Expected text content for system message");
        }
    }

    #[test]
    fn test_to_openai_request_uses_request_model_when_no_config_model() {
        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();
        let request = CompletionRequest::new("gpt-3.5-turbo", vec![Message::user("Hi")], 50);
        let openai_req = backend.to_openai_request(&request);
        assert_eq!(openai_req.model, "gpt-3.5-turbo");
    }

    #[test]
    fn test_to_openai_request_with_tools() {
        use crate::types::ToolDefinition;

        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();

        let tools = vec![ToolDefinition {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}}),
        }];

        let request = CompletionRequest::new("gpt-4", vec![Message::user("Read foo.rs")], 100)
            .with_tools(tools);

        let openai_req = backend.to_openai_request(&request);
        assert!(openai_req.tools.is_some());
        let tools = openai_req.tools.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "read_file");
        assert_eq!(tools[0].tool_type, "function");
    }

    #[test]
    fn test_to_openai_request_with_tool_calls_and_results() {
        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();

        let messages = vec![
            Message::user("Read the file"),
            // Assistant message with tool call
            Message::assistant_blocks(vec![ContentBlock::ToolUse {
                id: "call_1".to_string(),
                name: "read_file".to_string(),
                input: serde_json::json!({"path": "/foo.rs"}),
                cache_control: None,
            }]),
            // User message with tool result
            Message {
                role: Role::User,
                content: crate::types::Content::Blocks(vec![ContentBlock::ToolResult {
                    tool_use_id: "call_1".to_string(),
                    content: Some(ToolResultContent::Text("file contents".to_string())),
                    is_error: false,
                    cache_control: None,
                }]),
            },
        ];

        let request = CompletionRequest::new("gpt-4", messages, 100);
        let openai_req = backend.to_openai_request(&request);

        // 3 messages: user, assistant w/ tool_calls, tool result
        assert_eq!(openai_req.messages.len(), 3);
        assert_eq!(openai_req.messages[0].role, "user");
        assert_eq!(openai_req.messages[1].role, "assistant");
        assert!(openai_req.messages[1].tool_calls.is_some());
        assert_eq!(openai_req.messages[2].role, "tool");
        assert_eq!(
            openai_req.messages[2].tool_call_id,
            Some("call_1".to_string())
        );
    }

    #[test]
    fn test_to_openai_request_with_tool_result_blocks() {
        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();

        // Tool result with Blocks variant
        let messages = vec![Message {
            role: Role::User,
            content: crate::types::Content::Blocks(vec![ContentBlock::ToolResult {
                tool_use_id: "call_2".to_string(),
                content: Some(ToolResultContent::Blocks(vec![
                    serde_json::json!({"type": "text", "text": "line 1"}),
                    serde_json::json!({"type": "text", "text": "line 2"}),
                ])),
                is_error: false,
                cache_control: None,
            }]),
        }];

        let request = CompletionRequest::new("gpt-4", messages, 100);
        let openai_req = backend.to_openai_request(&request);

        assert_eq!(openai_req.messages.len(), 1);
        assert_eq!(openai_req.messages[0].role, "tool");
        if let Some(OpenAiContent::Text(ref t)) = openai_req.messages[0].content {
            assert_eq!(t, "line 1\nline 2");
        }
    }

    #[test]
    fn test_to_openai_request_with_tool_result_none_content() {
        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();

        let messages = vec![Message {
            role: Role::User,
            content: crate::types::Content::Blocks(vec![ContentBlock::ToolResult {
                tool_use_id: "call_3".to_string(),
                content: None,
                is_error: false,
                cache_control: None,
            }]),
        }];

        let request = CompletionRequest::new("gpt-4", messages, 100);
        let openai_req = backend.to_openai_request(&request);
        assert_eq!(openai_req.messages[0].role, "tool");
        if let Some(OpenAiContent::Text(ref t)) = openai_req.messages[0].content {
            assert!(t.is_empty());
        }
    }

    #[test]
    fn test_to_openai_request_with_stop_sequences() {
        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();

        let mut request = CompletionRequest::new("gpt-4", vec![Message::user("Hi")], 100);
        request.stop_sequences = vec!["STOP".to_string(), "END".to_string()];

        let openai_req = backend.to_openai_request(&request);
        assert_eq!(
            openai_req.stop,
            Some(vec!["STOP".to_string(), "END".to_string()])
        );
    }

    #[test]
    fn test_to_openai_request_with_temperature() {
        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();

        let request =
            CompletionRequest::new("gpt-4", vec![Message::user("Hi")], 100).with_temperature(0.7);

        let openai_req = backend.to_openai_request(&request);
        assert_eq!(openai_req.temperature, Some(0.7));
    }

    #[test]
    fn test_openai_response_no_choices() {
        let resp = OpenAiChatResponse {
            id: "id".to_string(),
            choices: vec![],
            model: "gpt-4".to_string(),
            usage: None,
        };
        let response: CompletionResponse = resp.into();
        assert!(response.content.is_empty());
        assert_eq!(response.stop_reason, Some(StopReason::EndTurn));
        assert_eq!(response.usage.input_tokens, 0);
        assert_eq!(response.usage.output_tokens, 0);
    }

    #[test]
    fn test_openai_response_length_finish_reason() {
        let resp = OpenAiChatResponse {
            id: "id".to_string(),
            choices: vec![OpenAiChoice {
                message: OpenAiResponseMessage {
                    content: Some("truncated".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some("length".to_string()),
            }],
            model: "gpt-4".to_string(),
            usage: Some(OpenAiUsage {
                prompt_tokens: 100,
                completion_tokens: 4096,
            }),
        };
        let response: CompletionResponse = resp.into();
        assert_eq!(response.stop_reason, Some(StopReason::MaxTokens));
    }

    #[test]
    fn test_openai_response_empty_text_omitted() {
        let resp = OpenAiChatResponse {
            id: "id".to_string(),
            choices: vec![OpenAiChoice {
                message: OpenAiResponseMessage {
                    content: Some("".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            model: "gpt-4".to_string(),
            usage: Some(OpenAiUsage {
                prompt_tokens: 1,
                completion_tokens: 0,
            }),
        };
        let response: CompletionResponse = resp.into();
        // Empty text should not be added as a content block
        assert!(response.content.is_empty());
    }

    #[test]
    fn test_openai_response_unknown_finish_reason() {
        let resp = OpenAiChatResponse {
            id: "id".to_string(),
            choices: vec![OpenAiChoice {
                message: OpenAiResponseMessage {
                    content: Some("text".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some("content_filter".to_string()),
            }],
            model: "gpt-4".to_string(),
            usage: None,
        };
        let response: CompletionResponse = resp.into();
        assert_eq!(response.stop_reason, Some(StopReason::EndTurn));
    }

    #[test]
    fn test_config_with_max_retries() {
        let config = OpenAiConfig::openai("key").with_max_retries(5);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_config_with_retry_backoff() {
        let config = OpenAiConfig::openai("key").with_retry_backoff(Duration::from_secs(2));
        assert_eq!(config.retry_backoff, Duration::from_secs(2));
    }

    #[tokio::test]
    async fn test_parse_openai_sse_stream_text() {
        use bytes::Bytes;

        let chunks = vec![
            Ok(Bytes::from(
                "data: {\"id\":\"chatcmpl-1\",\"model\":\"gpt-4\",\"choices\":[{\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n",
            )),
            Ok(Bytes::from(
                "data: {\"id\":\"chatcmpl-1\",\"model\":\"gpt-4\",\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n",
            )),
            Ok(Bytes::from(
                "data: {\"id\":\"chatcmpl-1\",\"model\":\"gpt-4\",\"choices\":[{\"delta\":{\"content\":\" world\"},\"finish_reason\":null}]}\n\n",
            )),
            Ok(Bytes::from(
                "data: {\"id\":\"chatcmpl-1\",\"model\":\"gpt-4\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            )),
            Ok(Bytes::from("data: [DONE]\n\n")),
        ];

        let stream = futures::stream::iter(chunks);
        let mut sse_stream = parse_openai_sse_stream(stream);

        // First event: MessageStart
        let event = sse_stream.next().await.unwrap().unwrap();
        match event {
            StreamEvent::MessageStart { id, model } => {
                assert_eq!(id, "chatcmpl-1");
                assert_eq!(model, "gpt-4");
            }
            other => panic!("Expected MessageStart, got {:?}", other),
        }

        // Second: text delta "Hello"
        let event = sse_stream.next().await.unwrap().unwrap();
        match event {
            StreamEvent::ContentBlockDelta {
                index: 0,
                delta: ContentDelta::TextDelta(text),
            } => assert_eq!(text, "Hello"),
            other => panic!("Expected TextDelta 'Hello', got {:?}", other),
        }

        // Third: text delta " world"
        let event = sse_stream.next().await.unwrap().unwrap();
        match event {
            StreamEvent::ContentBlockDelta {
                index: 0,
                delta: ContentDelta::TextDelta(text),
            } => assert_eq!(text, " world"),
            other => panic!("Expected TextDelta ' world', got {:?}", other),
        }

        // Fourth: MessageDelta with stop reason
        let event = sse_stream.next().await.unwrap().unwrap();
        match event {
            StreamEvent::MessageDelta { stop_reason, .. } => {
                assert_eq!(stop_reason, StopReason::EndTurn);
            }
            other => panic!("Expected MessageDelta, got {:?}", other),
        }

        // Fifth: MessageStop from [DONE]
        let event = sse_stream.next().await.unwrap().unwrap();
        assert!(matches!(event, StreamEvent::MessageStop));

        // Stream should end
        assert!(sse_stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parse_openai_sse_stream_tool_calls() {
        use bytes::Bytes;

        let chunks = vec![
            Ok(Bytes::from(
                "data: {\"id\":\"chatcmpl-2\",\"model\":\"gpt-4\",\"choices\":[{\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n",
            )),
            Ok(Bytes::from(
                "data: {\"id\":\"chatcmpl-2\",\"model\":\"gpt-4\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"path\\\"\"}}]},\"finish_reason\":null}]}\n\n",
            )),
            Ok(Bytes::from(
                "data: {\"id\":\"chatcmpl-2\",\"model\":\"gpt-4\",\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
            )),
            Ok(Bytes::from("data: [DONE]\n\n")),
        ];

        let stream = futures::stream::iter(chunks);
        let mut sse_stream = parse_openai_sse_stream(stream);

        // MessageStart
        let event = sse_stream.next().await.unwrap().unwrap();
        assert!(matches!(event, StreamEvent::MessageStart { .. }));

        // Tool call delta
        let event = sse_stream.next().await.unwrap().unwrap();
        match event {
            StreamEvent::ContentBlockDelta {
                index: 0,
                delta: ContentDelta::InputJsonDelta(json),
            } => assert_eq!(json, "{\"path\""),
            other => panic!("Expected InputJsonDelta, got {:?}", other),
        }

        // Finish: tool_calls
        let event = sse_stream.next().await.unwrap().unwrap();
        match event {
            StreamEvent::MessageDelta { stop_reason, .. } => {
                assert_eq!(stop_reason, StopReason::ToolUse);
            }
            other => panic!("Expected MessageDelta with ToolUse, got {:?}", other),
        }

        // [DONE]
        let event = sse_stream.next().await.unwrap().unwrap();
        assert!(matches!(event, StreamEvent::MessageStop));
    }

    #[tokio::test]
    async fn test_parse_openai_sse_stream_network_error() {
        use bytes::Bytes;

        let chunks: Vec<reqwest::Result<Bytes>> = vec![Err(reqwest::Client::new()
            .get("http://0.0.0.0:1")
            .send()
            .await
            .unwrap_err())];

        let stream = futures::stream::iter(chunks);
        let mut sse_stream = parse_openai_sse_stream(stream);

        let event = sse_stream.next().await.unwrap();
        assert!(event.is_err());
    }

    #[tokio::test]
    async fn test_parse_openai_sse_stream_length_finish_reason() {
        use bytes::Bytes;

        let chunks = vec![
            Ok(Bytes::from(
                "data: {\"id\":\"c\",\"model\":\"m\",\"choices\":[{\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n",
            )),
            Ok(Bytes::from(
                "data: {\"id\":\"c\",\"model\":\"m\",\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n\n",
            )),
            Ok(Bytes::from("data: [DONE]\n\n")),
        ];

        let stream = futures::stream::iter(chunks);
        let mut sse_stream = parse_openai_sse_stream(stream);

        // MessageStart
        sse_stream.next().await;
        // MessageDelta with MaxTokens
        let event = sse_stream.next().await.unwrap().unwrap();
        match event {
            StreamEvent::MessageDelta { stop_reason, .. } => {
                assert_eq!(stop_reason, StopReason::MaxTokens);
            }
            other => panic!("Expected MessageDelta MaxTokens, got {:?}", other),
        }
    }

    #[test]
    fn test_openai_response_tool_call_with_invalid_json_args() {
        let resp = OpenAiChatResponse {
            id: "id".to_string(),
            choices: vec![OpenAiChoice {
                message: OpenAiResponseMessage {
                    content: None,
                    tool_calls: Some(vec![OpenAiToolCall {
                        id: "call_1".to_string(),
                        call_type: "function".to_string(),
                        function: OpenAiFunctionCall {
                            name: "test".to_string(),
                            arguments: "not valid json".to_string(),
                        },
                    }]),
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            model: "gpt-4".to_string(),
            usage: None,
        };
        let response: CompletionResponse = resp.into();
        // Should still produce a tool use block with default value
        assert!(response.has_tool_use());
    }

    #[test]
    fn test_openai_response_none_content() {
        let resp = OpenAiChatResponse {
            id: "id".to_string(),
            choices: vec![OpenAiChoice {
                message: OpenAiResponseMessage {
                    content: None,
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            model: "gpt-4".to_string(),
            usage: Some(OpenAiUsage {
                prompt_tokens: 5,
                completion_tokens: 0,
            }),
        };
        let response: CompletionResponse = resp.into();
        assert!(response.content.is_empty());
    }

    #[test]
    fn test_to_openai_request_assistant_with_text_and_tool_call() {
        let config = OpenAiConfig::openai("key");
        let backend = OpenAiBackend::new(config).unwrap();

        let messages = vec![Message::assistant_blocks(vec![
            ContentBlock::Text {
                text: "I'll read that.".to_string(),
                cache_control: None,
            },
            ContentBlock::ToolUse {
                id: "call_1".to_string(),
                name: "read_file".to_string(),
                input: serde_json::json!({"path": "a.rs"}),
                cache_control: None,
            },
        ])];

        let request = CompletionRequest::new("gpt-4", messages, 100);
        let openai_req = backend.to_openai_request(&request);

        assert_eq!(openai_req.messages.len(), 1);
        let msg = &openai_req.messages[0];
        assert_eq!(msg.role, "assistant");
        // Should have text content
        assert!(msg.content.is_some());
        if let Some(OpenAiContent::Text(ref t)) = msg.content {
            assert_eq!(t, "I'll read that.");
        }
        // Should have tool calls
        assert!(msg.tool_calls.is_some());
        assert_eq!(msg.tool_calls.as_ref().unwrap().len(), 1);
    }
}

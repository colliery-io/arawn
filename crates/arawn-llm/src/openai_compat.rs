use std::pin::Pin;

use async_trait::async_trait;
use futures::stream::Stream;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::debug;

use crate::client::LlmClient;
use crate::error::LlmError;
use crate::types::{ChatChunk, ChatContent, ChatMessage, ChatRequest, ToolDefinition, Usage};

/// Generic client for any OpenAI-compatible API (Groq, Ollama, OpenAI, vLLM,
/// LM Studio, Together, Fireworks, etc.)
///
/// The only differences between providers are `base_url` and `api_key`.
pub struct OpenAICompatibleClient {
    http: Client,
    base_url: String,
    api_key: Option<String>,
    provider_name: String,
}

impl OpenAICompatibleClient {
    pub fn new(
        base_url: impl Into<String>,
        api_key: Option<String>,
        provider_name: impl Into<String>,
    ) -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: base_url.into(),
            api_key,
            provider_name: provider_name.into(),
        }
    }

    /// Create a client for Groq.
    pub fn groq(api_key: impl Into<String>) -> Self {
        Self::new(
            "https://api.groq.com/openai/v1",
            Some(api_key.into()),
            "groq",
        )
    }

    /// Create a client for Groq from the GROQ_API_KEY env var.
    pub fn groq_from_env() -> Result<Self, LlmError> {
        let api_key = std::env::var("GROQ_API_KEY")
            .map_err(|_| LlmError::Config("GROQ_API_KEY environment variable not set".into()))?;
        Ok(Self::groq(api_key))
    }

    /// Create a client for Ollama (local, no API key needed).
    pub fn ollama() -> Self {
        Self::new("http://localhost:11434/v1", None, "ollama")
    }

    /// Create a client for Ollama with a custom host/port.
    pub fn ollama_at(base_url: impl Into<String>) -> Self {
        Self::new(base_url, None, "ollama")
    }

    /// Create a client for OpenAI.
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self::new(
            "https://api.openai.com/v1",
            Some(api_key.into()),
            "openai",
        )
    }

    /// Create a client for OpenAI from the OPENAI_API_KEY env var.
    pub fn openai_from_env() -> Result<Self, LlmError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| LlmError::Config("OPENAI_API_KEY environment variable not set".into()))?;
        Ok(Self::openai(api_key))
    }

    /// Create from explicit config values.
    pub fn from_config(
        provider: &str,
        base_url: Option<&str>,
        api_key_env: &str,
    ) -> Result<Self, LlmError> {
        let (default_url, name) = match provider {
            "groq" => ("https://api.groq.com/openai/v1", "groq"),
            "ollama" => ("http://localhost:11434/v1", "ollama"),
            "openai" => ("https://api.openai.com/v1", "openai"),
            "lmstudio" => ("http://localhost:1234/v1", "lmstudio"),
            "mistral" => ("https://api.mistral.ai/v1", "mistral"),
            "together" => ("https://api.together.xyz/v1", "together"),
            "fireworks" => ("https://api.fireworks.ai/inference/v1", "fireworks"),
            other => (other, other), // Treat unknown provider as a direct URL
        };

        let url = base_url.unwrap_or(default_url);

        let api_key = if api_key_env.is_empty() {
            None
        } else {
            match std::env::var(api_key_env) {
                Ok(key) if !key.is_empty() => Some(key),
                _ => None,
            }
        };

        Ok(Self::new(url, api_key, name))
    }

    fn build_request_body(&self, request: &ChatRequest) -> Value {
        let messages = build_messages(&request.system_prompt, &request.messages);
        let tools = build_tools(&request.tools);

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }

        if !tools.is_empty() {
            body["tools"] = json!(tools);
        }

        body
    }

    fn completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl LlmClient for OpenAICompatibleClient {
    async fn stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError> {
        let body = self.build_request_body(&request);
        debug!(
            provider = %self.provider_name,
            url = %self.completions_url(),
            "LLM request"
        );

        let mut req = self
            .http
            .post(&self.completions_url())
            .header("Content-Type", "application/json")
            .json(&body);

        if let Some(ref api_key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {api_key}"));
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(LlmError::from_status(status, text));
        }

        let byte_stream = response.bytes_stream();
        let stream = SseParser::new(byte_stream, self.provider_name.clone());

        Ok(Box::pin(stream))
    }
}

// --- SSE stream parser (same as groq.rs, now provider-aware for error messages) ---

struct SseParser<S> {
    inner: S,
    buffer: String,
    pending_chunks: Vec<ChatChunk>,
    provider: String,
}

impl<S> SseParser<S> {
    fn new(inner: S, provider: String) -> Self {
        Self {
            inner,
            buffer: String::new(),
            pending_chunks: Vec::new(),
            provider,
        }
    }
}

impl<S> Stream for SseParser<S>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin + Send,
{
    type Item = Result<ChatChunk, LlmError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if let Some(chunk) = self.pending_chunks.pop() {
            return std::task::Poll::Ready(Some(Ok(chunk)));
        }

        loop {
            if let Some(chunk) = self.try_parse_buffer() {
                return std::task::Poll::Ready(Some(chunk));
            }

            match Pin::new(&mut self.inner).poll_next(cx) {
                std::task::Poll::Ready(Some(Ok(bytes))) => {
                    let text = String::from_utf8_lossy(&bytes);
                    self.buffer.push_str(&text);
                }
                std::task::Poll::Ready(Some(Err(e))) => {
                    return std::task::Poll::Ready(Some(Err(LlmError::Request(e))));
                }
                std::task::Poll::Ready(None) => {
                    if !self.buffer.is_empty() {
                        self.buffer.push('\n');
                        if let Some(chunk) = self.try_parse_buffer() {
                            return std::task::Poll::Ready(Some(chunk));
                        }
                        self.buffer.clear();
                    }
                    return std::task::Poll::Ready(None);
                }
                std::task::Poll::Pending => {
                    return std::task::Poll::Pending;
                }
            }
        }
    }
}

impl<S> SseParser<S> {
    fn try_parse_buffer(&mut self) -> Option<Result<ChatChunk, LlmError>> {
        loop {
            let line_end = self.buffer.find('\n')?;
            let line = self.buffer[..line_end].trim_end_matches('\r').to_string();
            self.buffer = self.buffer[line_end + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return Some(Ok(ChatChunk::Done { usage: None }));
                }

                // Check for inline error responses
                if let Ok(err_resp) = serde_json::from_str::<ApiErrorResponse>(data)
                    && let Some(err) = err_resp.error
                {
                    return Some(Err(LlmError::Api(format!(
                        "{} error ({}): {}",
                        self.provider,
                        err.code.unwrap_or_default(),
                        err.message
                    ))));
                }

                match serde_json::from_str::<StreamChunk>(data) {
                    Ok(chunk) => {
                        let mut chunks = parse_stream_chunk(&chunk);
                        if !chunks.is_empty() {
                            let first = chunks.remove(0);
                            for remaining in chunks.into_iter().rev() {
                                self.pending_chunks.push(remaining);
                            }
                            return Some(Ok(first));
                        }
                    }
                    Err(e) => {
                        return Some(Err(LlmError::Stream(format!(
                            "Failed to parse SSE data: {e}\nRaw: {data}"
                        ))));
                    }
                }
            }
        }
    }
}

fn parse_stream_chunk(chunk: &StreamChunk) -> Vec<ChatChunk> {
    let mut results = Vec::new();

    if let Some(ref usage) = chunk.usage {
        results.push(ChatChunk::Done {
            usage: Some(Usage {
                input_tokens: usage.prompt_tokens,
                output_tokens: usage.completion_tokens,
            }),
        });
        return results;
    }

    let Some(choice) = chunk.choices.first() else {
        return results;
    };
    let delta = &choice.delta;

    if let Some(tool_calls) = &delta.tool_calls
        && let Some(tc) = tool_calls.first()
        && let Some(ref func) = tc.function
    {
        if let Some(ref name) = func.name {
            results.push(ChatChunk::ToolUseStart {
                id: tc.id.clone().unwrap_or_default(),
                name: name.clone(),
            });
        }
        if let Some(ref args) = func.arguments
            && !args.is_empty()
        {
            results.push(ChatChunk::ToolUseInputDelta { json: args.clone() });
        }
        return results;
    }

    if let Some(ref content) = delta.content
        && !content.is_empty()
    {
        results.push(ChatChunk::TextDelta {
            text: content.clone(),
        });
    }

    results
}

// --- OpenAI-compatible request/response building ---

fn build_messages(system_prompt: &Option<String>, messages: &[ChatMessage]) -> Vec<Value> {
    let mut result = Vec::new();

    if let Some(system) = system_prompt {
        result.push(json!({
            "role": "system",
            "content": system,
        }));
    }

    for msg in messages {
        match msg.role.as_str() {
            "user" => {
                let ChatContent::Text(ref text) = msg.content;
                result.push(json!({
                    "role": "user",
                    "content": text,
                }));
            }
            "assistant" => {
                let ChatContent::Text(ref text) = msg.content;
                let mut m = json!({ "role": "assistant" });
                if !text.is_empty() {
                    m["content"] = json!(text);
                }
                if !msg.tool_calls.is_empty() {
                    let tool_calls: Vec<Value> = msg
                        .tool_calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments.to_string(),
                                }
                            })
                        })
                        .collect();
                    m["tool_calls"] = json!(tool_calls);
                }
                result.push(m);
            }
            "tool" => {
                let ChatContent::Text(ref text) = msg.content;
                result.push(json!({
                    "role": "tool",
                    "tool_call_id": msg.tool_call_id,
                    "content": text,
                }));
            }
            other => {
                let ChatContent::Text(ref text) = msg.content;
                result.push(json!({
                    "role": other,
                    "content": text,
                }));
            }
        }
    }

    result
}

fn build_tools(tools: &[ToolDefinition]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                }
            })
        })
        .collect()
}

// --- Response types (OpenAI-compatible) ---

#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
    #[serde(default)]
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<StreamUsage>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
    tool_calls: Option<Vec<StreamToolCall>>,
}

#[derive(Debug, Deserialize)]
struct StreamToolCall {
    id: Option<String>,
    function: Option<StreamFunction>,
}

#[derive(Debug, Deserialize)]
struct StreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolCall;

    #[test]
    fn groq_convenience_constructor() {
        let client = OpenAICompatibleClient::groq("test-key");
        assert_eq!(client.completions_url(), "https://api.groq.com/openai/v1/chat/completions");
        assert_eq!(client.provider_name, "groq");
        assert_eq!(client.api_key, Some("test-key".into()));
    }

    #[test]
    fn ollama_convenience_constructor() {
        let client = OpenAICompatibleClient::ollama();
        assert_eq!(client.completions_url(), "http://localhost:11434/v1/chat/completions");
        assert_eq!(client.provider_name, "ollama");
        assert!(client.api_key.is_none());
    }

    #[test]
    fn openai_convenience_constructor() {
        let client = OpenAICompatibleClient::openai("sk-test");
        assert_eq!(client.completions_url(), "https://api.openai.com/v1/chat/completions");
        assert_eq!(client.provider_name, "openai");
    }

    #[test]
    fn custom_base_url() {
        let client = OpenAICompatibleClient::new(
            "http://my-vllm-server:8000/v1",
            Some("my-key".into()),
            "vllm",
        );
        assert_eq!(client.completions_url(), "http://my-vllm-server:8000/v1/chat/completions");
    }

    #[test]
    fn from_config_known_providers() {
        let client = OpenAICompatibleClient::from_config("ollama", None, "").unwrap();
        assert_eq!(client.completions_url(), "http://localhost:11434/v1/chat/completions");
        assert!(client.api_key.is_none());
    }

    #[test]
    fn from_config_custom_url_override() {
        let client = OpenAICompatibleClient::from_config(
            "groq",
            Some("https://custom-groq-proxy.example.com/v1"),
            "",
        ).unwrap();
        assert_eq!(client.completions_url(), "https://custom-groq-proxy.example.com/v1/chat/completions");
    }

    #[test]
    fn build_messages_with_system_prompt() {
        let msgs = build_messages(
            &Some("You are helpful.".into()),
            &[ChatMessage {
                role: "user".into(),
                content: ChatContent::Text("hello".into()),
                tool_calls: vec![],
                tool_call_id: None,
            }],
        );
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[1]["content"], "hello");
    }

    #[test]
    fn parse_text_delta() {
        let chunk = StreamChunk {
            choices: vec![StreamChoice {
                delta: StreamDelta {
                    content: Some("Hello".into()),
                    tool_calls: None,
                },
            }],
            usage: None,
        };
        let result = parse_stream_chunk(&chunk);
        assert_eq!(result.len(), 1);
        assert!(matches!(&result[0], ChatChunk::TextDelta { text } if text == "Hello"));
    }

    #[test]
    fn parse_tool_use_start() {
        let chunk = StreamChunk {
            choices: vec![StreamChoice {
                delta: StreamDelta {
                    content: None,
                    tool_calls: Some(vec![StreamToolCall {
                        id: Some("call_abc".into()),
                        function: Some(StreamFunction {
                            name: Some("file_read".into()),
                            arguments: Some("".into()),
                        }),
                    }]),
                },
            }],
            usage: None,
        };
        let result = parse_stream_chunk(&chunk);
        assert_eq!(result.len(), 1);
        assert!(matches!(&result[0], ChatChunk::ToolUseStart { name, .. } if name == "file_read"));
    }

    #[test]
    fn parse_usage() {
        let chunk = StreamChunk {
            choices: vec![],
            usage: Some(StreamUsage {
                prompt_tokens: 100,
                completion_tokens: 50,
            }),
        };
        let result = parse_stream_chunk(&chunk);
        assert_eq!(result.len(), 1);
        assert!(matches!(&result[0], ChatChunk::Done { usage: Some(u) } if u.input_tokens == 100));
    }

    #[test]
    fn no_auth_header_when_no_api_key() {
        let client = OpenAICompatibleClient::ollama();
        let request = ChatRequest {
            model: "llama3".into(),
            system_prompt: None,
            messages: vec![ChatMessage {
                role: "user".into(),
                content: ChatContent::Text("hi".into()),
                tool_calls: vec![],
                tool_call_id: None,
            }],
            tools: vec![],
            max_tokens: None,
        };
        let body = client.build_request_body(&request);
        assert_eq!(body["model"], "llama3");
        // api_key is None — no Authorization header will be sent
        assert!(client.api_key.is_none());
    }
}

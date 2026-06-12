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
        Self::new("https://api.openai.com/v1", Some(api_key.into()), "openai")
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
        api_key: Option<String>,
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
        let api_key = api_key.filter(|s| !s.is_empty());

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
            .post(self.completions_url())
            .header("Content-Type", "application/json")
            .json(&body);

        if let Some(ref api_key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {api_key}"));
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            // Capture Retry-After before consuming the body — the retry layer
            // honors it on a 429.
            let retry_after = parse_retry_after(response.headers());
            let text = response.text().await.unwrap_or_default();
            return Err(LlmError::from_status_with_retry_after(
                status,
                text,
                retry_after,
            ));
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
    /// The most recent `finish_reason` seen in a choice. OpenAI reports it in
    /// a choice *before* the `[DONE]` marker (and any usage-only chunk), so we
    /// stash it here and attach it to whichever `Done` chunk we emit.
    finish_reason: Option<crate::types::FinishReason>,
}

impl<S> SseParser<S> {
    fn new(inner: S, provider: String) -> Self {
        Self {
            inner,
            buffer: String::new(),
            pending_chunks: Vec::new(),
            provider,
            finish_reason: None,
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
                    return Some(Ok(ChatChunk::Done {
                        usage: None,
                        finish_reason: self.finish_reason.take(),
                    }));
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
                        // Capture finish_reason as soon as a choice reports
                        // one — it arrives in a content/empty delta before the
                        // terminal usage chunk or `[DONE]`.
                        if let Some(fr) = chunk
                            .choices
                            .first()
                            .and_then(|c| c.finish_reason.as_deref())
                        {
                            self.finish_reason = Some(crate::types::FinishReason::from_openai(fr));
                        }
                        let mut chunks = parse_stream_chunk(&chunk, &mut self.finish_reason);
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

/// Translate one parsed SSE chunk into provider-neutral `ChatChunk`s.
///
/// `finish_reason` is the parser's running finish_reason, consumed (`take`n)
/// when this chunk is terminal so it rides along on the emitted `Done`.
fn parse_stream_chunk(
    chunk: &StreamChunk,
    finish_reason: &mut Option<crate::types::FinishReason>,
) -> Vec<ChatChunk> {
    let mut results = Vec::new();

    if let Some(ref usage) = chunk.usage {
        results.push(ChatChunk::Done {
            usage: Some(Usage {
                input_tokens: usage.prompt_tokens,
                output_tokens: usage.completion_tokens,
            }),
            finish_reason: finish_reason.take(),
        });
        return results;
    }

    let Some(choice) = chunk.choices.first() else {
        return results;
    };
    let delta = &choice.delta;

    if let Some(tool_calls) = &delta.tool_calls {
        // Iterate ALL tool calls in the delta — a model emitting parallel
        // calls can place several in one delta, and interleave argument
        // deltas for different `index`es across subsequent deltas. We key
        // every emitted chunk by index so the engine assembles them
        // independently rather than by arrival order.
        for (pos, tc) in tool_calls.iter().enumerate() {
            let Some(ref func) = tc.function else {
                continue;
            };
            // Fall back to array position when the provider omits `index`
            // (single-tool OpenAI-compatible servers sometimes do): a lone
            // call lands in slot 0 across deltas, matching the old behavior.
            let index = tc.index.unwrap_or(pos as u32);

            if let Some(ref name) = func.name {
                // Never key on an empty id — synthesize a deterministic,
                // non-empty one from the index so downstream tool-result
                // routing has a stable handle.
                let id = tc
                    .id
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("call_{index}"));
                results.push(ChatChunk::ToolUseStart {
                    index,
                    id,
                    name: name.clone(),
                });
            }
            if let Some(ref args) = func.arguments
                && !args.is_empty()
            {
                results.push(ChatChunk::ToolUseInputDelta {
                    index,
                    json: args.clone(),
                });
            }
        }
        if !results.is_empty() {
            return results;
        }
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
                // Ollama's `/v1/chat/completions` requires `content` to be
                // present as a string even when the assistant message is
                // tool-only (no narration). Omitting it produces an
                // `invalid message content type: <nil>` HTTP 400 from the
                // server. OpenAI's spec accepts `null`, `""`, or absent;
                // emit `""` here so both providers are happy. See the
                // tool-only roundtrip test below.
                let ChatContent::Text(ref text) = msg.content;
                let mut m = json!({
                    "role": "assistant",
                    "content": text,
                });
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

/// Parse a `Retry-After` header into a duration. Handles the common
/// delta-seconds form (`Retry-After: 30`); HTTP-date form is treated as
/// absent (callers fall back to exponential backoff).
fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<std::time::Duration> {
    let v = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    let secs: u64 = v.trim().parse().ok()?;
    Some(std::time::Duration::from_secs(secs))
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
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
    tool_calls: Option<Vec<StreamToolCall>>,
}

#[derive(Debug, Deserialize)]
struct StreamToolCall {
    /// Position of this tool call within the assistant turn. The OpenAI
    /// streaming spec sends one `index` per parallel tool call and keys
    /// interleaved argument deltas by it. Providers that only ever emit a
    /// single tool call may omit it; we fall back to the call's position in
    /// the delta array (see `parse_stream_chunk`).
    #[serde(default)]
    index: Option<u32>,
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

    #[test]
    fn groq_convenience_constructor() {
        let client = OpenAICompatibleClient::groq("test-key");
        assert_eq!(
            client.completions_url(),
            "https://api.groq.com/openai/v1/chat/completions"
        );
        assert_eq!(client.provider_name, "groq");
        assert_eq!(client.api_key, Some("test-key".into()));
    }

    #[test]
    fn ollama_convenience_constructor() {
        let client = OpenAICompatibleClient::ollama();
        assert_eq!(
            client.completions_url(),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(client.provider_name, "ollama");
        assert!(client.api_key.is_none());
    }

    #[test]
    fn openai_convenience_constructor() {
        let client = OpenAICompatibleClient::openai("sk-test");
        assert_eq!(
            client.completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(client.provider_name, "openai");
    }

    #[test]
    fn custom_base_url() {
        let client = OpenAICompatibleClient::new(
            "http://my-vllm-server:8000/v1",
            Some("my-key".into()),
            "vllm",
        );
        assert_eq!(
            client.completions_url(),
            "http://my-vllm-server:8000/v1/chat/completions"
        );
    }

    #[test]
    fn from_config_known_providers() {
        let client = OpenAICompatibleClient::from_config("ollama", None, None).unwrap();
        assert_eq!(
            client.completions_url(),
            "http://localhost:11434/v1/chat/completions"
        );
        assert!(client.api_key.is_none());
    }

    #[test]
    fn from_config_custom_url_override() {
        let client = OpenAICompatibleClient::from_config(
            "groq",
            Some("https://custom-groq-proxy.example.com/v1"),
            None,
        )
        .unwrap();
        assert_eq!(
            client.completions_url(),
            "https://custom-groq-proxy.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn assistant_tool_only_message_includes_empty_content() {
        // Regression for the ollama HTTP 400 "invalid message content type: <nil>"
        // failure: when the assistant performs a tool call with no narration,
        // the serialized message MUST still carry `content` (empty string is
        // fine). Omitting the field makes ollama reject the request.
        use crate::types::ToolCall;
        use serde_json::json;
        let msgs = build_messages(
            &None,
            &[ChatMessage {
                role: "assistant".into(),
                content: ChatContent::Text(String::new()),
                tool_calls: vec![ToolCall {
                    id: "call_1".into(),
                    name: "skill".into(),
                    arguments: json!({"skill": "workflows"}),
                }],
                tool_call_id: None,
            }],
        );
        assert_eq!(msgs.len(), 1);
        let m = &msgs[0];
        assert_eq!(m["role"], "assistant");
        assert!(
            m.get("content").is_some(),
            "tool-only assistant message dropped the `content` field — ollama HTTP 400s"
        );
        assert_eq!(m["content"], "");
        assert!(m["tool_calls"].is_array());
    }

    #[test]
    fn assistant_message_with_text_and_tool_calls_keeps_both() {
        use crate::types::ToolCall;
        use serde_json::json;
        let msgs = build_messages(
            &None,
            &[ChatMessage {
                role: "assistant".into(),
                content: ChatContent::Text("Let me look that up.".into()),
                tool_calls: vec![ToolCall {
                    id: "call_2".into(),
                    name: "feed_search".into(),
                    arguments: json!({"q": "x"}),
                }],
                tool_call_id: None,
            }],
        );
        let m = &msgs[0];
        assert_eq!(m["content"], "Let me look that up.");
        assert!(m["tool_calls"].is_array());
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
                finish_reason: None,
            }],
            usage: None,
        };
        let result = parse_stream_chunk(&chunk, &mut None);
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
                        index: Some(0),
                        id: Some("call_abc".into()),
                        function: Some(StreamFunction {
                            name: Some("file_read".into()),
                            arguments: Some("".into()),
                        }),
                    }]),
                },
                finish_reason: None,
            }],
            usage: None,
        };
        let result = parse_stream_chunk(&chunk, &mut None);
        assert_eq!(result.len(), 1);
        assert!(matches!(
            &result[0],
            ChatChunk::ToolUseStart { name, id, index } if name == "file_read" && id == "call_abc" && *index == 0
        ));
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
        let result = parse_stream_chunk(&chunk, &mut None);
        assert_eq!(result.len(), 1);
        assert!(
            matches!(&result[0], ChatChunk::Done { usage: Some(u), .. } if u.input_tokens == 100)
        );
    }

    /// Helper to build a single-choice chunk carrying tool-call deltas.
    fn tool_delta_chunk(tool_calls: Vec<StreamToolCall>) -> StreamChunk {
        StreamChunk {
            choices: vec![StreamChoice {
                delta: StreamDelta {
                    content: None,
                    tool_calls: Some(tool_calls),
                },
                finish_reason: None,
            }],
            usage: None,
        }
    }

    #[test]
    fn parse_two_parallel_tool_calls_in_one_delta() {
        // Regression for P1-1: the old parser did `tool_calls.first()` and
        // silently dropped every call after the first.
        let chunk = tool_delta_chunk(vec![
            StreamToolCall {
                index: Some(0),
                id: Some("call_a".into()),
                function: Some(StreamFunction {
                    name: Some("file_read".into()),
                    arguments: Some(r#"{"path":"a"}"#.into()),
                }),
            },
            StreamToolCall {
                index: Some(1),
                id: Some("call_b".into()),
                function: Some(StreamFunction {
                    name: Some("file_read".into()),
                    arguments: Some(r#"{"path":"b"}"#.into()),
                }),
            },
        ]);
        let result = parse_stream_chunk(&chunk, &mut None);
        // start+args for each of the two calls
        let starts: Vec<_> = result
            .iter()
            .filter_map(|c| match c {
                ChatChunk::ToolUseStart { index, id, name } => {
                    Some((*index, id.clone(), name.clone()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(starts.len(), 2, "both parallel tool calls must be emitted");
        assert_eq!(starts[0], (0, "call_a".into(), "file_read".into()));
        assert_eq!(starts[1], (1, "call_b".into(), "file_read".into()));
    }

    #[test]
    fn tool_call_index_keys_interleaved_arg_deltas() {
        // Two calls' argument deltas arrive in separate, interleaved chunks;
        // each must keep its own index so the engine can route them.
        let c1 = tool_delta_chunk(vec![StreamToolCall {
            index: Some(0),
            id: Some("call_a".into()),
            function: Some(StreamFunction {
                name: Some("a".into()),
                arguments: Some(r#"{"x":"#.into()),
            }),
        }]);
        let c2 = tool_delta_chunk(vec![StreamToolCall {
            index: Some(1),
            id: Some("call_b".into()),
            function: Some(StreamFunction {
                name: Some("b".into()),
                arguments: Some(r#"{"y":"#.into()),
            }),
        }]);
        let c3 = tool_delta_chunk(vec![StreamToolCall {
            index: Some(0),
            id: None,
            function: Some(StreamFunction {
                name: None,
                arguments: Some("1}".into()),
            }),
        }]);
        let mut all = Vec::new();
        for c in [c1, c2, c3] {
            all.extend(parse_stream_chunk(&c, &mut None));
        }
        // Deltas for index 0 should be "{"x":" then "1}"
        let idx0_json: String = all
            .iter()
            .filter_map(|c| match c {
                ChatChunk::ToolUseInputDelta { index: 0, json } => Some(json.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(idx0_json, r#"{"x":1}"#);
    }

    #[test]
    fn missing_tool_call_id_is_synthesized_not_empty() {
        // Regression for P1-1: empty id must never reach the engine as an
        // empty-string key.
        let chunk = tool_delta_chunk(vec![StreamToolCall {
            index: Some(2),
            id: None,
            function: Some(StreamFunction {
                name: Some("thing".into()),
                arguments: None,
            }),
        }]);
        let result = parse_stream_chunk(&chunk, &mut None);
        let id = result.iter().find_map(|c| match c {
            ChatChunk::ToolUseStart { id, .. } => Some(id.clone()),
            _ => None,
        });
        assert_eq!(id, Some("call_2".into()));
    }

    #[test]
    fn missing_index_falls_back_to_array_position() {
        // A single tool call with no index lands in slot 0 (single-tool path).
        let chunk = tool_delta_chunk(vec![StreamToolCall {
            index: None,
            id: Some("call_x".into()),
            function: Some(StreamFunction {
                name: Some("solo".into()),
                arguments: None,
            }),
        }]);
        let result = parse_stream_chunk(&chunk, &mut None);
        assert!(matches!(
            &result[0],
            ChatChunk::ToolUseStart { index: 0, .. }
        ));
    }

    #[test]
    fn finish_reason_rides_on_done() {
        // A length-truncated stream: finish_reason captured on a choice, then
        // attached to the Done emitted from the usage chunk.
        let mut fr = Some(crate::types::FinishReason::Length);
        let usage_chunk = StreamChunk {
            choices: vec![],
            usage: Some(StreamUsage {
                prompt_tokens: 1,
                completion_tokens: 2,
            }),
        };
        let result = parse_stream_chunk(&usage_chunk, &mut fr);
        assert!(matches!(
            &result[0],
            ChatChunk::Done {
                finish_reason: Some(crate::types::FinishReason::Length),
                ..
            }
        ));
        assert!(
            fr.is_none(),
            "finish_reason should be taken, not duplicated"
        );
    }

    #[test]
    fn finish_reason_wire_mapping() {
        use crate::types::FinishReason;
        assert_eq!(FinishReason::from_openai("stop"), FinishReason::Stop);
        assert_eq!(
            FinishReason::from_openai("tool_calls"),
            FinishReason::ToolCalls
        );
        assert_eq!(FinishReason::from_openai("length"), FinishReason::Length);
        assert_eq!(
            FinishReason::from_openai("content_filter"),
            FinishReason::ContentFilter
        );
        assert!(FinishReason::from_openai("length").is_truncated());
        assert!(!FinishReason::from_openai("stop").is_truncated());
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

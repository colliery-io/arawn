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

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";

/// Groq LLM client using the OpenAI-compatible API.
pub struct GroqClient {
    http: Client,
    api_key: String,
}

impl GroqClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            api_key: api_key.into(),
        }
    }

    pub fn from_env() -> Result<Self, LlmError> {
        let api_key = std::env::var("GROQ_API_KEY")
            .map_err(|_| LlmError::Config("GROQ_API_KEY environment variable not set".into()))?;
        Ok(Self::new(api_key))
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
}

#[async_trait]
impl LlmClient for GroqClient {
    async fn stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError> {
        let body = self.build_request_body(&request);
        debug!(
            "Groq request: {}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );

        let response = self
            .http
            .post(GROQ_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(LlmError::from_status(status, text));
        }

        let byte_stream = response.bytes_stream();
        let stream = SseParser::new(byte_stream);

        Ok(Box::pin(stream))
    }
}

// --- SSE stream parser ---

/// Parses Server-Sent Events from a byte stream into ChatChunks.
struct SseParser<S> {
    inner: S,
    buffer: String,
    pending_chunks: Vec<ChatChunk>,
}

impl<S> SseParser<S> {
    fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: String::new(),
            pending_chunks: Vec::new(),
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
        // Drain any pending chunks from multi-chunk SSE events first
        if let Some(chunk) = self.pending_chunks.pop() {
            return std::task::Poll::Ready(Some(Ok(chunk)));
        }

        loop {
            // Try to extract a complete SSE event from the buffer
            if let Some(chunk) = self.try_parse_buffer() {
                return std::task::Poll::Ready(Some(chunk));
            }

            // Read more data from the inner stream
            match Pin::new(&mut self.inner).poll_next(cx) {
                std::task::Poll::Ready(Some(Ok(bytes))) => {
                    let text = String::from_utf8_lossy(&bytes);
                    self.buffer.push_str(&text);
                }
                std::task::Poll::Ready(Some(Err(e))) => {
                    return std::task::Poll::Ready(Some(Err(LlmError::Request(e))));
                }
                std::task::Poll::Ready(None) => {
                    // Stream ended — try to parse any remaining data in buffer
                    if !self.buffer.is_empty() {
                        // Add a newline so try_parse_buffer can find it
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
                debug!("SSE data: {data}");

                if data == "[DONE]" {
                    return Some(Ok(ChatChunk::Done { usage: None }));
                }

                // Check for inline error responses first
                if let Ok(err_resp) = serde_json::from_str::<GroqErrorResponse>(data)
                    && let Some(err) = err_resp.error
                {
                    return Some(Err(LlmError::Api(format!(
                        "Groq error ({}): {}",
                        err.code.unwrap_or_default(),
                        err.message
                    ))));
                }

                match serde_json::from_str::<GroqStreamChunk>(data) {
                    Ok(chunk) => {
                        let mut chunks = parse_groq_chunk(&chunk);
                        if !chunks.is_empty() {
                            // Return first chunk now, push rest back for next poll
                            let first = chunks.remove(0);
                            debug!("parsed chunk: {first:?}");
                            // Prepend remaining chunks as synthetic SSE lines
                            for remaining in chunks.into_iter().rev() {
                                self.pending_chunks.push(remaining);
                            }
                            return Some(Ok(first));
                        }
                        // Chunk had no actionable content, continue parsing
                    }
                    Err(e) => {
                        return Some(Err(LlmError::Stream(format!(
                            "Failed to parse SSE data: {e}\nRaw: {data}"
                        ))));
                    }
                }
            }
            // Skip non-data lines (comments, event types, etc.)
        }
    }
}

fn parse_groq_chunk(chunk: &GroqStreamChunk) -> Vec<ChatChunk> {
    let mut results = Vec::new();

    // Check for usage first (may come with empty choices)
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

    // Check for tool calls — name and arguments may arrive in the same chunk
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

    // Check for text content
    if let Some(ref content) = delta.content
        && !content.is_empty()
    {
        results.push(ChatChunk::TextDelta {
            text: content.clone(),
        });
    }

    results
}

// --- OpenAI-compatible request/response types ---

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
                let mut m = json!({
                    "role": "assistant",
                });
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

// --- Groq error response ---

#[derive(Debug, Deserialize)]
struct GroqErrorResponse {
    error: Option<GroqError>,
}

#[derive(Debug, Deserialize)]
struct GroqError {
    message: String,
    #[serde(default)]
    code: Option<String>,
}

// --- Groq SSE response types ---

#[derive(Debug, Deserialize)]
struct GroqStreamChunk {
    #[serde(default)]
    choices: Vec<GroqChoice>,
    #[serde(default)]
    usage: Option<GroqUsage>,
}

#[derive(Debug, Deserialize)]
struct GroqChoice {
    delta: GroqDelta,
}

#[derive(Debug, Deserialize)]
struct GroqDelta {
    content: Option<String>,
    tool_calls: Option<Vec<GroqToolCall>>,
}

#[derive(Debug, Deserialize)]
struct GroqToolCall {
    id: Option<String>,
    function: Option<GroqFunction>,
}

#[derive(Debug, Deserialize)]
struct GroqFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GroqUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolCall;

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
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "hello");
    }

    #[test]
    fn build_messages_with_tool_calls() {
        let msgs = build_messages(
            &None,
            &[ChatMessage {
                role: "assistant".into(),
                content: ChatContent::Text("".into()),
                tool_calls: vec![ToolCall {
                    id: "call_1".into(),
                    name: "file_read".into(),
                    arguments: json!({"path": "test.txt"}),
                }],
                tool_call_id: None,
            }],
        );
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0]["tool_calls"].is_array());
        assert_eq!(msgs[0]["tool_calls"][0]["function"]["name"], "file_read");
    }

    #[test]
    fn build_tools_format() {
        let tools = build_tools(&[ToolDefinition {
            name: "shell".into(),
            description: "Run a shell command".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string"}
                },
                "required": ["command"]
            }),
        }]);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "shell");
    }

    #[test]
    fn parse_text_delta_chunk() {
        let chunk = GroqStreamChunk {
            choices: vec![GroqChoice {
                delta: GroqDelta {
                    content: Some("Hello".into()),
                    tool_calls: None,
                },
            }],
            usage: None,
        };
        let result = parse_groq_chunk(&chunk);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ChatChunk::TextDelta { text } => assert_eq!(text, "Hello"),
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }

    #[test]
    fn parse_tool_use_start_chunk() {
        let chunk = GroqStreamChunk {
            choices: vec![GroqChoice {
                delta: GroqDelta {
                    content: None,
                    tool_calls: Some(vec![GroqToolCall {
                        id: Some("call_abc".into()),
                        function: Some(GroqFunction {
                            name: Some("file_read".into()),
                            arguments: Some("".into()),
                        }),
                    }]),
                },
            }],
            usage: None,
        };
        let result = parse_groq_chunk(&chunk);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ChatChunk::ToolUseStart { id, name } => {
                assert_eq!(id, "call_abc");
                assert_eq!(name, "file_read");
            }
            other => panic!("expected ToolUseStart, got {other:?}"),
        }
    }

    #[test]
    fn parse_tool_call_with_name_and_args_in_same_chunk() {
        // This is how Groq actually sends tool calls — name + args together
        let chunk = GroqStreamChunk {
            choices: vec![GroqChoice {
                delta: GroqDelta {
                    content: None,
                    tool_calls: Some(vec![GroqToolCall {
                        id: Some("fc_123".into()),
                        function: Some(GroqFunction {
                            name: Some("shell".into()),
                            arguments: Some(r#"{"command":"ls -R"}"#.into()),
                        }),
                    }]),
                },
            }],
            usage: None,
        };
        let result = parse_groq_chunk(&chunk);
        assert_eq!(result.len(), 2, "expected ToolUseStart + ToolUseInputDelta");
        match &result[0] {
            ChatChunk::ToolUseStart { id, name } => {
                assert_eq!(id, "fc_123");
                assert_eq!(name, "shell");
            }
            other => panic!("expected ToolUseStart, got {other:?}"),
        }
        match &result[1] {
            ChatChunk::ToolUseInputDelta { json } => {
                assert!(json.contains("ls -R"));
            }
            other => panic!("expected ToolUseInputDelta, got {other:?}"),
        }
    }

    #[test]
    fn parse_tool_use_input_delta_chunk() {
        let chunk = GroqStreamChunk {
            choices: vec![GroqChoice {
                delta: GroqDelta {
                    content: None,
                    tool_calls: Some(vec![GroqToolCall {
                        id: None,
                        function: Some(GroqFunction {
                            name: None,
                            arguments: Some(r#"{"path":"test.txt"}"#.into()),
                        }),
                    }]),
                },
            }],
            usage: None,
        };
        let result = parse_groq_chunk(&chunk);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ChatChunk::ToolUseInputDelta { json } => {
                assert!(json.contains("test.txt"));
            }
            other => panic!("expected ToolUseInputDelta, got {other:?}"),
        }
    }

    #[test]
    fn parse_usage_chunk() {
        let chunk = GroqStreamChunk {
            choices: vec![],
            usage: Some(GroqUsage {
                prompt_tokens: 100,
                completion_tokens: 50,
            }),
        };
        let result = parse_groq_chunk(&chunk);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ChatChunk::Done { usage: Some(u) } => {
                assert_eq!(u.input_tokens, 100);
                assert_eq!(u.output_tokens, 50);
            }
            other => panic!("expected Done with usage, got {other:?}"),
        }
    }

    #[test]
    fn build_request_body_includes_tools() {
        let client = GroqClient::new("test-key");
        let request = ChatRequest {
            model: "llama-3.3-70b-versatile".into(),
            system_prompt: Some("Be helpful".into()),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: ChatContent::Text("hi".into()),
                tool_calls: vec![],
                tool_call_id: None,
            }],
            tools: vec![ToolDefinition {
                name: "think".into(),
                description: "Think step by step".into(),
                parameters: json!({"type": "object", "properties": {}}),
            }],
            max_tokens: Some(1024),
        };
        let body = client.build_request_body(&request);
        assert_eq!(body["model"], "llama-3.3-70b-versatile");
        assert!(body["tools"].is_array());
        assert_eq!(body["max_tokens"], 1024);
        assert!(body["stream"].as_bool().unwrap());
    }
}

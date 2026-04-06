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

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

/// Client for Anthropic's Claude API (Messages API).
pub struct AnthropicClient {
    http: Client,
    api_key: String,
}

impl AnthropicClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            api_key: api_key.into(),
        }
    }

    pub fn from_env() -> Result<Self, LlmError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| LlmError::Config("ANTHROPIC_API_KEY environment variable not set".into()))?;
        Ok(Self::new(api_key))
    }

    fn build_request_body(&self, request: &ChatRequest) -> Value {
        let messages = build_messages(&request.messages);
        let tools = build_tools(&request.tools);

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if let Some(ref system) = request.system_prompt {
            body["system"] = json!(system);
        }

        if !tools.is_empty() {
            body["tools"] = json!(tools);
        }

        body
    }
}

#[async_trait]
impl LlmClient for AnthropicClient {
    async fn stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError> {
        let body = self.build_request_body(&request);
        debug!(url = API_URL, "Anthropic request");

        let response = self
            .http
            .post(API_URL)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body)
            .send()
            .await?;

        let status = response.status().as_u16();
        if status != 200 {
            let body_text = response.text().await.unwrap_or_default();
            return Err(LlmError::from_status(status, body_text));
        }

        let byte_stream = response.bytes_stream();

        let stream = async_stream::stream! {
            use futures::StreamExt;

            let mut buffer = String::new();
            let mut byte_stream = std::pin::pin!(byte_stream);

            while let Some(chunk) = byte_stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(LlmError::Stream(format!("HTTP stream error: {e}")));
                        return;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    let line = line.trim();
                    if line.is_empty() || line.starts_with(':') {
                        continue;
                    }

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            yield Ok(ChatChunk::Done { usage: None });
                            return;
                        }

                        match serde_json::from_str::<Value>(data) {
                            Ok(event) => {
                                let event_type = event["type"].as_str().unwrap_or("");

                                match event_type {
                                    "content_block_start" => {
                                        let block = &event["content_block"];
                                        if block["type"] == "tool_use" {
                                            let id = block["id"].as_str().unwrap_or("").to_string();
                                            let name = block["name"].as_str().unwrap_or("").to_string();
                                            yield Ok(ChatChunk::ToolUseStart { id, name });
                                        }
                                    }
                                    "content_block_delta" => {
                                        let delta = &event["delta"];
                                        match delta["type"].as_str().unwrap_or("") {
                                            "text_delta" => {
                                                let text = delta["text"].as_str().unwrap_or("").to_string();
                                                if !text.is_empty() {
                                                    yield Ok(ChatChunk::TextDelta { text });
                                                }
                                            }
                                            "input_json_delta" => {
                                                let json = delta["partial_json"].as_str().unwrap_or("").to_string();
                                                if !json.is_empty() {
                                                    yield Ok(ChatChunk::ToolUseInputDelta { json });
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    "message_delta" => {
                                        let usage = if let Some(u) = event.get("usage") {
                                            Some(Usage {
                                                input_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                                                output_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                                            })
                                        } else {
                                            None
                                        };
                                        // Don't emit Done here — wait for message_stop
                                        if let Some(u) = usage {
                                            // Store usage for message_stop
                                            yield Ok(ChatChunk::Done { usage: Some(u) });
                                        }
                                    }
                                    "message_stop" => {
                                        // Final event
                                        return;
                                    }
                                    "message_start" => {
                                        // Extract input token count from message_start
                                        if let Some(u) = event.get("message").and_then(|m| m.get("usage")) {
                                            let input = u["input_tokens"].as_u64().unwrap_or(0) as u32;
                                            // We'll capture this in message_delta's output_tokens
                                            debug!(input_tokens = input, "message_start usage");
                                        }
                                    }
                                    "error" => {
                                        let msg = event["error"]["message"].as_str().unwrap_or("unknown error");
                                        yield Err(LlmError::Api(format!("Anthropic error: {msg}")));
                                        return;
                                    }
                                    _ => {}
                                }
                            }
                            Err(e) => {
                                yield Err(LlmError::Stream(format!("Failed to parse SSE: {e}\nRaw: {data}")));
                            }
                        }
                    }

                    if let Some(event_data) = line.strip_prefix("event: ") {
                        // Event type lines — we handle type in the data payload
                        let _ = event_data;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

/// Convert arawn messages to Anthropic format.
/// Anthropic uses "user" and "assistant" roles only in the messages array.
/// System prompt is sent separately at the top level.
/// Tool results use role "user" with tool_result content blocks.
fn build_messages(messages: &[ChatMessage]) -> Vec<Value> {
    let mut result = Vec::new();

    for msg in messages {
        match msg.role.as_str() {
            "user" => {
                if let ChatContent::Text(ref text) = msg.content {
                    result.push(json!({"role": "user", "content": text}));
                }
            }
            "assistant" => {
                let mut content_blocks: Vec<Value> = Vec::new();

                // Add text block if present
                if let ChatContent::Text(ref text) = msg.content {
                    if !text.is_empty() {
                        content_blocks.push(json!({"type": "text", "text": text}));
                    }
                }

                // Add tool_use blocks
                for tc in &msg.tool_calls {
                    content_blocks.push(json!({
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.name,
                        "input": tc.arguments,
                    }));
                }

                if !content_blocks.is_empty() {
                    result.push(json!({"role": "assistant", "content": content_blocks}));
                }
            }
            "tool_result" => {
                // Anthropic expects tool results as user messages with tool_result content blocks
                let tool_use_id = msg.tool_call_id.as_deref().unwrap_or("");
                let content = match msg.content {
                    ChatContent::Text(ref text) => text.clone(),
                };
                result.push(json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": content,
                    }]
                }));
            }
            _ => {
                // Summary or other — treat as user message
                if let ChatContent::Text(ref text) = msg.content {
                    result.push(json!({"role": "user", "content": text}));
                }
            }
        }
    }

    // Anthropic requires alternating user/assistant. Merge consecutive same-role messages.
    merge_consecutive_roles(&mut result);

    result
}

/// Merge consecutive messages with the same role into a single message
/// with array content blocks. Anthropic requires alternating roles.
fn merge_consecutive_roles(messages: &mut Vec<Value>) {
    if messages.len() < 2 {
        return;
    }

    let mut merged = Vec::new();
    let mut i = 0;

    while i < messages.len() {
        let mut current = messages[i].clone();
        let current_role = current["role"].as_str().unwrap_or("").to_string();

        // Look ahead for consecutive same-role messages
        let mut j = i + 1;
        while j < messages.len() {
            let next_role = messages[j]["role"].as_str().unwrap_or("");
            if next_role != current_role {
                break;
            }

            // Merge: convert both to array content blocks
            let current_content = normalize_content(&current["content"]);
            let next_content = normalize_content(&messages[j]["content"]);

            let mut combined = current_content;
            combined.extend(next_content);

            current["content"] = json!(combined);
            j += 1;
        }

        merged.push(current);
        i = j;
    }

    *messages = merged;
}

/// Normalize content to a Vec<Value> of content blocks.
fn normalize_content(content: &Value) -> Vec<Value> {
    match content {
        Value::String(s) => vec![json!({"type": "text", "text": s})],
        Value::Array(arr) => arr.clone(),
        _ => vec![],
    }
}

/// Convert tool definitions to Anthropic format.
fn build_tools(tools: &[ToolDefinition]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.parameters,
            })
        })
        .collect()
}

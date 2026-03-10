//! StreamingMockBackend — mock that yields proper streaming chunks.

use std::time::Duration;

use async_trait::async_trait;
use futures::stream;

use arawn_llm::{
    CompletionRequest, CompletionResponse, ContentBlock, ContentDelta, LlmBackend, ResponseStream,
    Result, StopReason, StreamEvent, Usage,
};

/// An event that the streaming mock backend can yield.
#[derive(Debug, Clone)]
pub enum StreamingMockEvent {
    /// Text chunk to yield as a TextDelta.
    Text(String),
    /// Tool use block — yields ContentBlockStart(tool_use), InputJsonDelta chunks, ContentBlockStop.
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// A mock backend that yields proper streaming chunks with configurable delays.
///
/// Unlike the standard `MockBackend` which converts sync responses to a stream
/// in one shot, this backend yields individual text deltas with optional delays
/// to simulate real streaming behavior.
#[derive(Debug)]
pub struct StreamingMockBackend {
    /// Events to yield.
    events: Vec<StreamingMockEvent>,
    /// Delay between chunks.
    chunk_delay: Option<Duration>,
    /// Model name to report.
    model: String,
}

impl StreamingMockBackend {
    /// Create a streaming backend that yields the given text chunks.
    pub fn new(chunks: Vec<String>) -> Self {
        let events = chunks.into_iter().map(StreamingMockEvent::Text).collect();
        Self {
            events,
            chunk_delay: None,
            model: "streaming-mock".to_string(),
        }
    }

    /// Create from a single text, split into word-sized chunks.
    pub fn from_text(text: &str) -> Self {
        let chunks: Vec<String> = text.split_whitespace().map(|w| format!("{} ", w)).collect();
        Self::new(chunks)
    }

    /// Create from a full list of streaming events.
    pub fn from_events(events: Vec<StreamingMockEvent>) -> Self {
        Self {
            events,
            chunk_delay: None,
            model: "streaming-mock".to_string(),
        }
    }

    /// Convenience constructor: a tool call followed by a text response.
    pub fn tool_then_text(
        tool_name: &str,
        tool_id: &str,
        args: serde_json::Value,
        text: &str,
    ) -> Self {
        let events = vec![
            StreamingMockEvent::ToolUse {
                id: tool_id.to_string(),
                name: tool_name.to_string(),
                input: args,
            },
            StreamingMockEvent::Text(text.to_string()),
        ];
        Self::from_events(events)
    }

    /// Set delay between chunks.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.chunk_delay = Some(delay);
        self
    }

    /// Set the model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Returns true if any event is a ToolUse.
    fn has_tool_use(&self) -> bool {
        self.events
            .iter()
            .any(|e| matches!(e, StreamingMockEvent::ToolUse { .. }))
    }
}

#[async_trait]
impl LlmBackend for StreamingMockBackend {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        let mut content = Vec::new();

        for event in &self.events {
            match event {
                StreamingMockEvent::Text(text) => {
                    content.push(ContentBlock::Text {
                        text: text.clone(),
                        cache_control: None,
                    });
                }
                StreamingMockEvent::ToolUse { id, name, input } => {
                    content.push(ContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                        cache_control: None,
                    });
                }
            }
        }

        let stop_reason = if self.has_tool_use() {
            StopReason::ToolUse
        } else {
            StopReason::EndTurn
        };

        Ok(CompletionResponse::new(
            "stream_mock_msg",
            &self.model,
            content,
            stop_reason,
            Usage::new(10, 20),
        ))
    }

    async fn complete_stream(&self, _request: CompletionRequest) -> Result<ResponseStream> {
        let model = self.model.clone();
        let events_list = self.events.clone();
        let delay = self.chunk_delay;
        let has_tool_use = self.has_tool_use();

        // Build the full event sequence
        let mut events: Vec<arawn_llm::Result<StreamEvent>> = Vec::new();

        events.push(Ok(StreamEvent::MessageStart {
            id: "stream_mock_msg".to_string(),
            model,
        }));

        let mut block_index: usize = 0;

        for mock_event in &events_list {
            match mock_event {
                StreamingMockEvent::Text(text) => {
                    events.push(Ok(StreamEvent::ContentBlockStart {
                        index: block_index,
                        content_type: "text".to_string(),
                    }));
                    events.push(Ok(StreamEvent::ContentBlockDelta {
                        index: block_index,
                        delta: ContentDelta::TextDelta(text.clone()),
                    }));
                    events.push(Ok(StreamEvent::ContentBlockStop { index: block_index }));
                    block_index += 1;
                }
                StreamingMockEvent::ToolUse { id, name, input } => {
                    events.push(Ok(StreamEvent::ContentBlockStart {
                        index: block_index,
                        content_type: "tool_use".to_string(),
                    }));

                    // Chunk the JSON input into pieces for InputJsonDelta
                    let json_str = serde_json::to_string(input).unwrap_or_default();
                    let chunk_size = 20;
                    let mut pos = 0;
                    while pos < json_str.len() {
                        let end = (pos + chunk_size).min(json_str.len());
                        let chunk = &json_str[pos..end];
                        events.push(Ok(StreamEvent::ContentBlockDelta {
                            index: block_index,
                            delta: ContentDelta::InputJsonDelta(chunk.to_string()),
                        }));
                        pos = end;
                    }

                    // If input was empty object, still emit at least one delta
                    if json_str.is_empty() {
                        events.push(Ok(StreamEvent::ContentBlockDelta {
                            index: block_index,
                            delta: ContentDelta::InputJsonDelta("{}".to_string()),
                        }));
                    }

                    let _ = (id, name); // used in ContentBlockStart context (tool_use type)

                    events.push(Ok(StreamEvent::ContentBlockStop { index: block_index }));
                    block_index += 1;
                }
            }
        }

        let stop_reason = if has_tool_use {
            StopReason::ToolUse
        } else {
            StopReason::EndTurn
        };

        events.push(Ok(StreamEvent::MessageDelta {
            stop_reason,
            usage: Usage::new(10, 20),
        }));

        events.push(Ok(StreamEvent::MessageStop));

        if let Some(delay) = delay {
            // Yield events with delays
            let stream = async_stream::stream! {
                for event in events {
                    tokio::time::sleep(delay).await;
                    yield event;
                }
            };
            Ok(Box::pin(stream))
        } else {
            // Yield all events immediately
            Ok(Box::pin(stream::iter(events)))
        }
    }

    fn name(&self) -> &str {
        "streaming-mock"
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ScriptedMockBackend
// ─────────────────────────────────────────────────────────────────────────────

/// A single scripted invocation: either a set of events or an error.
#[derive(Debug, Clone)]
pub enum ScriptedInvocation {
    /// Return these events as a successful response.
    Events(Vec<StreamingMockEvent>),
    /// Return this error from both `complete()` and `complete_stream()`.
    Error(String),
}

impl From<Vec<StreamingMockEvent>> for ScriptedInvocation {
    fn from(events: Vec<StreamingMockEvent>) -> Self {
        Self::Events(events)
    }
}

/// A mock backend that returns different responses on each logical invocation.
///
/// Unlike `StreamingMockBackend` which returns the same events every time,
/// this backend advances through a scripted sequence of responses, enabling
/// multi-turn tool-execution flows where the agent calls the LLM multiple
/// times per turn.
///
/// **Important**: The agent's streaming loop calls both `complete_stream()` and
/// `complete()` for each iteration (stream for deltas, sync for tool-use check).
/// This backend tracks whether the last call was a stream call so that the
/// paired sync call returns the same events without advancing the index.
///
/// # Example: Tool-use flow
/// ```rust,ignore
/// let backend = ScriptedMockBackend::new(vec![
///     // Invocation 0: LLM requests tool use
///     vec![StreamingMockEvent::ToolUse {
///         id: "tc-1".into(),
///         name: "echo".into(),
///         input: json!({"message": "hello"}),
///     }],
///     // Invocation 1: LLM responds with text after seeing tool result
///     vec![StreamingMockEvent::Text("The tool said hello.".into())],
/// ]);
/// ```
#[derive(Debug)]
pub struct ScriptedMockBackend {
    /// List of invocations (indexed, not consumed).
    invocations: Vec<ScriptedInvocation>,
    /// Current invocation index.
    index: std::sync::Mutex<usize>,
    /// Whether the last call was `complete_stream` — if so, a subsequent
    /// `complete()` returns the same invocation without advancing.
    last_was_stream: std::sync::atomic::AtomicBool,
    /// Delay between chunks.
    chunk_delay: Option<Duration>,
    /// Model name.
    model: String,
}

impl ScriptedMockBackend {
    /// Create from a sequence of invocations.
    ///
    /// Each inner `Vec<StreamingMockEvent>` represents one logical LLM invocation.
    /// Both `complete_stream()` and `complete()` return the same events within
    /// a single invocation.
    pub fn new(invocations: Vec<Vec<StreamingMockEvent>>) -> Self {
        Self {
            invocations: invocations
                .into_iter()
                .map(ScriptedInvocation::Events)
                .collect(),
            index: std::sync::Mutex::new(0),
            last_was_stream: std::sync::atomic::AtomicBool::new(false),
            chunk_delay: None,
            model: "scripted-mock".to_string(),
        }
    }

    /// Create from a sequence of invocations that may include errors.
    pub fn from_invocations(invocations: Vec<ScriptedInvocation>) -> Self {
        Self {
            invocations,
            index: std::sync::Mutex::new(0),
            last_was_stream: std::sync::atomic::AtomicBool::new(false),
            chunk_delay: None,
            model: "scripted-mock".to_string(),
        }
    }

    /// Convenience: tool call on first invocation, text on second.
    pub fn tool_then_text(
        tool_name: &str,
        tool_id: &str,
        args: serde_json::Value,
        response_text: &str,
    ) -> Self {
        Self::new(vec![
            vec![StreamingMockEvent::ToolUse {
                id: tool_id.to_string(),
                name: tool_name.to_string(),
                input: args,
            }],
            vec![StreamingMockEvent::Text(response_text.to_string())],
        ])
    }

    /// Convenience: returns an error on every call.
    pub fn always_error(message: &str) -> Self {
        Self::from_invocations(vec![ScriptedInvocation::Error(message.to_string())])
    }

    /// Set delay between chunks.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.chunk_delay = Some(delay);
        self
    }

    /// Set the model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Get the invocation for a streaming call.
    ///
    /// Returns the invocation at the current index and marks that the last call
    /// was a stream call. Does NOT advance the index — the subsequent `complete()`
    /// call will return the same invocation and then advance.
    fn invocation_for_stream(&self) -> ScriptedInvocation {
        self.last_was_stream
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let index = *self.index.lock().unwrap();
        if index < self.invocations.len() {
            self.invocations[index].clone()
        } else {
            ScriptedInvocation::Events(vec![StreamingMockEvent::Text(
                "(no more scripted responses)".to_string(),
            )])
        }
    }

    /// Get the invocation for a sync (complete) call.
    ///
    /// Returns the invocation at the current index and always advances.
    fn invocation_for_sync(&self) -> ScriptedInvocation {
        self.last_was_stream
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let mut idx = self.index.lock().unwrap();
        let index = *idx;
        let invocation = if index < self.invocations.len() {
            self.invocations[index].clone()
        } else {
            ScriptedInvocation::Events(vec![StreamingMockEvent::Text(
                "(no more scripted responses)".to_string(),
            )])
        };
        *idx += 1;
        invocation
    }

    /// Check if any event in a list is a ToolUse.
    fn events_have_tool_use(events: &[StreamingMockEvent]) -> bool {
        events
            .iter()
            .any(|e| matches!(e, StreamingMockEvent::ToolUse { .. }))
    }
}

#[async_trait]
impl LlmBackend for ScriptedMockBackend {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        let invocation = self.invocation_for_sync();

        let events = match invocation {
            ScriptedInvocation::Events(events) => events,
            ScriptedInvocation::Error(msg) => {
                return Err(arawn_llm::LlmError::Backend(msg));
            }
        };

        let has_tool_use = Self::events_have_tool_use(&events);

        let mut content = Vec::new();
        for event in &events {
            match event {
                StreamingMockEvent::Text(text) => {
                    content.push(ContentBlock::Text {
                        text: text.clone(),
                        cache_control: None,
                    });
                }
                StreamingMockEvent::ToolUse { id, name, input } => {
                    content.push(ContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                        cache_control: None,
                    });
                }
            }
        }

        let stop_reason = if has_tool_use {
            StopReason::ToolUse
        } else {
            StopReason::EndTurn
        };

        Ok(CompletionResponse::new(
            "scripted_mock_msg",
            &self.model,
            content,
            stop_reason,
            Usage::new(10, 20),
        ))
    }

    async fn complete_stream(&self, _request: CompletionRequest) -> Result<ResponseStream> {
        let invocation = self.invocation_for_stream();

        let events_list = match invocation {
            ScriptedInvocation::Events(events) => events,
            ScriptedInvocation::Error(msg) => {
                return Err(arawn_llm::LlmError::Backend(msg));
            }
        };
        let has_tool_use = Self::events_have_tool_use(&events_list);
        let model = self.model.clone();
        let delay = self.chunk_delay;

        let mut stream_events: Vec<arawn_llm::Result<StreamEvent>> = Vec::new();

        stream_events.push(Ok(StreamEvent::MessageStart {
            id: "scripted_mock_msg".to_string(),
            model,
        }));

        let mut block_index: usize = 0;
        for mock_event in &events_list {
            match mock_event {
                StreamingMockEvent::Text(text) => {
                    stream_events.push(Ok(StreamEvent::ContentBlockStart {
                        index: block_index,
                        content_type: "text".to_string(),
                    }));
                    stream_events.push(Ok(StreamEvent::ContentBlockDelta {
                        index: block_index,
                        delta: ContentDelta::TextDelta(text.clone()),
                    }));
                    stream_events.push(Ok(StreamEvent::ContentBlockStop { index: block_index }));
                    block_index += 1;
                }
                StreamingMockEvent::ToolUse {
                    id: _,
                    name: _,
                    input,
                } => {
                    stream_events.push(Ok(StreamEvent::ContentBlockStart {
                        index: block_index,
                        content_type: "tool_use".to_string(),
                    }));
                    let json_str = serde_json::to_string(input).unwrap_or_default();
                    let chunk_size = 20;
                    let mut pos = 0;
                    while pos < json_str.len() {
                        let end = (pos + chunk_size).min(json_str.len());
                        let chunk = &json_str[pos..end];
                        stream_events.push(Ok(StreamEvent::ContentBlockDelta {
                            index: block_index,
                            delta: ContentDelta::InputJsonDelta(chunk.to_string()),
                        }));
                        pos = end;
                    }
                    if json_str.is_empty() {
                        stream_events.push(Ok(StreamEvent::ContentBlockDelta {
                            index: block_index,
                            delta: ContentDelta::InputJsonDelta("{}".to_string()),
                        }));
                    }
                    stream_events.push(Ok(StreamEvent::ContentBlockStop { index: block_index }));
                    block_index += 1;
                }
            }
        }

        let stop_reason = if has_tool_use {
            StopReason::ToolUse
        } else {
            StopReason::EndTurn
        };

        stream_events.push(Ok(StreamEvent::MessageDelta {
            stop_reason,
            usage: Usage::new(10, 20),
        }));
        stream_events.push(Ok(StreamEvent::MessageStop));

        if let Some(delay) = delay {
            let stream = async_stream::stream! {
                for event in stream_events {
                    tokio::time::sleep(delay).await;
                    yield event;
                }
            };
            Ok(Box::pin(stream))
        } else {
            Ok(Box::pin(stream::iter(stream_events)))
        }
    }

    fn name(&self) -> &str {
        "scripted-mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_streaming_mock_yields_chunks() {
        let backend = StreamingMockBackend::new(vec!["Hello ".to_string(), "world!".to_string()]);

        let request = CompletionRequest::new("mock", vec![arawn_llm::Message::user("Hi")], 1024);

        let mut stream = backend.complete_stream(request).await.unwrap();

        let mut text_deltas = Vec::new();
        while let Some(Ok(event)) = stream.next().await {
            if let StreamEvent::ContentBlockDelta {
                delta: ContentDelta::TextDelta(text),
                ..
            } = event
            {
                text_deltas.push(text);
            }
        }

        assert_eq!(text_deltas, vec!["Hello ", "world!"]);
    }

    #[tokio::test]
    async fn test_streaming_mock_complete_combines() {
        let backend = StreamingMockBackend::new(vec!["Hello ".to_string(), "world!".to_string()]);

        let request = CompletionRequest::new("mock", vec![arawn_llm::Message::user("Hi")], 1024);

        let response = backend.complete(request).await.unwrap();
        // Each text chunk becomes its own ContentBlock::Text
        assert_eq!(response.content.len(), 2);
    }

    #[tokio::test]
    async fn test_streaming_mock_from_text() {
        let backend = StreamingMockBackend::from_text("The quick brown fox");

        let request = CompletionRequest::new("mock", vec![arawn_llm::Message::user("Hi")], 1024);

        let mut stream = backend.complete_stream(request).await.unwrap();

        let mut chunks = Vec::new();
        while let Some(Ok(event)) = stream.next().await {
            if let StreamEvent::ContentBlockDelta {
                delta: ContentDelta::TextDelta(text),
                ..
            } = event
            {
                chunks.push(text);
            }
        }

        assert_eq!(chunks.len(), 4); // 4 words
    }

    #[tokio::test]
    async fn test_tool_then_text() {
        let backend = StreamingMockBackend::tool_then_text(
            "read_file",
            "tool_1",
            serde_json::json!({"path": "/foo.rs"}),
            "Here is the file content.",
        );

        let request =
            CompletionRequest::new("mock", vec![arawn_llm::Message::user("Read foo.rs")], 1024);

        // Test streaming
        let mut stream = backend.complete_stream(request.clone()).await.unwrap();

        let mut saw_tool_use_start = false;
        let mut json_parts = Vec::new();
        let mut text_parts = Vec::new();
        let mut block_types = Vec::new();

        while let Some(Ok(event)) = stream.next().await {
            match event {
                StreamEvent::ContentBlockStart { content_type, .. } => {
                    block_types.push(content_type.clone());
                    if content_type == "tool_use" {
                        saw_tool_use_start = true;
                    }
                }
                StreamEvent::ContentBlockDelta { delta, .. } => match delta {
                    ContentDelta::InputJsonDelta(json) => json_parts.push(json),
                    ContentDelta::TextDelta(text) => text_parts.push(text),
                },
                StreamEvent::MessageDelta { stop_reason, .. } => {
                    assert_eq!(stop_reason, StopReason::ToolUse);
                }
                _ => {}
            }
        }

        assert!(saw_tool_use_start);
        assert_eq!(block_types, vec!["tool_use", "text"]);
        assert!(!json_parts.is_empty());
        assert_eq!(text_parts, vec!["Here is the file content."]);

        // Reassemble JSON and verify
        let reassembled: String = json_parts.into_iter().collect();
        let parsed: serde_json::Value = serde_json::from_str(&reassembled).unwrap();
        assert_eq!(parsed, serde_json::json!({"path": "/foo.rs"}));

        // Test sync complete
        let response = backend.complete(request).await.unwrap();
        assert!(response.has_tool_use());
        assert_eq!(response.stop_reason, Some(StopReason::ToolUse));
        assert_eq!(response.content.len(), 2);
    }
}

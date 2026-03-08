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

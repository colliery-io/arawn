use std::pin::Pin;
use std::sync::Mutex;

use async_trait::async_trait;
use futures::stream;

use crate::client::LlmClient;
use crate::error::LlmError;
use crate::types::{ChatChunk, ChatRequest};

/// A scripted response for one LLM turn.
pub enum MockResponse {
    /// Pure text response (no tool calls).
    Text(String),
    /// A single tool call.
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    /// Raw chunks — full control over exactly what the mock returns.
    Raw(Vec<ChatChunk>),
    /// Immediate error — `stream()` returns `Err(error)` without producing a stream.
    Error(LlmError),
    /// Mid-stream error — yields Ok chunks then a final Err item.
    StreamError {
        chunks_before_error: Vec<ChatChunk>,
        error: LlmError,
    },
}

impl MockResponse {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    pub fn tool_call(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self::ToolCall {
            id: id.into(),
            name: name.into(),
            arguments: arguments.into(),
        }
    }

    pub fn raw(chunks: Vec<ChatChunk>) -> Self {
        Self::Raw(chunks)
    }

    pub fn error(error: LlmError) -> Self {
        Self::Error(error)
    }

    pub fn stream_error(chunks_before_error: Vec<ChatChunk>, error: LlmError) -> Self {
        Self::StreamError {
            chunks_before_error,
            error,
        }
    }

    fn into_chunks(self) -> Vec<ChatChunk> {
        match self {
            Self::Text(text) => vec![
                ChatChunk::TextDelta { text },
                ChatChunk::Done { usage: None },
            ],
            Self::ToolCall {
                id,
                name,
                arguments,
            } => vec![
                ChatChunk::ToolUseStart { id, name },
                ChatChunk::ToolUseInputDelta { json: arguments },
                ChatChunk::Done { usage: None },
            ],
            Self::Raw(chunks) => chunks,
            Self::Error(_) | Self::StreamError { .. } => {
                unreachable!("Error/StreamError variants are handled directly in stream()")
            }
        }
    }
}

/// Mock LLM client that returns pre-scripted responses.
/// Each call to `stream()` consumes the next scripted response.
/// Panics if more calls are made than responses available.
pub struct MockLlmClient {
    responses: Mutex<Vec<MockResponse>>,
    call_count: Mutex<usize>,
    captured_requests: Mutex<Vec<ChatRequest>>,
}

impl MockLlmClient {
    pub fn new(responses: Vec<MockResponse>) -> Self {
        Self {
            responses: Mutex::new(responses),
            call_count: Mutex::new(0),
            captured_requests: Mutex::new(Vec::new()),
        }
    }

    /// How many times `stream()` has been called.
    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }

    /// Returns a clone of all captured requests for test assertions.
    pub fn captured_requests(&self) -> Vec<ChatRequest> {
        self.captured_requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError>
    {
        self.captured_requests.lock().unwrap().push(request);
        let mut responses = self.responses.lock().unwrap();
        let mut count = self.call_count.lock().unwrap();
        *count += 1;

        if responses.is_empty() {
            panic!(
                "MockLlmClient: no more scripted responses (call #{})",
                *count
            );
        }

        let response = responses.remove(0);

        match response {
            MockResponse::Error(error) => Err(error),
            MockResponse::StreamError {
                chunks_before_error,
                error,
            } => {
                let ok_items: Vec<Result<ChatChunk, LlmError>> =
                    chunks_before_error.into_iter().map(Ok).collect();
                let err_item: Vec<Result<ChatChunk, LlmError>> = vec![Err(error)];
                let all_items = ok_items.into_iter().chain(err_item);
                Ok(Box::pin(stream::iter(all_items)))
            }
            other => {
                let chunks = other.into_chunks();
                Ok(Box::pin(stream::iter(chunks.into_iter().map(Ok))))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn mock_text_response() {
        let mock = MockLlmClient::new(vec![MockResponse::text("hello")]);
        let request = ChatRequest {
            model: "test".into(),
            system_prompt: None,
            messages: vec![],
            tools: vec![],
            max_tokens: None,
        };

        let mut stream = mock.stream(request).await.unwrap();
        let mut text = String::new();
        while let Some(Ok(chunk)) = stream.next().await {
            if let ChatChunk::TextDelta { text: t } = chunk {
                text.push_str(&t);
            }
        }
        assert_eq!(text, "hello");
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn mock_tool_call_response() {
        let mock = MockLlmClient::new(vec![MockResponse::tool_call(
            "call_1",
            "think",
            r#"{"thought":"test"}"#,
        )]);
        let request = ChatRequest {
            model: "test".into(),
            system_prompt: None,
            messages: vec![],
            tools: vec![],
            max_tokens: None,
        };

        let mut stream = mock.stream(request).await.unwrap();
        let mut got_start = false;
        let mut got_delta = false;
        while let Some(Ok(chunk)) = stream.next().await {
            match chunk {
                ChatChunk::ToolUseStart { name, .. } => {
                    assert_eq!(name, "think");
                    got_start = true;
                }
                ChatChunk::ToolUseInputDelta { json } => {
                    assert!(json.contains("thought"));
                    got_delta = true;
                }
                _ => {}
            }
        }
        assert!(got_start);
        assert!(got_delta);
    }

    #[tokio::test]
    async fn mock_multiple_responses_consumed_in_order() {
        let mock = MockLlmClient::new(vec![
            MockResponse::text("first"),
            MockResponse::text("second"),
        ]);
        let request = ChatRequest {
            model: "test".into(),
            system_prompt: None,
            messages: vec![],
            tools: vec![],
            max_tokens: None,
        };

        // First call
        let mut stream = mock.stream(request.clone()).await.unwrap();
        let mut text = String::new();
        while let Some(Ok(ChatChunk::TextDelta { text: t })) = stream.next().await {
            text.push_str(&t);
        }
        assert_eq!(text, "first");

        // Second call
        let mut stream = mock.stream(request).await.unwrap();
        let mut text = String::new();
        while let Some(Ok(ChatChunk::TextDelta { text: t })) = stream.next().await {
            text.push_str(&t);
        }
        assert_eq!(text, "second");
        assert_eq!(mock.call_count(), 2);
    }

    #[tokio::test]
    async fn mock_error_returns_err_immediately() {
        let mock = MockLlmClient::new(vec![MockResponse::error(
            crate::error::LlmError::Auth("invalid key".into()),
        )]);
        let request = ChatRequest {
            model: "test".into(),
            system_prompt: None,
            messages: vec![],
            tools: vec![],
            max_tokens: None,
        };

        let result = mock.stream(request).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            err.to_string().contains("invalid key"),
            "expected auth error, got: {err}"
        );
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn mock_stream_error_yields_chunks_then_err() {
        let mock = MockLlmClient::new(vec![MockResponse::stream_error(
            vec![
                ChatChunk::TextDelta {
                    text: "partial".into(),
                },
                ChatChunk::TextDelta {
                    text: " output".into(),
                },
            ],
            crate::error::LlmError::Stream("connection reset".into()),
        )]);
        let request = ChatRequest {
            model: "test".into(),
            system_prompt: None,
            messages: vec![],
            tools: vec![],
            max_tokens: None,
        };

        let mut stream = mock.stream(request).await.unwrap();
        let mut text = String::new();
        let mut got_error = false;
        while let Some(item) = stream.next().await {
            match item {
                Ok(ChatChunk::TextDelta { text: t }) => text.push_str(&t),
                Err(e) => {
                    assert!(e.to_string().contains("connection reset"));
                    got_error = true;
                }
                _ => {}
            }
        }
        assert_eq!(text, "partial output");
        assert!(got_error, "expected stream error after chunks");
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn mock_error_then_success_simulates_retry() {
        let mock = MockLlmClient::new(vec![
            MockResponse::error(crate::error::LlmError::RateLimited("slow down".into())),
            MockResponse::text("recovered"),
        ]);
        let request = ChatRequest {
            model: "test".into(),
            system_prompt: None,
            messages: vec![],
            tools: vec![],
            max_tokens: None,
        };

        // First call: error
        let result = mock.stream(request.clone()).await;
        assert!(result.is_err());

        // Second call: success
        let mut stream = mock.stream(request).await.unwrap();
        let mut text = String::new();
        while let Some(Ok(ChatChunk::TextDelta { text: t })) = stream.next().await {
            text.push_str(&t);
        }
        assert_eq!(text, "recovered");
        assert_eq!(mock.call_count(), 2);
    }

    #[tokio::test]
    #[should_panic(expected = "no more scripted responses")]
    async fn mock_panics_when_exhausted() {
        let mock = MockLlmClient::new(vec![]);
        let request = ChatRequest {
            model: "test".into(),
            system_prompt: None,
            messages: vec![],
            tools: vec![],
            max_tokens: None,
        };
        let _ = mock.stream(request).await;
    }
}

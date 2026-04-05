use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;
use tracing::{info, warn};

use crate::client::LlmClient;
use crate::error::LlmError;
use crate::types::{ChatChunk, ChatRequest};

const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_BASE_DELAY_MS: u64 = 1000;

/// Wraps any LlmClient and adds retry with exponential backoff for transient errors.
pub struct RetryClient {
    inner: Arc<dyn LlmClient>,
    max_retries: u32,
    base_delay_ms: u64,
}

impl RetryClient {
    pub fn new(inner: Arc<dyn LlmClient>) -> Self {
        Self {
            inner,
            max_retries: DEFAULT_MAX_RETRIES,
            base_delay_ms: DEFAULT_BASE_DELAY_MS,
        }
    }

    pub fn with_config(inner: Arc<dyn LlmClient>, max_retries: u32, base_delay_ms: u64) -> Self {
        Self {
            inner,
            max_retries,
            base_delay_ms,
        }
    }

    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        Duration::from_millis(self.base_delay_ms * 2u64.pow(attempt))
    }
}

#[async_trait]
impl LlmClient for RetryClient {
    async fn stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = self.delay_for_attempt(attempt - 1);
                info!(
                    attempt,
                    delay_ms = delay.as_millis() as u64,
                    "retrying LLM request"
                );
                tokio::time::sleep(delay).await;
            }

            match self.inner.stream(request.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    if e.is_retryable() && attempt < self.max_retries {
                        warn!(
                            attempt,
                            max_retries = self.max_retries,
                            error = %e,
                            "transient LLM error, will retry"
                        );
                        last_error = Some(e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::Api("max retries exceeded".into())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockLlmClient, MockResponse};
    use crate::types::ChatRequest;
    use futures::StreamExt;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A mock that fails N times then succeeds.
    struct FailThenSucceed {
        failures_remaining: Mutex<u32>,
        error_type: LlmError,
        success_response: Vec<ChatChunk>,
    }

    #[async_trait]
    impl LlmClient for FailThenSucceed {
        async fn stream(
            &self,
            _request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError>
        {
            let mut remaining = self.failures_remaining.lock().unwrap();
            if *remaining > 0 {
                *remaining -= 1;
                Err(LlmError::ServerError("500 Internal Server Error".into()))
            } else {
                let chunks = self.success_response.clone();
                Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
            }
        }
    }

    fn dummy_request() -> ChatRequest {
        ChatRequest {
            model: "test".into(),
            system_prompt: None,
            messages: vec![],
            tools: vec![],
            max_tokens: None,
        }
    }

    #[tokio::test]
    async fn succeeds_on_first_try() {
        let mock = Arc::new(MockLlmClient::new(vec![MockResponse::text("ok")]));
        let client = RetryClient::new(mock);

        let mut stream = client.stream(dummy_request()).await.unwrap();
        let mut text = String::new();
        while let Some(Ok(ChatChunk::TextDelta { text: t })) = stream.next().await {
            text.push_str(&t);
        }
        assert_eq!(text, "ok");
    }

    #[tokio::test]
    async fn retries_on_server_error_then_succeeds() {
        let inner = Arc::new(FailThenSucceed {
            failures_remaining: Mutex::new(2),
            error_type: LlmError::ServerError("500".into()),
            success_response: vec![
                ChatChunk::TextDelta {
                    text: "recovered".into(),
                },
                ChatChunk::Done { usage: None },
            ],
        });

        let client = RetryClient::with_config(inner, 3, 10); // 10ms delays for fast tests

        let mut stream = client.stream(dummy_request()).await.unwrap();
        let mut text = String::new();
        while let Some(Ok(ChatChunk::TextDelta { text: t })) = stream.next().await {
            text.push_str(&t);
        }
        assert_eq!(text, "recovered");
    }

    #[tokio::test]
    async fn gives_up_after_max_retries() {
        let inner = Arc::new(FailThenSucceed {
            failures_remaining: Mutex::new(10), // will never succeed
            error_type: LlmError::ServerError("500".into()),
            success_response: vec![],
        });

        let client = RetryClient::with_config(inner, 2, 10);

        let result = client.stream(dummy_request()).await;
        match result {
            Err(LlmError::ServerError(_)) => {} // expected
            Err(other) => panic!("expected ServerError, got: {other}"),
            Ok(_) => panic!("expected error"),
        }
    }

    #[tokio::test]
    async fn does_not_retry_terminal_errors() {
        // API errors (4xx non-429) are not retryable
        struct AlwaysBadRequest;

        #[async_trait]
        impl LlmClient for AlwaysBadRequest {
            async fn stream(
                &self,
                _request: ChatRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError>
            {
                Err(LlmError::Api("400 Bad Request: invalid model".into()))
            }
        }

        let inner = Arc::new(AlwaysBadRequest);
        let client = RetryClient::with_config(inner, 3, 10);

        let result = client.stream(dummy_request()).await;
        match result {
            Err(LlmError::Api(_)) => {} // terminal, no retry
            Err(other) => panic!("expected Api error, got: {other}"),
            Ok(_) => panic!("expected error"),
        }
    }

    #[tokio::test]
    async fn retries_rate_limit_errors() {
        let inner = Arc::new(FailThenSucceed {
            failures_remaining: Mutex::new(1),
            error_type: LlmError::RateLimited("429".into()),
            success_response: vec![
                ChatChunk::TextDelta {
                    text: "after rate limit".into(),
                },
                ChatChunk::Done { usage: None },
            ],
        });

        // Override the mock to return RateLimited
        struct RateLimitThenSucceed {
            inner: FailThenSucceed,
        }

        #[async_trait]
        impl LlmClient for RateLimitThenSucceed {
            async fn stream(
                &self,
                request: ChatRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError>
            {
                let mut remaining = self.inner.failures_remaining.lock().unwrap();
                if *remaining > 0 {
                    *remaining -= 1;
                    Err(LlmError::RateLimited("429 Too Many Requests".into()))
                } else {
                    let chunks = self.inner.success_response.clone();
                    Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
                }
            }
        }

        let client = RetryClient::with_config(
            Arc::new(RateLimitThenSucceed {
                inner: FailThenSucceed {
                    failures_remaining: Mutex::new(1),
                    error_type: LlmError::RateLimited("429".into()),
                    success_response: vec![
                        ChatChunk::TextDelta {
                            text: "after rate limit".into(),
                        },
                        ChatChunk::Done { usage: None },
                    ],
                },
            }),
            3,
            10,
        );

        let mut stream = client.stream(dummy_request()).await.unwrap();
        let mut text = String::new();
        while let Some(Ok(ChatChunk::TextDelta { text: t })) = stream.next().await {
            text.push_str(&t);
        }
        assert_eq!(text, "after rate limit");
    }
}

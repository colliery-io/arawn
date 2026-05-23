//! `LlmClient` decorator that records every completion's token usage.
//!
//! Wraps another `LlmClient` (typically already wrapped by
//! `RetryClient`). Forwards every method through, but inspects each
//! streamed `ChatChunk::Done { usage }` chunk: if `usage` is `Some`,
//! a [`TokenUsageRecord`] is appended via [`super::record`].
//!
//! When the stream ends without a `Done` chunk (provider error,
//! cancellation, etc.), no record is written and a tracing event
//! marks the gap so the operator knows the log under-counts.

use std::pin::Pin;

use async_stream::try_stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt};

use crate::client::LlmClient;
use crate::error::LlmError;
use crate::types::{ChatChunk, ChatRequest};

use super::{TokenUsageRecord, now_secs, record};

/// Decorator client that records usage on each completed stream.
pub struct UsageTrackingClient {
    inner: std::sync::Arc<dyn LlmClient>,
    /// Provider name attached to recorded events. Captured at
    /// construction (the underlying client doesn't surface it).
    provider: String,
    /// Optional call-site tag. When set, every event records it;
    /// when unset, the caller can pass it through other channels
    /// (or it lands as `None`).
    call_site: Option<String>,
}

impl UsageTrackingClient {
    /// Wrap a client. `provider` is the stable provider name
    /// (`"groq"`, `"anthropic"`, …) used in the recorded events.
    pub fn new(inner: std::sync::Arc<dyn LlmClient>, provider: impl Into<String>) -> Self {
        Self {
            inner,
            provider: provider.into(),
            call_site: None,
        }
    }

    /// Attach a call-site tag to every event recorded through this
    /// client. Useful for the few paths that have a stable name
    /// (compactor, steward subroutines, …). Most consumers leave
    /// it unset.
    pub fn with_call_site(mut self, site: impl Into<String>) -> Self {
        self.call_site = Some(site.into());
        self
    }
}

#[async_trait]
impl LlmClient for UsageTrackingClient {
    async fn stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError> {
        let model = request.model.clone();
        let provider = self.provider.clone();
        let call_site = self.call_site.clone();
        let upstream = self.inner.stream(request).await?;

        let s = try_stream! {
            let mut upstream = upstream;
            let mut recorded = false;
            while let Some(chunk) = upstream.next().await {
                let chunk = chunk?;
                if let ChatChunk::Done { usage: Some(usage) } = &chunk {
                    record(TokenUsageRecord {
                        ts: now_secs(),
                        provider: provider.clone(),
                        model: model.clone(),
                        prompt_tokens: usage.input_tokens,
                        completion_tokens: usage.output_tokens,
                        call_site: call_site.clone(),
                    });
                    recorded = true;
                }
                yield chunk;
            }
            if !recorded {
                tracing::debug!(
                    %provider, %model,
                    "LLM stream ended without Done{{usage}} — token usage not recorded"
                );
            }
        };
        Ok(Box::pin(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockLlmClient, MockResponse};
    use crate::types::Usage;
    use crate::usage::{UsagePeriod, UsageTracker, install};
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::Mutex;

    // A coarse serial lock for tests that install the global tracker.
    // Cargo runs lib tests in parallel; the global OnceLock can only
    // be installed once per process so tests must share an instance.
    static GLOBAL_INIT: Mutex<bool> = Mutex::const_new(false);

    async fn ensure_global_tracker_for_tests(tmp_path: &std::path::Path) -> Arc<UsageTracker> {
        let mut guard = GLOBAL_INIT.lock().await;
        let tracker = Arc::new(UsageTracker::open(tmp_path));
        if !*guard {
            install(tracker.clone());
            *guard = true;
        }
        tracker
    }

    fn make_request(model: &str) -> ChatRequest {
        ChatRequest {
            model: model.to_string(),
            system_prompt: None,
            messages: vec![],
            tools: vec![],
            max_tokens: None,
        }
    }

    #[tokio::test]
    async fn records_usage_from_done_chunk() {
        let tmp = TempDir::new().unwrap();
        let _global = ensure_global_tracker_for_tests(tmp.path()).await;
        // Build a mock that emits a Done chunk with usage.
        let mock = Arc::new(MockLlmClient::new(vec![MockResponse::raw(vec![
            ChatChunk::TextDelta { text: "hi".into() },
            ChatChunk::Done {
                usage: Some(Usage {
                    input_tokens: 7,
                    output_tokens: 3,
                }),
            },
        ])]));
        let wrapped = UsageTrackingClient::new(mock, "mock-provider");
        let mut stream = wrapped.stream(make_request("test-model")).await.unwrap();
        while let Some(c) = stream.next().await {
            c.unwrap();
        }
        // The global tracker is the test instance we installed — but
        // because tests share it, we can't assert about a clean log.
        // Instead, instantiate a fresh local tracker pointed at the
        // same temp dir and verify the record landed.
        let local = UsageTracker::open(tmp.path());
        let summary = local.summary(UsagePeriod::All, Some("test-model"), false);
        assert!(
            summary.total_calls >= 1,
            "expected at least one recorded call for test-model, got {summary:?}"
        );
        let row = summary
            .models
            .iter()
            .find(|m| m.model == "test-model")
            .unwrap();
        assert!(row.total_prompt_tokens >= 7);
        assert!(row.total_completion_tokens >= 3);
    }
}

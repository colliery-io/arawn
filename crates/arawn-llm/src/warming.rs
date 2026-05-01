//! Wrapper that caches model warmup state and re-warms on TTL expiry or cold-restart errors.
//!
//! `WarmingClient` sits above `RetryClient` in the client stack. It probes the
//! configured model with a 1-token request the first time `stream` is called,
//! after the cached warmup goes stale, and once after a request fails with a
//! signature consistent with the model being unloaded by the provider.
//!
//! Pool layering: raw provider → `RetryClient` → `WarmingClient`.

use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::Stream;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::client::LlmClient;
use crate::error::LlmError;
use crate::types::{ChatChunk, ChatRequest};

/// Default TTL chosen for Ollama Cloud, which unloads idle models aggressively.
/// Local Ollama or Groq could safely use a much higher TTL, but a single
/// conservative default avoids per-provider config until we have evidence
/// it matters.
pub const DEFAULT_WARMUP_TTL: Duration = Duration::from_secs(4 * 60);

/// Wraps any [`LlmClient`] with TTL-based warmup caching and a one-shot
/// retry-after-warmup on cold-restart-shaped errors.
pub struct WarmingClient {
    inner: Arc<dyn LlmClient>,
    /// Used both to identify the client in logs and so warmup logs include
    /// what was actually probed.
    provider: String,
    ttl: Duration,
    /// Last successful warmup timestamp. `None` = never warmed (or warmup
    /// invalidated by a cold-restart error).
    last_warmup: Mutex<Option<Instant>>,
}

impl WarmingClient {
    pub fn new(inner: Arc<dyn LlmClient>, provider: impl Into<String>) -> Self {
        Self::with_ttl(inner, provider, DEFAULT_WARMUP_TTL)
    }

    pub fn with_ttl(
        inner: Arc<dyn LlmClient>,
        provider: impl Into<String>,
        ttl: Duration,
    ) -> Self {
        Self {
            inner,
            provider: provider.into(),
            ttl,
            last_warmup: Mutex::new(None),
        }
    }

    /// Ensure the cached warmup is fresh. Probes the model if the cache is
    /// empty or expired. No-op when fresh.
    async fn ensure_warm(&self, model: &str) -> Result<(), LlmError> {
        let guard = self.last_warmup.lock().await;
        if let Some(t) = *guard
            && t.elapsed() < self.ttl
        {
            return Ok(());
        }
        // Drop the lock around the actual probe so other callers don't pile up.
        drop(guard);

        debug!(provider = %self.provider, model = %model, "warming up LLM");
        self.inner.warmup(model).await?;
        let mut guard = self.last_warmup.lock().await;
        *guard = Some(Instant::now());
        Ok(())
    }

    async fn invalidate(&self) {
        let mut guard = self.last_warmup.lock().await;
        *guard = None;
    }

    /// Returns the cached `last_warmup` timestamp. Test-only.
    #[cfg(test)]
    pub async fn last_warmup_for_test(&self) -> Option<Instant> {
        *self.last_warmup.lock().await
    }
}

/// Errors that look like the provider unloaded the model and the next request
/// will need a fresh warmup. Conservative initial set: HTTP 503 from the
/// upstream API.
fn looks_like_cold_restart(err: &LlmError) -> bool {
    matches!(err, LlmError::ServerError(msg) if msg.contains("HTTP 503"))
}

#[async_trait]
impl LlmClient for WarmingClient {
    async fn stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError> {
        // Lazy warmup before first/stale request.
        if let Err(e) = self.ensure_warm(&request.model).await {
            warn!(
                provider = %self.provider,
                model = %request.model,
                error = %e,
                "LLM warmup failed before request"
            );
            return Err(e);
        }

        let model = request.model.clone();
        match self.inner.stream(request.clone()).await {
            Ok(stream) => Ok(stream),
            Err(e) if looks_like_cold_restart(&e) => {
                info!(
                    provider = %self.provider,
                    model = %model,
                    error = %e,
                    "request looked like cold restart — invalidating warmup and retrying once"
                );
                self.invalidate().await;
                self.ensure_warm(&model).await?;
                self.inner.stream(request).await
            }
            Err(e) => Err(e),
        }
    }

    async fn warmup(&self, model: &str) -> Result<(), LlmError> {
        // Explicit warmup always probes, regardless of cache state, but does
        // update the cache on success so the next `stream` skips it.
        debug!(provider = %self.provider, model = %model, "explicit warmup");
        self.inner.warmup(model).await?;
        let mut guard = self.last_warmup.lock().await;
        *guard = Some(Instant::now());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockLlmClient, MockResponse};
    use crate::types::{ChatChunk, ChatContent, ChatMessage, Usage};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn ok_response() -> MockResponse {
        MockResponse::raw(vec![ChatChunk::Done {
            usage: Some(Usage {
                input_tokens: 1,
                output_tokens: 1,
            }),
        }])
    }

    fn user_request(model: &str) -> ChatRequest {
        ChatRequest {
            model: model.to_string(),
            system_prompt: None,
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text("hi".to_string()),
                tool_calls: Vec::new(),
                tool_call_id: None,
            }],
            tools: Vec::new(),
            max_tokens: Some(1),
        }
    }

    /// Counts how many times `stream` was invoked on the inner client.
    /// (`MockLlmClient::new(responses)` returns each response in order, so
    /// counting how many we consumed equals how many calls happened.)
    struct CountingClient {
        inner: MockLlmClient,
        calls: AtomicUsize,
    }

    impl CountingClient {
        fn new(responses: Vec<MockResponse>) -> Self {
            Self {
                inner: MockLlmClient::new(responses),
                calls: AtomicUsize::new(0),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LlmClient for CountingClient {
        async fn stream(
            &self,
            request: ChatRequest,
        ) -> Result<
            Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>,
            LlmError,
        > {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.inner.stream(request).await
        }
    }

    #[tokio::test]
    async fn warmup_probes_inner_and_caches() {
        let inner = Arc::new(CountingClient::new(vec![ok_response()]));
        let counter = inner.clone();
        let client = WarmingClient::new(inner, "test");

        client.warmup("model-a").await.unwrap();
        assert_eq!(counter.calls(), 1, "warmup should probe once");
        assert!(client.last_warmup_for_test().await.is_some());
    }

    #[tokio::test]
    async fn stream_skips_warmup_when_cache_fresh() {
        // Two responses: first goes to explicit warmup, second to stream.
        // If the cache wasn't honored we'd consume three.
        let inner = Arc::new(CountingClient::new(vec![ok_response(), ok_response()]));
        let counter = inner.clone();
        let client = WarmingClient::new(inner, "test");

        client.warmup("model-a").await.unwrap();
        let _stream = client.stream(user_request("model-a")).await.unwrap();
        assert_eq!(counter.calls(), 2, "warmup + stream, no extra warmup");
    }

    #[tokio::test]
    async fn stream_warms_lazily_when_cache_empty() {
        // First call to stream should trigger an internal warmup, then the real
        // request — i.e. two underlying calls.
        let inner = Arc::new(CountingClient::new(vec![ok_response(), ok_response()]));
        let counter = inner.clone();
        let client = WarmingClient::new(inner, "test");

        let _stream = client.stream(user_request("model-a")).await.unwrap();
        assert_eq!(counter.calls(), 2, "lazy warmup + real request");
        assert!(client.last_warmup_for_test().await.is_some());
    }

    #[tokio::test]
    async fn stream_re_warms_after_ttl_expiry() {
        // 4 responses: explicit warmup, then post-TTL warmup, then stream.
        // If TTL wasn't honored the second stream wouldn't trigger warmup
        // and we'd only consume 2 responses.
        let inner = Arc::new(CountingClient::new(vec![
            ok_response(),
            ok_response(),
            ok_response(),
            ok_response(),
        ]));
        let counter = inner.clone();
        let client = WarmingClient::with_ttl(inner, "test", Duration::from_millis(50));

        client.warmup("model-a").await.unwrap();
        assert_eq!(counter.calls(), 1);

        // Sleep past TTL so cache is stale.
        tokio::time::sleep(Duration::from_millis(80)).await;

        let _stream = client.stream(user_request("model-a")).await.unwrap();
        assert_eq!(
            counter.calls(),
            3,
            "post-TTL stream should re-warm before the real call"
        );
    }

    #[tokio::test]
    async fn stream_retries_once_on_cold_restart_signature() {
        // Sequence: explicit warmup OK → stream gets 503 → invalidate → re-warm OK → retry stream OK.
        let inner = Arc::new(CountingClient::new(vec![
            ok_response(),                                                  // explicit warmup
            MockResponse::error(LlmError::ServerError("HTTP 503: model loading".into())), // first stream attempt
            ok_response(),                                                  // re-warmup after invalidate
            ok_response(),                                                  // retry stream
        ]));
        let counter = inner.clone();
        let client = WarmingClient::new(inner, "test");

        client.warmup("model-a").await.unwrap();
        let result = client.stream(user_request("model-a")).await;
        assert!(result.is_ok(), "expected retry to succeed");
        assert_eq!(counter.calls(), 4, "warmup + bad stream + re-warm + retry stream");
    }

    #[tokio::test]
    async fn stream_does_not_retry_on_non_cold_restart_errors() {
        // 401 should propagate immediately — no point re-warming an auth failure.
        let inner = Arc::new(CountingClient::new(vec![
            ok_response(),
            MockResponse::error(LlmError::Auth("HTTP 401: bad key".into())),
        ]));
        let counter = inner.clone();
        let client = WarmingClient::new(inner, "test");

        client.warmup("model-a").await.unwrap();
        let result = client.stream(user_request("model-a")).await;
        assert!(matches!(result, Err(LlmError::Auth(_))));
        assert_eq!(counter.calls(), 2, "warmup + single failed stream, no retry");
    }

    #[tokio::test]
    async fn warmup_failure_does_not_update_cache() {
        let inner = Arc::new(CountingClient::new(vec![MockResponse::error(
            LlmError::Auth("HTTP 403: subscription required".into()),
        )]));
        let client = WarmingClient::new(inner, "test");

        let result = client.warmup("model-a").await;
        assert!(matches!(result, Err(LlmError::Auth(_))));
        assert!(
            client.last_warmup_for_test().await.is_none(),
            "failed warmup must not poison the cache as fresh"
        );
    }

    #[test]
    fn cold_restart_classifier() {
        assert!(looks_like_cold_restart(&LlmError::ServerError(
            "HTTP 503: loading".into()
        )));
        assert!(!looks_like_cold_restart(&LlmError::ServerError(
            "HTTP 500: internal".into()
        )));
        assert!(!looks_like_cold_restart(&LlmError::Auth(
            "HTTP 401".into()
        )));
        assert!(!looks_like_cold_restart(&LlmError::RateLimited(
            "HTTP 429".into()
        )));
    }
}

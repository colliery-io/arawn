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

/// Default TTL for providers that unload idle models — chosen for Ollama
/// Cloud, which evicts aggressively. This is the re-warm cadence: after a
/// successful warmup we trust it for this long before probing again.
pub const DEFAULT_WARMUP_TTL: Duration = Duration::from_secs(4 * 60);

/// TTL used for hosted providers that never go cold (Groq, OpenAI,
/// Anthropic, …). Effectively "warm once, never re-warm": a year is far
/// longer than any session, so after the first lazy warmup these providers
/// stop paying for needless re-probes. Not literally infinite so the
/// `elapsed() < ttl` comparison stays cheap and overflow-free.
pub const NEVER_COLD_WARMUP_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 365);

/// Pick a warmup TTL for a provider. Providers known to unload idle models
/// get the short [`DEFAULT_WARMUP_TTL`]; hosted providers that stay warm get
/// [`NEVER_COLD_WARMUP_TTL`]. Unknown providers default to the conservative
/// short TTL — re-warming an always-warm provider wastes a probe, but
/// *failing* to re-warm a cold one breaks the next request, so we err toward
/// re-warming. Matching is case-insensitive and substring-based so
/// `ollama-cloud`, `ollama-local`, etc. all resolve to the cold path.
pub fn warmup_ttl_for_provider(provider: &str) -> Duration {
    let p = provider.to_lowercase();
    // Hosted, always-warm providers — re-warming buys nothing.
    const NEVER_COLD: &[&str] = &["groq", "openai", "anthropic", "together", "fireworks"];
    if NEVER_COLD.iter().any(|n| p.contains(n)) {
        NEVER_COLD_WARMUP_TTL
    } else {
        // ollama (cloud or local) and anything unrecognised → short TTL.
        DEFAULT_WARMUP_TTL
    }
}

/// Providers that can serve a request straight from a cold/unloaded state,
/// where a connection-refused or "model loading" signal means "try warming
/// and retry" rather than "permanently broken". Hosted providers never go
/// cold, so we don't widen cold-start detection for them.
fn provider_can_go_cold(provider: &str) -> bool {
    let p = provider.to_lowercase();
    p.contains("ollama") || p.contains("local") || p.contains("llamacpp") || p.contains("lmstudio")
}

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
        let provider = provider.into();
        let ttl = warmup_ttl_for_provider(&provider);
        Self::with_ttl(inner, provider, ttl)
    }

    pub fn with_ttl(inner: Arc<dyn LlmClient>, provider: impl Into<String>, ttl: Duration) -> Self {
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

/// Errors that look like the provider unloaded the model (or hasn't loaded
/// it yet) and the next request needs a fresh warmup.
///
/// Universal signals (any provider): HTTP 503, and messages that explicitly
/// say the model is loading/unloaded. Cold-capable providers (Ollama, local
/// servers) additionally treat connection-refused as cold-start — the local
/// daemon may be mid-restart and a warmup probe will spin it back up. Hosted
/// providers don't get the connection-refused widening: a refused connection
/// there is a real outage, not a cold model, and re-warming just wastes a
/// round-trip before surfacing the error.
fn looks_like_cold_restart(provider: &str, err: &LlmError) -> bool {
    match err {
        LlmError::ServerError(msg) => msg.contains("HTTP 503") || mentions_model_loading(msg),
        LlmError::Api(msg) => mentions_model_loading(msg),
        LlmError::Request(e) if provider_can_go_cold(provider) => {
            // Local daemon down/restarting — a connect error is recoverable
            // by warming. Timeouts are handled by the retry layer, not here.
            e.is_connect()
        }
        _ => false,
    }
}

/// Whether an error message names a model-loading / not-loaded condition.
/// Case-insensitive; covers Ollama's "model is loading", vLLM/TGI "loading",
/// and "model not loaded" style messages.
fn mentions_model_loading(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("loading") || m.contains("not loaded") || m.contains("warming up")
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
            Err(e) if looks_like_cold_restart(&self.provider, &e) => {
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
            finish_reason: None,
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
        ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError>
        {
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
            ok_response(), // explicit warmup
            MockResponse::error(LlmError::ServerError("HTTP 503: model loading".into())), // first stream attempt
            ok_response(), // re-warmup after invalidate
            ok_response(), // retry stream
        ]));
        let counter = inner.clone();
        let client = WarmingClient::new(inner, "test");

        client.warmup("model-a").await.unwrap();
        let result = client.stream(user_request("model-a")).await;
        assert!(result.is_ok(), "expected retry to succeed");
        assert_eq!(
            counter.calls(),
            4,
            "warmup + bad stream + re-warm + retry stream"
        );
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
        assert_eq!(
            counter.calls(),
            2,
            "warmup + single failed stream, no retry"
        );
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
        // 503 is cold-start for any provider.
        assert!(looks_like_cold_restart(
            "groq",
            &LlmError::ServerError("HTTP 503: loading".into())
        ));
        // A plain 500 / auth / rate-limit is not cold-start.
        assert!(!looks_like_cold_restart(
            "groq",
            &LlmError::ServerError("HTTP 500: internal".into())
        ));
        assert!(!looks_like_cold_restart(
            "groq",
            &LlmError::Auth("HTTP 401".into())
        ));
        assert!(!looks_like_cold_restart(
            "groq",
            &LlmError::RateLimited {
                message: "HTTP 429".into(),
                retry_after: None,
            }
        ));
    }

    #[test]
    fn cold_restart_recognizes_model_loading_messages() {
        // "model loading" style messages count as cold-start for any provider.
        assert!(looks_like_cold_restart(
            "ollama",
            &LlmError::ServerError("HTTP 500: model is loading".into())
        ));
        assert!(looks_like_cold_restart(
            "groq",
            &LlmError::Api("the model is warming up, retry shortly".into())
        ));
        assert!(looks_like_cold_restart(
            "ollama",
            &LlmError::ServerError("model not loaded yet".into())
        ));
    }

    #[test]
    fn cold_restart_connection_refused_only_for_cold_providers() {
        // Build a reqwest connect error by hitting a closed local port.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let connect_err = rt.block_on(async {
            reqwest::Client::new()
                .get("http://127.0.0.1:1/") // port 1 — refused
                .timeout(Duration::from_secs(2))
                .send()
                .await
                .expect_err("connection should fail")
        });
        assert!(connect_err.is_connect(), "expected a connect-class error");

        // A cold-capable provider treats connect-refused as cold-start…
        assert!(looks_like_cold_restart(
            "ollama",
            &LlmError::Request(connect_err)
        ));

        // …but a hosted provider does not (need a fresh error — reqwest::Error
        // isn't Clone). Re-derive it.
        let connect_err2 = rt.block_on(async {
            reqwest::Client::new()
                .get("http://127.0.0.1:1/")
                .timeout(Duration::from_secs(2))
                .send()
                .await
                .expect_err("connection should fail")
        });
        assert!(!looks_like_cold_restart(
            "groq",
            &LlmError::Request(connect_err2)
        ));
    }

    #[test]
    fn ttl_is_short_for_cold_providers_and_long_for_hosted() {
        // Cold-capable providers re-warm on the short cadence.
        assert_eq!(warmup_ttl_for_provider("ollama"), DEFAULT_WARMUP_TTL);
        assert_eq!(warmup_ttl_for_provider("ollama-cloud"), DEFAULT_WARMUP_TTL);
        // Hosted providers effectively never re-warm.
        assert_eq!(warmup_ttl_for_provider("groq"), NEVER_COLD_WARMUP_TTL);
        assert_eq!(warmup_ttl_for_provider("openai"), NEVER_COLD_WARMUP_TTL);
        assert_eq!(warmup_ttl_for_provider("anthropic"), NEVER_COLD_WARMUP_TTL);
        // Unknown providers err toward the conservative short TTL.
        assert_eq!(warmup_ttl_for_provider("mystery"), DEFAULT_WARMUP_TTL);
    }

    #[tokio::test]
    async fn hosted_provider_does_not_rewarm_within_session() {
        // With a hosted provider TTL, an explicit warmup followed by a stream
        // must NOT trigger a second warmup — proving the long TTL is applied
        // through `new()` (not the test-only `with_ttl`).
        let inner = Arc::new(CountingClient::new(vec![ok_response(), ok_response()]));
        let counter = inner.clone();
        let client = WarmingClient::new(inner, "groq");
        client.warmup("model-a").await.unwrap();
        let _stream = client.stream(user_request("model-a")).await.unwrap();
        assert_eq!(
            counter.calls(),
            2,
            "hosted provider should warm once, then never re-warm"
        );
    }
}

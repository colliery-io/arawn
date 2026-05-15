//! `LlmClient` decorator that picks Local vs Remote per request and
//! transparently falls back on failure.
//!
//! Constructed with `(local_client, local_model, remote_client,
//! remote_model, health, hints)`. On every `stream()` call:
//!
//! 1. Look up the model hint via `crate::hints::classify`. Non-hint
//!    model strings are routed Remote unconditionally (they were
//!    written for a specific provider; we don't second-guess).
//! 2. Run [`super::policy::decide`] with the cached health snapshot
//!    and the per-provider [`super::policy::RoutingHints`].
//! 3. Dispatch to the chosen client with that target's concrete
//!    model. On failure, retry on `fallback` if present. Emit a
//!    `RoutingRecord` either way.

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;

use crate::client::LlmClient;
use crate::error::LlmError;
use crate::hints::{classify, is_hint_shape};
use crate::types::{ChatChunk, ChatRequest};

use super::health::SharedHealth;
use super::policy::{Decision, RoutingHints, RoutingTarget, decide};
use super::telemetry::{RoutingOutcome, RoutingRecord};

/// Bundle of one provider's client + concrete model name.
pub struct ProviderHandle {
    pub client: Arc<dyn LlmClient>,
    pub model: String,
}

impl Clone for ProviderHandle {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            model: self.model.clone(),
        }
    }
}

/// Routes each LLM call to Local or Remote per [`RoutingHints`] and
/// the configured policy. Provides transparent fallback when the
/// primary target fails and emits a [`RoutingRecord`] per decision.
pub struct IntelligentRoutingProvider {
    local: Option<ProviderHandle>,
    remote: ProviderHandle,
    health: SharedHealth,
    hints: RoutingHints,
}

impl IntelligentRoutingProvider {
    /// Build a routing provider. `local` is optional — when `None`,
    /// the provider always goes Remote (and the policy's
    /// `LocalHealth::NotConfigured` short-circuit kicks in).
    pub fn new(
        local: Option<ProviderHandle>,
        remote: ProviderHandle,
        health: SharedHealth,
        hints: RoutingHints,
    ) -> Self {
        Self {
            local,
            remote,
            health,
            hints,
        }
    }

    /// Return a copy of this provider with updated hints. Cheap — no
    /// new allocations beyond the small struct itself.
    pub fn with_hints(&self, hints: RoutingHints) -> Self {
        Self {
            local: self.local.clone(),
            remote: self.remote.clone(),
            health: Arc::clone(&self.health),
            hints,
        }
    }

    fn handle(&self, target: &RoutingTarget) -> Option<&ProviderHandle> {
        match target {
            RoutingTarget::Local => self.local.as_ref(),
            RoutingTarget::Remote => Some(&self.remote),
        }
    }
}

#[async_trait]
impl LlmClient for IntelligentRoutingProvider {
    async fn stream(
        &self,
        mut request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError> {
        let hint = match classify(&request.model) {
            Some(h) => h,
            None => {
                // Non-hint model string. The caller picked a concrete
                // model on purpose; respect that and go Remote
                // straight away with no routing layer.
                if is_hint_shape(&request.model) {
                    tracing::warn!(
                        hint = %request.model,
                        "unknown hint string; routing as Remote/Heavy"
                    );
                }
                return self.remote.client.stream(request).await;
            }
        };

        let health = self.health.snapshot();
        let decision = decide(hint, &self.hints, health);

        // Look up the primary handle; if it's Local-not-configured
        // the decision should have already chosen Remote. Defensive
        // fallback in case the policy and the available handles
        // disagree (shouldn't happen, but a routing layer should not
        // panic on misconfig).
        let primary = match self.handle(&decision.primary) {
            Some(h) => h.clone(),
            None => {
                tracing::warn!(
                    primary = ?decision.primary,
                    "routing primary unavailable; using Remote"
                );
                self.remote.clone()
            }
        };

        request.model = primary.model.clone();
        let model_for_record = primary.model.clone();

        match primary.client.stream(request.clone()).await {
            Ok(s) => {
                RoutingRecord::from_decision(
                    &decision,
                    hint,
                    &self.hints,
                    model_for_record,
                    RoutingOutcome::Success,
                    false,
                )
                .emit();
                Ok(s)
            }
            Err(e) => self.try_fallback(decision, hint, request, e).await,
        }
    }
}

impl IntelligentRoutingProvider {
    async fn try_fallback(
        &self,
        decision: Decision,
        hint: crate::hints::ModelHint,
        mut request: ChatRequest,
        primary_err: LlmError,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError> {
        let Some(target) = decision.fallback.clone() else {
            RoutingRecord::from_decision(
                &decision,
                hint,
                &self.hints,
                request.model.clone(),
                RoutingOutcome::Failed,
                false,
            )
            .emit();
            return Err(primary_err);
        };
        let Some(handle) = self.handle(&target).cloned() else {
            RoutingRecord::from_decision(
                &decision,
                hint,
                &self.hints,
                request.model.clone(),
                RoutingOutcome::Failed,
                false,
            )
            .emit();
            return Err(primary_err);
        };
        tracing::warn!(
            primary = ?decision.primary,
            fallback = ?target,
            error = %primary_err,
            "routing primary failed; falling back"
        );
        request.model = handle.model.clone();
        let model_for_record = handle.model.clone();
        match handle.client.stream(request).await {
            Ok(s) => {
                RoutingRecord::from_decision(
                    &decision,
                    hint,
                    &self.hints,
                    model_for_record,
                    RoutingOutcome::FellBack,
                    true,
                )
                .emit();
                Ok(s)
            }
            Err(e) => {
                RoutingRecord::from_decision(
                    &decision,
                    hint,
                    &self.hints,
                    model_for_record,
                    RoutingOutcome::Failed,
                    true,
                )
                .emit();
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hints::ModelHint;
    use crate::mock::{MockLlmClient, MockResponse};
    use crate::routing::health::LocalHealthChecker;
    use crate::routing::policy::{LatencyBudget, RoutingHints, UsagePressure};
    use futures::StreamExt;

    fn handle(label: &str, ok: bool) -> ProviderHandle {
        let mock = if ok {
            MockLlmClient::new(vec![MockResponse::text(format!("from {label}"))])
        } else {
            MockLlmClient::new(vec![MockResponse::error(LlmError::Api(format!("{label} failed")))])
        };
        ProviderHandle {
            client: Arc::new(mock),
            model: format!("{label}-model"),
        }
    }

    fn req(hint: ModelHint) -> ChatRequest {
        ChatRequest {
            model: hint.as_hint(),
            system_prompt: None,
            messages: vec![],
            tools: vec![],
            max_tokens: None,
        }
    }

    async fn drain(
        s: Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>,
    ) -> Vec<ChatChunk> {
        s.collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|c| c.unwrap())
            .collect()
    }

    #[tokio::test]
    async fn lightweight_healthy_routes_local() {
        let health = Arc::new(LocalHealthChecker::configured());
        let p = IntelligentRoutingProvider::new(
            Some(handle("local", true)),
            handle("remote", true),
            health,
            RoutingHints::default(),
        );
        let s = p.stream(req(ModelHint::Lightweight)).await.unwrap();
        let chunks = drain(s).await;
        let text = chunks
            .iter()
            .find_map(|c| match c {
                ChatChunk::TextDelta { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(text, "from local");
    }

    #[tokio::test]
    async fn heavy_always_routes_remote() {
        let health = Arc::new(LocalHealthChecker::configured());
        let p = IntelligentRoutingProvider::new(
            Some(handle("local", true)),
            handle("remote", true),
            health,
            RoutingHints::default(),
        );
        let s = p.stream(req(ModelHint::Heavy)).await.unwrap();
        let chunks = drain(s).await;
        let text = chunks
            .iter()
            .find_map(|c| match c {
                ChatChunk::TextDelta { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(text, "from remote");
    }

    #[tokio::test]
    async fn lightweight_falls_back_to_remote_on_local_failure() {
        let health = Arc::new(LocalHealthChecker::configured());
        let p = IntelligentRoutingProvider::new(
            Some(handle("local", false)),
            handle("remote", true),
            health,
            RoutingHints::default(),
        );
        let s = p.stream(req(ModelHint::Lightweight)).await.unwrap();
        let chunks = drain(s).await;
        let text = chunks
            .iter()
            .find_map(|c| match c {
                ChatChunk::TextDelta { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(text, "from remote");
    }

    #[tokio::test]
    async fn privacy_required_blocks_fallback() {
        // Local fails AND privacy is required → call must fail, not
        // fall back.
        let health = Arc::new(LocalHealthChecker::configured());
        let mut hints = RoutingHints::default();
        hints.privacy_required = true;
        let p = IntelligentRoutingProvider::new(
            Some(handle("local", false)),
            handle("remote", true),
            health,
            hints,
        );
        let res = p.stream(req(ModelHint::Heavy)).await;
        assert!(res.is_err(), "privacy_required must not fall back");
    }

    #[tokio::test]
    async fn unhealthy_lightweight_goes_remote() {
        let health = Arc::new(LocalHealthChecker::configured());
        health.mark_unhealthy();
        let p = IntelligentRoutingProvider::new(
            Some(handle("local", true)),
            handle("remote", true),
            health,
            RoutingHints::default(),
        );
        let s = p.stream(req(ModelHint::Lightweight)).await.unwrap();
        let chunks = drain(s).await;
        let text = chunks
            .iter()
            .find_map(|c| match c {
                ChatChunk::TextDelta { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(text, "from remote");
    }

    #[tokio::test]
    async fn medium_with_low_latency_routes_local() {
        let health = Arc::new(LocalHealthChecker::configured());
        let mut hints = RoutingHints::default();
        hints.latency_budget = LatencyBudget::Low;
        let p = IntelligentRoutingProvider::new(
            Some(handle("local", true)),
            handle("remote", true),
            health,
            hints,
        );
        let s = p.stream(req(ModelHint::Medium)).await.unwrap();
        let chunks = drain(s).await;
        let text = chunks
            .iter()
            .find_map(|c| match c {
                ChatChunk::TextDelta { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(text, "from local");
    }

    #[tokio::test]
    async fn medium_with_high_usage_routes_local() {
        let health = Arc::new(LocalHealthChecker::configured());
        let mut hints = RoutingHints::default();
        hints.usage_pressure = UsagePressure::High;
        let p = IntelligentRoutingProvider::new(
            Some(handle("local", true)),
            handle("remote", true),
            health,
            hints,
        );
        let s = p.stream(req(ModelHint::Medium)).await.unwrap();
        let chunks = drain(s).await;
        let text = chunks
            .iter()
            .find_map(|c| match c {
                ChatChunk::TextDelta { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(text, "from local");
    }

    #[tokio::test]
    async fn medium_default_remote_falls_back_to_local() {
        // Default medium → Remote primary, Local fallback. Make
        // Remote fail.
        let health = Arc::new(LocalHealthChecker::configured());
        let p = IntelligentRoutingProvider::new(
            Some(handle("local", true)),
            handle("remote", false),
            health,
            RoutingHints::default(),
        );
        let s = p.stream(req(ModelHint::Medium)).await.unwrap();
        let chunks = drain(s).await;
        let text = chunks
            .iter()
            .find_map(|c| match c {
                ChatChunk::TextDelta { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(text, "from local");
    }

    #[tokio::test]
    async fn concrete_model_string_passes_through_to_remote() {
        let health = Arc::new(LocalHealthChecker::configured());
        let p = IntelligentRoutingProvider::new(
            Some(handle("local", true)),
            handle("remote", true),
            health,
            RoutingHints::default(),
        );
        let mut request = req(ModelHint::Lightweight);
        request.model = "some-specific-fine-tune".into();
        let s = p.stream(request).await.unwrap();
        let chunks = drain(s).await;
        let text = chunks
            .iter()
            .find_map(|c| match c {
                ChatChunk::TextDelta { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(text, "from remote");
    }
}

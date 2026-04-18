//! Per-tool / per-agent LLM selection.
//!
//! Tools and agents declare what kind of LLM they want via [`LlmPreference`].
//! The runtime resolves preferences against an `LlmClientPool` (defined in the
//! `arawn` binary crate) following: named → provider+model → capability →
//! fallback to engine.
//!
//! These types live here (not in the binary crate) so tools can depend on
//! them without pulling in `arawn-bin`.

use std::sync::Arc;

use arawn_llm::LlmClient;

/// What a tool or agent wants from an LLM.
///
/// Every field is optional. An empty preference (`LlmPreference::any()`) only
/// matches the engine fallback. The most specific field wins:
/// `named` > `provider` + `model` > `capabilities`.
#[derive(Debug, Clone, Default)]
pub struct LlmPreference {
    /// Specific named entry from `arawn.toml` (e.g., "cheap", "judge").
    pub named: Option<String>,
    /// Preferred provider (e.g., "anthropic", "groq").
    pub provider: Option<String>,
    /// Preferred model (e.g., "claude-sonnet-4-20250514").
    pub model: Option<String>,
    /// Minimum capability requirements.
    pub capabilities: LlmCapabilities,
}

impl LlmPreference {
    /// A preference that matches anything — resolves to the engine LLM.
    pub fn any() -> Self {
        Self::default()
    }

    /// Request a specific named pool entry.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            named: Some(name.into()),
            ..Self::default()
        }
    }

    /// Request a specific provider+model pair.
    pub fn provider_model(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: Some(provider.into()),
            model: Some(model.into()),
            ..Self::default()
        }
    }
}

/// Minimum capability requirements an LLM must satisfy.
#[derive(Debug, Clone, Default)]
pub struct LlmCapabilities {
    /// Minimum context window (tokens). `None` = no minimum.
    pub min_context_window: Option<u32>,
    /// Requires tool/function calling support.
    pub tool_use: bool,
    /// Requires vision/image input.
    pub vision: bool,
}

impl LlmCapabilities {
    /// Returns true if `info` meets every requirement.
    pub fn satisfied_by(&self, info: &ResolvedLlmInfo) -> bool {
        if let Some(min) = self.min_context_window
            && info.context_window < min
        {
            return false;
        }
        if self.tool_use && !info.tool_use {
            return false;
        }
        if self.vision && !info.vision {
            return false;
        }
        true
    }

    /// True if no capability constraints are set.
    pub fn is_empty(&self) -> bool {
        self.min_context_window.is_none() && !self.tool_use && !self.vision
    }
}

/// Static capability metadata for a resolved LLM. Mirrors the relevant fields
/// of `arawn-bin`'s `LlmConfig` without forcing a dependency on the binary
/// crate.
#[derive(Debug, Clone)]
pub struct ResolvedLlmInfo {
    pub provider: String,
    pub model: String,
    pub context_window: u32,
    pub tool_use: bool,
    pub vision: bool,
}

/// The result of resolving an [`LlmPreference`] against a pool.
pub struct LlmResolution {
    pub client: Arc<dyn LlmClient>,
    pub info: ResolvedLlmInfo,
    pub match_quality: MatchQuality,
}

impl std::fmt::Debug for LlmResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmResolution")
            .field("info", &self.info)
            .field("match_quality", &self.match_quality)
            .finish()
    }
}

/// Type-erased resolver function. Constructed by the binary crate from an
/// `LlmClientPool` and attached to a context; the engine calls it via
/// [`crate::ToolContext::resolve_llm`]. Kept as a closure (not a trait
/// object of a one-impl trait) so no new abstraction layer is introduced
/// just to wire the pool through.
pub type LlmResolverFn =
    dyn Fn(&LlmPreference) -> LlmResolution + Send + Sync;

/// How closely the resolved client matched the requested preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchQuality {
    /// Got exactly what was requested (named match, or provider+model match).
    Exact,
    /// Different model from what was requested but meets capability bounds.
    Capability,
    /// No preference satisfied — engine default returned.
    Fallback,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(provider: &str, model: &str, ctx: u32, tools: bool, vision: bool) -> ResolvedLlmInfo {
        ResolvedLlmInfo {
            provider: provider.into(),
            model: model.into(),
            context_window: ctx,
            tool_use: tools,
            vision,
        }
    }

    #[test]
    fn capabilities_default_is_satisfied_by_anything() {
        let cap = LlmCapabilities::default();
        assert!(cap.satisfied_by(&info("groq", "x", 1024, false, false)));
    }

    #[test]
    fn capabilities_min_context_window_blocks_small_models() {
        let cap = LlmCapabilities {
            min_context_window: Some(100_000),
            ..Default::default()
        };
        assert!(!cap.satisfied_by(&info("groq", "x", 32_000, true, false)));
        assert!(cap.satisfied_by(&info("groq", "x", 200_000, true, false)));
    }

    #[test]
    fn capabilities_tool_use_required() {
        let cap = LlmCapabilities {
            tool_use: true,
            ..Default::default()
        };
        assert!(!cap.satisfied_by(&info("x", "y", 128_000, false, false)));
        assert!(cap.satisfied_by(&info("x", "y", 128_000, true, false)));
    }

    #[test]
    fn capabilities_vision_required() {
        let cap = LlmCapabilities {
            vision: true,
            ..Default::default()
        };
        assert!(!cap.satisfied_by(&info("x", "y", 128_000, true, false)));
        assert!(cap.satisfied_by(&info("x", "y", 128_000, true, true)));
    }

    #[test]
    fn preference_constructors() {
        let p = LlmPreference::named("cheap");
        assert_eq!(p.named.as_deref(), Some("cheap"));

        let p = LlmPreference::provider_model("anthropic", "claude-sonnet");
        assert_eq!(p.provider.as_deref(), Some("anthropic"));
        assert_eq!(p.model.as_deref(), Some("claude-sonnet"));
    }
}

//! Per-tool / per-agent LLM selection.
//!
//! Tools that take a runtime LLM choice (today: the `agent` tool's `llm`
//! sub-arg for pinning a sub-agent to a specific named pool entry) build
//! an [`LlmPreference`] and resolve it through [`crate::ToolContext::resolve_llm`].
//! The runtime resolves against an `LlmClientPool` in the `arawn` binary
//! crate by named lookup, with a fallback to the engine model.
//!
//! These types live here (not in the binary crate) so tools can depend on
//! them without pulling in `arawn-bin`.

use std::sync::Arc;

use arawn_llm::LlmClient;

/// What a tool or agent wants from an LLM. Today: a named `[llm.NAME]`
/// pool entry. The capability-matching subsystem (provider+model and
/// capability filters) was removed in the YAGNI pass — production never
/// constructed anything beyond `named`.
#[derive(Debug, Clone, Default)]
pub struct LlmPreference {
    /// Specific named entry from `arawn.toml` (e.g., "cheap", "judge").
    pub named: Option<String>,
}

impl LlmPreference {
    /// Request a specific named pool entry.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            named: Some(name.into()),
        }
    }
}

/// Static metadata for a resolved LLM. Mirrors the relevant fields of
/// `arawn-bin`'s `LlmConfig` without forcing a dependency on the binary
/// crate. Used by the agent tool for logging / display of which LLM
/// resolved.
#[derive(Debug, Clone)]
pub struct ResolvedLlmInfo {
    pub provider: String,
    pub model: String,
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
pub type LlmResolverFn = dyn Fn(&LlmPreference) -> LlmResolution + Send + Sync;

/// How closely the resolved client matched the requested preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchQuality {
    /// Got exactly what was requested (named match).
    Exact,
    /// No named preference satisfied — engine default returned.
    Fallback,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_named_constructor() {
        let p = LlmPreference::named("cheap");
        assert_eq!(p.named.as_deref(), Some("cheap"));
    }
}

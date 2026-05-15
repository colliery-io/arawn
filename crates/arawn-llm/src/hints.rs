//! Hint-style model routing taxonomy.
//!
//! Instead of hard-coding concrete model names at every call site, callers
//! emit a *hint* — a tier description that says how heavy the request is.
//! The pool maps each hint to a concrete named profile (`[llm.NAME]`) via
//! the `[routing.hints]` config section.
//!
//! Today every hint can resolve to the same provider; the value is the
//! call-site decoupling, not the actual routing decision. The real
//! per-request policy (health-aware local/remote, privacy/latency/usage
//! biases) lands in T-0278.
//!
//! # Recognised hints
//!
//! - `hint:lightweight` → [`ModelHint::Lightweight`]. Reactions, short
//!   classifications, anything that does not need reasoning depth.
//! - `hint:medium` → [`ModelHint::Medium`]. Summarisation, focused
//!   extraction, single-shot tool plumbing.
//! - `hint:heavy` → [`ModelHint::Heavy`]. The main agent loop, deep
//!   reasoning, long-context planning.
//!
//! Anything else returns `None`; the caller should treat that as
//! "this is not a hint" and pass the string through as a concrete
//! model name.

/// Tier description for a model call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelHint {
    /// Reactions, classifications, short formatting.
    Lightweight,
    /// Summarisation, focused extraction.
    Medium,
    /// Deep reasoning, the main agent loop.
    Heavy,
}

/// String prefix that marks a hint at the model-string boundary.
pub const HINT_PREFIX: &str = "hint:";

impl ModelHint {
    /// Stable identifier used inside hint strings and telemetry.
    pub fn as_str(self) -> &'static str {
        match self {
            ModelHint::Lightweight => "lightweight",
            ModelHint::Medium => "medium",
            ModelHint::Heavy => "heavy",
        }
    }

    /// The full hint string, including the `hint:` prefix.
    pub fn as_hint(self) -> String {
        format!("{HINT_PREFIX}{}", self.as_str())
    }
}

/// Parse a hint at the model-string boundary.
///
/// Returns `Some(_)` only when `input` starts with `hint:` and the rest
/// matches a recognised tier name. Concrete model names (e.g.
/// `"openai/gpt-oss-120b"`) and unknown `hint:*` strings return `None`,
/// so callers can pass them through as concrete model strings.
///
/// Matching is case-insensitive on the tier name to match the
/// permissive shape callers tend to want.
pub fn classify(input: &str) -> Option<ModelHint> {
    let rest = input.strip_prefix(HINT_PREFIX)?;
    match rest.to_ascii_lowercase().as_str() {
        "lightweight" | "light" => Some(ModelHint::Lightweight),
        "medium" => Some(ModelHint::Medium),
        "heavy" => Some(ModelHint::Heavy),
        _ => None,
    }
}

/// True when `input` looks like a hint, regardless of whether it parses.
/// Useful for callers that want to log unknown-hint fallbacks.
pub fn is_hint_shape(input: &str) -> bool {
    input.starts_with(HINT_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_recognised_hints() {
        assert_eq!(classify("hint:lightweight"), Some(ModelHint::Lightweight));
        assert_eq!(classify("hint:light"), Some(ModelHint::Lightweight));
        assert_eq!(classify("hint:medium"), Some(ModelHint::Medium));
        assert_eq!(classify("hint:heavy"), Some(ModelHint::Heavy));
    }

    #[test]
    fn classify_case_insensitive_tier() {
        assert_eq!(classify("hint:Medium"), Some(ModelHint::Medium));
        assert_eq!(classify("hint:HEAVY"), Some(ModelHint::Heavy));
    }

    #[test]
    fn classify_unknown_hint_is_none() {
        assert_eq!(classify("hint:reaction"), None);
        assert_eq!(classify("hint:"), None);
        assert_eq!(classify("hint:summarize"), None);
    }

    #[test]
    fn classify_concrete_model_is_none() {
        assert_eq!(classify("openai/gpt-oss-120b"), None);
        assert_eq!(classify("llama-3.3-70b-versatile"), None);
        assert_eq!(classify(""), None);
    }

    #[test]
    fn round_trip_as_hint() {
        for h in [ModelHint::Lightweight, ModelHint::Medium, ModelHint::Heavy] {
            assert_eq!(classify(&h.as_hint()), Some(h));
        }
    }

    #[test]
    fn is_hint_shape_independent_of_validity() {
        assert!(is_hint_shape("hint:medium"));
        assert!(is_hint_shape("hint:reaction")); // unknown but still hint-shaped
        assert!(!is_hint_shape("openai/gpt-oss-120b"));
    }
}

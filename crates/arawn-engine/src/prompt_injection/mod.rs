//! Centralised prompt-injection guard.
//!
//! Every inbound-text boundary in the engine — web-fetch results,
//! feed-search hits, future channel-inbound dispatch — funnels its
//! payload through [`enforce`] before letting it flow into LLM
//! context. The guard returns a [`Verdict`] the caller acts on.
//!
//! # Verdicts
//!
//! - [`Verdict::Allow`] — no signal worth acting on, pass the text
//!   through unchanged.
//! - [`Verdict::Sanitize`] — soft signal. The text is either
//!   stripped of control chars / zero-width Unicode, or wrapped in
//!   `<UNTRUSTED-CONTENT>` markers so the model treats it as data
//!   rather than instructions. Carries the list of reasons that
//!   fired.
//! - [`Verdict::Block`] — strong signal (explicit instruction-
//!   override phrasing, role-tag spoofing). Caller should refuse to
//!   forward the content; what that means is caller-specific
//!   (return an error from a tool, drop the item from a feed, etc.).
//!
//! # Hook events
//!
//! Whenever a `Sanitize` or `Block` verdict surfaces, the caller is
//! expected to fire a [`HookEvent::PromptInjectionVerdict`] hook
//! with [`HookInput::PromptInjectionVerdict`] so the user has
//! visibility. The guard itself is hook-free — it returns the
//! verdict and the caller fires.
//!
//! # Design notes
//!
//! - The guard is intentionally cheap (string scans, no LLM). It
//!   runs on every inbound payload and must not be a bottleneck.
//! - The ruleset is conservative: false positives result in
//!   `Sanitize` rather than `Block`, which is the safer asymmetry.
//! - Heuristics live in [`heuristics`]; the decision composition
//!   lives in [`detector`].

pub mod detector;
pub mod heuristics;

#[cfg(test)]
mod coverage_test;

pub use detector::enforce;

/// The outcome of running the prompt-injection guard against
/// inbound text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Pass-through, no findings.
    Allow,
    /// Soft-signal: caller forwards `sanitized` in place of the
    /// original text. `reasons` lists every finding that fired.
    Sanitize {
        sanitized: String,
        reasons: Vec<String>,
    },
    /// Strong signal: caller must not forward the original text.
    /// `reasons` lists every finding that fired.
    Block { reasons: Vec<String> },
}

impl Verdict {
    pub fn is_block(&self) -> bool {
        matches!(self, Verdict::Block { .. })
    }
    pub fn is_sanitize(&self) -> bool {
        matches!(self, Verdict::Sanitize { .. })
    }
    pub fn is_allow(&self) -> bool {
        matches!(self, Verdict::Allow)
    }
}

/// Helper for callers that want to emit a hook event on every
/// non-`Allow` verdict. Returns the same verdict it was given.
///
/// The hook event is fired in a fire-and-forget fashion — failure
/// to dispatch the hook is logged but does not change the verdict.
pub async fn report(
    verdict: &Verdict,
    context: &str,
    hook_runner: Option<&std::sync::Arc<crate::hooks::HookRunner>>,
) {
    let (verdict_label, reasons) = match verdict {
        Verdict::Allow => return,
        Verdict::Sanitize { reasons, .. } => ("sanitize", reasons.clone()),
        Verdict::Block { reasons } => ("block", reasons.clone()),
    };
    if let Some(runner) = hook_runner {
        let input = crate::hooks::HookInput::PromptInjectionVerdict {
            context: context.to_string(),
            verdict: verdict_label.to_string(),
            reasons,
        };
        let _ = runner.run(&input).await;
    } else {
        tracing::warn!(
            context = %context,
            verdict = verdict_label,
            ?reasons,
            "prompt-injection verdict (no hook runner attached)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_helpers() {
        assert!(Verdict::Allow.is_allow());
        assert!(
            Verdict::Sanitize {
                sanitized: "".into(),
                reasons: vec![]
            }
            .is_sanitize()
        );
        assert!(Verdict::Block { reasons: vec![] }.is_block());
    }

    #[test]
    fn allow_is_passthrough() {
        let v = enforce("plain news article body", "web_fetch");
        assert!(v.is_allow());
    }
}

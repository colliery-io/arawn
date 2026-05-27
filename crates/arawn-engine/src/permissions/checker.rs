use arawn_tool::PermissionCategory;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::rules::{PermissionDecision, PermissionRule, RuleMatcher};

/// Permission mode — controls fallback behavior when no explicit rule matches.
///
/// Variant strings (slash command, TOML, audit log, serde JSON):
/// `ask | edits | full | plan`. T-0347 unified the vocabulary across
/// every surface; old strings (`default`, `accept_edits`, `bypass`)
/// are not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionMode {
    /// Read-only tools auto-allowed, write/shell tools trigger Ask.
    #[default]
    Ask,
    /// File tools (read + write) auto-allowed, shell still triggers Ask.
    Edits,
    /// Everything auto-allowed — power user / CI mode.
    Full,
    /// Plan mode — only side-effect-free tools allowed. All tools with side
    /// effects are denied (not asked — denied outright). The agent can observe
    /// the world but cannot take action until the plan is approved.
    Plan,
}

impl PermissionMode {
    /// Determine the fallback decision for a tool when no explicit rule
    /// matched. Takes the tool's declared [`PermissionCategory`] plus the
    /// tool name — the name is only used for the Plan-mode exception that
    /// lets `enter_plan_mode` / `exit_plan_mode` through regardless of
    /// category.
    pub fn fallback(&self, category: PermissionCategory, tool_name: &str) -> PermissionDecision {
        match self {
            PermissionMode::Ask => match category {
                PermissionCategory::ReadOnly => PermissionDecision::Allowed,
                _ => PermissionDecision::Ask,
            },
            PermissionMode::Edits => match category {
                PermissionCategory::ReadOnly | PermissionCategory::FileWrite => {
                    PermissionDecision::Allowed
                }
                PermissionCategory::Shell | PermissionCategory::Other => PermissionDecision::Ask,
            },
            PermissionMode::Full => PermissionDecision::Allowed,
            PermissionMode::Plan => match category {
                PermissionCategory::ReadOnly => PermissionDecision::Allowed,
                _ => {
                    // Plan mode tools (EnterPlanMode/ExitPlanMode) are always allowed
                    if tool_name == "enter_plan_mode" || tool_name == "exit_plan_mode" {
                        PermissionDecision::Allowed
                    } else {
                        PermissionDecision::Denied
                    }
                }
            },
        }
    }
}

/// Response from a user when prompted for permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionResponse {
    AllowOnce,
    AllowAlways,
    Deny,
}

/// A single option displayed in a modal prompt.
#[derive(Debug, Clone)]
pub struct ModalOption {
    pub label: String,
    pub description: Option<String>,
}

impl ModalOption {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// A request to show a modal to the user and get a selection.
#[derive(Debug, Clone)]
pub struct ModalRequest {
    pub title: String,
    pub subtitle: Option<String>,
    pub options: Vec<ModalOption>,
}

/// Generic trait for prompting the user with a modal dialog.
/// Tools build their own ModalRequest and interpret the returned index.
/// Returns None if the user cancels (Escape), Some(index) if they select.
#[async_trait]
pub trait ModalPrompt: Send + Sync {
    async fn prompt(&self, request: ModalRequest) -> Option<usize>;
}

/// In-memory store for session-scoped permission grants.
/// When a user selects "Allow Always", a `(tool_name, ArgShape)`
/// entry is added here and matching calls won't be prompted again
/// for the rest of the session. `ArgShape` lives in
/// [`crate::approval::allowlist`] — see that module for the
/// normalisation rules.
///
/// Grants are keyed by `(tool, shape)` (T-0276) so an approved
/// `file_write` to `~/Desktop/proj/foo.rs` does not auto-approve
/// `file_write` to `/etc/`. A shape of `"<tool>:*"` acts as a
/// wildcard — `is_granted_shape` falls back to the wildcard if no
/// exact shape match is found.
#[derive(Debug, Default)]
pub struct SessionGrants {
    inner: crate::approval::SessionAllowlist,
}

impl SessionGrants {
    pub fn new() -> Self {
        Self::default()
    }

    /// Shape-aware grant.
    pub fn grant_shape(&mut self, tool_name: String, shape: crate::approval::ArgShape) {
        self.inner.grant(tool_name, shape);
    }

    /// Shape-aware check. Falls back to the `"<tool>:*"` wildcard
    /// entry when no exact shape match is found.
    pub fn is_granted_shape(&self, tool_name: &str, shape: &crate::approval::ArgShape) -> bool {
        if self.inner.is_granted(tool_name, shape) {
            return true;
        }
        let wildcard = crate::approval::ArgShape(format!("{tool_name}:*"));
        self.inner.is_granted(tool_name, &wildcard)
    }

    /// Clear all session grants.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

/// Why a permission decision came out the way it did. Surfaced to the
/// engine for error messages ("denied by rule X", "denied by mode default
/// 'plan'") and to the audit log behind `/permissions`.
#[derive(Debug, Clone)]
pub enum DecisionReason {
    /// A specific rule matched. Carries the matched rule so callers can
    /// quote it back to the user.
    MatchedRule(PermissionRule),
    /// A previous session grant let it through.
    SessionGrant,
    /// No rule matched; the active mode's fallback decided.
    ModeFallback { mode: PermissionMode },
    /// User answered an interactive prompt.
    Prompted,
    /// No checker is wired (default-allow, internal callers).
    NoChecker,
}

impl DecisionReason {
    /// One-line human-readable form for error messages and audit display.
    pub fn display(&self) -> String {
        match self {
            DecisionReason::MatchedRule(r) => {
                format!(
                    "rule '{} {}'",
                    match r.kind {
                        crate::permissions::rules::RuleKind::Allow => "allow",
                        crate::permissions::rules::RuleKind::Deny => "deny",
                        crate::permissions::rules::RuleKind::Ask => "ask",
                    },
                    r.display_spec()
                )
            }
            DecisionReason::SessionGrant => "session grant".to_string(),
            DecisionReason::ModeFallback { mode } => {
                format!("mode default '{:?}'", mode).to_lowercase()
            }
            DecisionReason::Prompted => "user prompt".to_string(),
            DecisionReason::NoChecker => "no permission checker configured".to_string(),
        }
    }
}

/// One row of the audit log — what was checked, when, and how it was decided.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: std::time::SystemTime,
    pub tool_name: String,
    pub tool_input_summary: String,
    pub decision: PermissionDecision,
    pub reason: String,
}

/// Cap on the audit ring buffer — newest decisions evict oldest. Sized so a
/// chatty session doesn't crowd out everything older but a long session
/// doesn't grow memory unbounded.
const AUDIT_CAP: usize = 50;

/// Shareable audit buffer — held in an Arc so callers (e.g. the
/// long-lived service) can both pass it into per-message PermissionChecker
/// instances *and* read snapshots from it without going through a checker.
pub type SharedAudit = std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<AuditEntry>>>;

/// Construct a fresh shared audit buffer with the standard cap.
pub fn new_shared_audit() -> SharedAudit {
    std::sync::Arc::new(std::sync::Mutex::new(
        std::collections::VecDeque::with_capacity(AUDIT_CAP),
    ))
}

/// The central permission checker. Evaluates rules against tool calls,
/// prompts the user when needed, and manages session grants.
pub struct PermissionChecker {
    rules: std::sync::RwLock<Vec<PermissionRule>>,
    mode: std::sync::RwLock<PermissionMode>,
    grants: std::sync::Mutex<SessionGrants>,
    prompter: Option<Box<dyn ModalPrompt>>,
    audit: SharedAudit,
    /// Optional hook runner — fires `PermissionRequest` before a modal
    /// prompt is raised and `PermissionDenied` when a tool call is
    /// rejected (by rule or by user denial). Both non-blocking in V1.
    hook_runner: Option<std::sync::Arc<crate::hooks::HookRunner>>,
}

impl PermissionChecker {
    /// Create a new permission checker with the given rules and default mode.
    /// Without a prompter, `Ask` decisions are treated as `Denied`.
    pub fn new(rules: Vec<PermissionRule>) -> Self {
        Self {
            rules: std::sync::RwLock::new(rules),
            mode: std::sync::RwLock::new(PermissionMode::Ask),
            grants: std::sync::Mutex::new(SessionGrants::new()),
            prompter: None,
            audit: new_shared_audit(),
            hook_runner: None,
        }
    }

    /// Wire an externally-owned audit buffer so per-message checkers can
    /// share a single rolling log. The default `new` constructs its own
    /// buffer (fine for tests); production callers pass one in here.
    pub fn with_audit(mut self, audit: SharedAudit) -> Self {
        self.audit = audit;
        self
    }

    /// Wire a hook runner. When set, the checker fires
    /// `PermissionRequest` before a modal prompt and `PermissionDenied`
    /// when a tool call is rejected. Both non-blocking.
    pub fn with_hook_runner(mut self, runner: std::sync::Arc<crate::hooks::HookRunner>) -> Self {
        self.hook_runner = Some(runner);
        self
    }

    fn record_audit(
        &self,
        tool_name: &str,
        tool_input: &str,
        decision: PermissionDecision,
        reason: &DecisionReason,
    ) {
        let mut buf = self.audit.lock().unwrap();
        if buf.len() >= AUDIT_CAP {
            buf.pop_front();
        }
        // Truncate input summary to avoid bloating memory with large tool args.
        let summary: String = tool_input.chars().take(120).collect();
        buf.push_back(AuditEntry {
            timestamp: std::time::SystemTime::now(),
            tool_name: tool_name.to_string(),
            tool_input_summary: summary,
            decision,
            reason: reason.display(),
        });
    }

    /// Set the permission mode (Default, AcceptEdits, BypassPermissions).
    pub fn with_mode(self, mode: PermissionMode) -> Self {
        if mode == PermissionMode::Full {
            info!("permission mode: bypass — all tools auto-allowed");
        }
        *self.mode.write().unwrap() = mode;
        self
    }

    /// Set the modal prompter for interactive permission requests.
    pub fn with_prompter(mut self, prompter: Box<dyn ModalPrompt>) -> Self {
        self.prompter = Some(prompter);
        self
    }

    /// Hot-reload: replace the current rules with new ones.
    pub fn update_rules(&self, rules: Vec<PermissionRule>) {
        info!(count = rules.len(), "hot-reloading permission rules");
        *self.rules.write().unwrap() = rules;
    }

    /// Hot-reload: update the permission mode.
    pub fn update_mode(&self, mode: PermissionMode) {
        info!(mode = ?mode, "hot-reloading permission mode");
        *self.mode.write().unwrap() = mode;
    }

    /// Check if a tool call is permitted. The caller supplies the tool's
    /// declared [`PermissionCategory`] — the engine looks this up via the
    /// [`Tool`] trait (`tool.permission_category()`) before calling.
    ///
    /// Flow:
    /// 1. Evaluate deny rules first (deny always wins, even over session grants)
    /// 2. Check session grants (short-circuit if granted and not denied)
    /// 3. Evaluate remaining rules: allow > ask > no match
    /// 4. If Ask, prompt the user (or deny if no prompter)
    /// 5. NoMatch falls through to permission mode fallback, which uses the
    ///    category to decide (ReadOnly auto-allow in Default, etc.)
    pub async fn check(
        &self,
        tool_name: &str,
        tool_input: &str,
        category: PermissionCategory,
    ) -> PermissionDecision {
        self.check_explained(tool_name, tool_input, category)
            .await
            .0
    }

    /// Same as [`check`] but also returns *why* the decision was made.
    /// Callers that surface denial messages to the user (engine error chain,
    /// audit log) use this; everything else uses `check`.
    pub async fn check_explained(
        &self,
        tool_name: &str,
        tool_input: &str,
        category: PermissionCategory,
    ) -> (PermissionDecision, DecisionReason) {
        // 1. Evaluate rules — deny rules always take priority
        let (rule_decision, matched_rule) = {
            let rules = self.rules.read().unwrap();
            RuleMatcher::evaluate_with_match(&rules, tool_name, tool_input)
        };
        debug!(tool_name, decision = ?rule_decision, "permission rule evaluation");

        // Deny rules override everything, including session grants
        if rule_decision == PermissionDecision::Denied {
            warn!(
                tool_name,
                "permission denied by rule (overrides session grants)"
            );
            let reason = matched_rule.map(DecisionReason::MatchedRule).unwrap_or(
                DecisionReason::ModeFallback {
                    mode: *self.mode.read().unwrap(),
                },
            );
            self.record_audit(tool_name, tool_input, PermissionDecision::Denied, &reason);
            self.fire_permission_denied_hook(tool_name, tool_input, &reason.display())
                .await;
            return (PermissionDecision::Denied, reason);
        }

        // 2. Session grants short-circuit (only checked after deny rules pass).
        // Match either an exact (tool, shape) grant or a pre-T-0276 wildcard.
        let shape = crate::approval::ArgShape::for_tool(tool_name, tool_input);
        if self
            .grants
            .lock()
            .unwrap()
            .is_granted_shape(tool_name, &shape)
        {
            debug!(tool_name, shape = %shape.as_str(), "session grant hit — allowed");
            let reason = DecisionReason::SessionGrant;
            self.record_audit(tool_name, tool_input, PermissionDecision::Allowed, &reason);
            return (PermissionDecision::Allowed, reason);
        }

        // 3. Remaining rule decisions
        let (decision, reason) = match rule_decision {
            PermissionDecision::Allowed => (
                PermissionDecision::Allowed,
                matched_rule.map(DecisionReason::MatchedRule).unwrap_or(
                    DecisionReason::ModeFallback {
                        mode: *self.mode.read().unwrap(),
                    },
                ),
            ),
            PermissionDecision::Denied => unreachable!("handled above"),
            PermissionDecision::Ask => {
                self.fire_permission_request_hook(tool_name, tool_input)
                    .await;
                let prompted = self.prompt_user(tool_name, tool_input).await;
                if prompted == PermissionDecision::Denied {
                    self.fire_permission_denied_hook(tool_name, tool_input, "user prompt")
                        .await;
                }
                (prompted, DecisionReason::Prompted)
            }
            // NoMatch = no explicit rule applies. Fall back to permission mode.
            PermissionDecision::NoMatch => {
                let mode = *self.mode.read().unwrap();
                let fallback = mode.fallback(category, tool_name);
                debug!(tool_name, ?fallback, ?mode, ?category, "mode fallback");
                let reason = DecisionReason::ModeFallback { mode };
                let final_decision = match fallback {
                    PermissionDecision::Ask => {
                        self.fire_permission_request_hook(tool_name, tool_input)
                            .await;
                        let prompted = self.prompt_user(tool_name, tool_input).await;
                        if prompted == PermissionDecision::Denied {
                            self.fire_permission_denied_hook(
                                tool_name,
                                tool_input,
                                &reason.display(),
                            )
                            .await;
                        }
                        // Prompted from a NoMatch path — still prefer reporting the
                        // fallback mode rather than "user prompt", since a future
                        // re-evaluation against a stable rule set should hit the
                        // same fallback. Audit captures the eventual decision.
                        return {
                            self.record_audit(tool_name, tool_input, prompted, &reason);
                            (prompted, reason)
                        };
                    }
                    other => other,
                };
                if final_decision == PermissionDecision::Denied {
                    self.fire_permission_denied_hook(tool_name, tool_input, &reason.display())
                        .await;
                }
                (final_decision, reason)
            }
        };
        self.record_audit(tool_name, tool_input, decision, &reason);
        (decision, reason)
    }

    /// Prompt the user for permission (or deny if no prompter is configured).
    async fn prompt_user(&self, tool_name: &str, tool_input: &str) -> PermissionDecision {
        match &self.prompter {
            Some(prompter) => {
                let request = ModalRequest {
                    title: "Permission Required".to_string(),
                    subtitle: Some(format!(
                        "Tool: {} — {}",
                        tool_name,
                        truncate_input(tool_input, 200)
                    )),
                    options: vec![
                        ModalOption::new("Allow Once")
                            .with_description("Allow this tool call only"),
                        ModalOption::new("Allow Always")
                            .with_description("Allow for the rest of the session"),
                        ModalOption::new("Deny").with_description("Block this tool call"),
                    ],
                };
                let response = prompter.prompt(request).await;
                let shape = crate::approval::ArgShape::for_tool(tool_name, tool_input);
                match response {
                    Some(0) => PermissionDecision::Allowed,
                    Some(1) => {
                        self.grants
                            .lock()
                            .unwrap()
                            .grant_shape(tool_name.to_string(), shape);
                        PermissionDecision::Allowed
                    }
                    _ => PermissionDecision::Denied,
                }
            }
            None => {
                warn!(
                    tool_name,
                    "ask decision but no prompter — denying (fail closed)"
                );
                PermissionDecision::Denied
            }
        }
    }

    /// Get the current permission mode.
    pub fn mode(&self) -> PermissionMode {
        *self.mode.read().unwrap()
    }

    /// Clear all session grants.
    pub fn clear_grants(&self) {
        self.grants.lock().unwrap().clear();
    }

    /// Fire the `PermissionRequest` hook before a user prompt is raised.
    /// Non-blocking; result is ignored.
    async fn fire_permission_request_hook(&self, tool_name: &str, tool_input: &str) {
        if let Some(ref runner) = self.hook_runner {
            let parsed: serde_json::Value =
                serde_json::from_str(tool_input).unwrap_or(serde_json::Value::Null);
            let hook_input = crate::hooks::HookInput::PermissionRequest {
                tool_name: tool_name.to_string(),
                tool_input: parsed,
                rule: "ask".to_string(),
            };
            let _ = runner.run(&hook_input).await;
        }
    }

    /// Fire the `PermissionDenied` hook when a tool call is rejected.
    /// Non-blocking; result is ignored.
    async fn fire_permission_denied_hook(&self, tool_name: &str, tool_input: &str, reason: &str) {
        if let Some(ref runner) = self.hook_runner {
            let parsed: serde_json::Value =
                serde_json::from_str(tool_input).unwrap_or(serde_json::Value::Null);
            let hook_input = crate::hooks::HookInput::PermissionDenied {
                tool_name: tool_name.to_string(),
                tool_input: parsed,
                reason: reason.to_string(),
            };
            let _ = runner.run(&hook_input).await;
        }
    }
}

fn truncate_input(input: &str, max_len: usize) -> String {
    if input.len() <= max_len {
        input.to_string()
    } else {
        // Find the nearest char boundary at or before max_len to avoid panic on multi-byte UTF-8
        let boundary = input.floor_char_boundary(max_len);
        format!("{}…", &input[..boundary])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::rules::RuleKind;

    /// Mock prompter that returns a fixed index (0=AllowOnce, 1=AllowAlways, 2/None=Deny).
    struct MockPrompter {
        index: Option<usize>,
    }

    impl MockPrompter {
        fn allow_once() -> Self {
            Self { index: Some(0) }
        }
        fn allow_always() -> Self {
            Self { index: Some(1) }
        }
        fn deny() -> Self {
            Self { index: Some(2) }
        }
    }

    #[async_trait]
    impl ModalPrompt for MockPrompter {
        async fn prompt(&self, _request: ModalRequest) -> Option<usize> {
            self.index
        }
    }

    #[tokio::test]
    async fn allowed_by_rule() {
        let rules = vec![PermissionRule::new(RuleKind::Allow, "Read")];
        let checker = PermissionChecker::new(rules);
        assert_eq!(
            checker
                .check("Read", "/foo", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn denied_by_rule() {
        let rules = vec![PermissionRule::new(RuleKind::Deny, "Bash")];
        let checker = PermissionChecker::new(rules);
        assert_eq!(
            checker
                .check("Bash", "rm -rf /", PermissionCategory::Shell)
                .await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn ask_without_prompter_denies() {
        let rules = vec![PermissionRule::new(RuleKind::Ask, "Bash")];
        let checker = PermissionChecker::new(rules);
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn ask_with_allow_once() {
        let rules = vec![PermissionRule::new(RuleKind::Ask, "Bash")];
        let checker =
            PermissionChecker::new(rules).with_prompter(Box::new(MockPrompter::allow_once()));
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Allowed
        );
        // Not granted for session — next call should ask again
        assert!(
            !checker
                .grants
                .lock()
                .unwrap()
                .is_granted_shape("Bash", &crate::approval::ArgShape("Bash:*".into()))
        );
    }

    #[tokio::test]
    async fn ask_with_allow_always_grants_session() {
        let rules = vec![PermissionRule::new(RuleKind::Ask, "Bash")];
        let checker =
            PermissionChecker::new(rules).with_prompter(Box::new(MockPrompter::allow_always()));
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Allowed
        );
        // Session grant recorded — subsequent calls skip prompting
        assert!(
            checker
                .grants
                .lock()
                .unwrap()
                .is_granted_shape("Bash", &crate::approval::ArgShape("Bash:*".into()))
        );
        assert_eq!(
            checker
                .check("Bash", "cargo test", PermissionCategory::Shell)
                .await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn ask_with_deny() {
        let rules = vec![PermissionRule::new(RuleKind::Ask, "Edit")];
        let checker = PermissionChecker::new(rules).with_prompter(Box::new(MockPrompter::deny()));
        assert_eq!(
            checker
                .check("Edit", "/foo.rs", PermissionCategory::FileWrite)
                .await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn default_mode_allows_read_only() {
        let checker = PermissionChecker::new(vec![]);
        // Read-only tools auto-allowed in Default mode
        assert_eq!(
            checker
                .check("Read", "/foo", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("Glob", "*.rs", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("Grep", "pattern", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("Think", "hmm", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn default_mode_asks_for_writes() {
        // Without a prompter, Ask → Denied
        let checker = PermissionChecker::new(vec![]);
        assert_eq!(
            checker
                .check("Edit", "/foo.rs", PermissionCategory::FileWrite)
                .await,
            PermissionDecision::Denied
        );
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Denied
        );
        assert_eq!(
            checker
                .check("Write", "/bar.rs", PermissionCategory::FileWrite)
                .await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn accept_edits_mode_allows_file_ops() {
        let checker = PermissionChecker::new(vec![]).with_mode(PermissionMode::Edits);
        // File ops allowed
        assert_eq!(
            checker
                .check("Read", "/foo", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("Edit", "/foo.rs", PermissionCategory::FileWrite)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("Write", "/bar.rs", PermissionCategory::FileWrite)
                .await,
            PermissionDecision::Allowed
        );
        // Shell still asks (denied without prompter)
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn bypass_mode_allows_everything() {
        let checker = PermissionChecker::new(vec![]).with_mode(PermissionMode::Full);
        assert_eq!(
            checker
                .check("Read", "/foo", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("Edit", "/foo.rs", PermissionCategory::FileWrite)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("Bash", "rm -rf /", PermissionCategory::Shell)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("UnknownTool", "", PermissionCategory::Other)
                .await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn explicit_rules_override_mode() {
        // Even in Default mode, an explicit Allow rule for Bash should work
        let rules = vec![PermissionRule::new(RuleKind::Allow, "Bash")];
        let checker = PermissionChecker::new(rules);
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn deny_rules_override_session_grants() {
        let rules = vec![PermissionRule::new(RuleKind::Deny, "Bash")];
        let checker = PermissionChecker::new(rules);
        // Manually grant — deny rule should still win
        checker.grants.lock().unwrap().grant_shape(
            "Bash".to_string(),
            crate::approval::ArgShape("Bash:*".into()),
        );
        assert_eq!(
            checker
                .check("Bash", "rm -rf /", PermissionCategory::Shell)
                .await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn session_grant_works_for_non_denied_tools() {
        // Allow rule + grant: grant should short-circuit
        let rules = vec![PermissionRule::new(RuleKind::Ask, "think")];
        let checker = PermissionChecker::new(rules);
        checker.grants.lock().unwrap().grant_shape(
            "think".to_string(),
            crate::approval::ArgShape("think:*".into()),
        );
        assert_eq!(
            checker
                .check("think", "", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn shape_aware_grant_only_allows_matching_shape() {
        use crate::approval::ArgShape;
        // Allow once for `shell` with command "ls" should not auto-allow
        // a later `shell` with command "rm -rf /".
        let rules = vec![PermissionRule::new(RuleKind::Ask, "shell")];
        let checker =
            PermissionChecker::new(rules).with_prompter(Box::new(MockPrompter::allow_always()));
        // First call: prompted, granted for shape "shell:ls".
        let safe_input = r#"{"command":"ls"}"#;
        assert_eq!(
            checker
                .check("shell", safe_input, PermissionCategory::Shell)
                .await,
            PermissionDecision::Allowed
        );
        let safe_shape = ArgShape::for_tool("shell", safe_input);
        assert!(
            checker
                .grants
                .lock()
                .unwrap()
                .is_granted_shape("shell", &safe_shape)
        );
        let dangerous_shape = ArgShape::for_tool("shell", r#"{"command":"rm -rf /"}"#);
        assert!(
            !checker
                .grants
                .lock()
                .unwrap()
                .is_granted_shape("shell", &dangerous_shape)
        );
    }

    #[tokio::test]
    async fn fail_closed_when_no_prompter() {
        let rules = vec![PermissionRule::new(RuleKind::Ask, "shell")];
        let checker = PermissionChecker::new(rules);
        // No prompter → Ask resolves to Denied.
        assert_eq!(
            checker
                .check("shell", r#"{"command":"ls"}"#, PermissionCategory::Shell)
                .await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn clear_grants_resets() {
        let rules = vec![PermissionRule::new(RuleKind::Deny, "Bash")];
        let checker = PermissionChecker::new(rules);
        checker.grants.lock().unwrap().grant_shape(
            "Bash".to_string(),
            crate::approval::ArgShape("Bash:*".into()),
        );
        checker.clear_grants();
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Denied
        );
    }

    #[test]
    fn truncate_input_short() {
        assert_eq!(truncate_input("hello", 10), "hello");
    }

    #[test]
    fn truncate_input_long() {
        let long = "a".repeat(300);
        let truncated = truncate_input(&long, 200);
        assert_eq!(truncated.len(), 203); // 200 + "…" (3 bytes)
    }

    #[test]
    fn truncate_input_multibyte_utf8_no_panic() {
        // Each emoji is 4 bytes. 60 emojis = 240 bytes > 200 byte limit.
        // Slicing at byte 200 would land mid-character and panic without floor_char_boundary.
        let emojis = "🦀".repeat(60);
        let truncated = truncate_input(&emojis, 200);
        // Should truncate at a char boundary (200 / 4 = 50 emojis exactly)
        assert!(truncated.len() <= 203); // at most 200 + "…" (3 bytes)
        assert!(truncated.ends_with('…'));
    }

    #[tokio::test]
    async fn update_rules_hot_reload() {
        // Start with no rules — default mode denies Bash (asks, but no prompter)
        let checker = PermissionChecker::new(vec![]);
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Denied
        );

        // Hot-reload: add an allow rule for Bash
        checker.update_rules(vec![PermissionRule::new(RuleKind::Allow, "Bash")]);
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Allowed
        );

        // Hot-reload again: deny Bash
        checker.update_rules(vec![PermissionRule::new(RuleKind::Deny, "Bash")]);
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn update_mode_hot_reload() {
        let checker = PermissionChecker::new(vec![]);

        // Default mode: Bash denied (asks, no prompter)
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Denied
        );

        // Hot-reload to bypass mode
        checker.update_mode(PermissionMode::Full);
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Allowed
        );

        // Hot-reload back to default
        checker.update_mode(PermissionMode::Ask);
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Denied
        );
    }

    // T-0347: enum serde strings are now `ask | edits | full | plan`
    // (was `default | accept_edits | bypass | plan`). No backward
    // compat — old strings should fail to deserialize.
    #[test]
    fn permission_mode_serde() {
        for (variant, want) in [
            (PermissionMode::Ask, "\"ask\""),
            (PermissionMode::Edits, "\"edits\""),
            (PermissionMode::Full, "\"full\""),
            (PermissionMode::Plan, "\"plan\""),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, want);
            let round: PermissionMode = serde_json::from_str(want).unwrap();
            assert_eq!(round, variant);
        }
    }

    #[test]
    fn permission_mode_legacy_strings_fail() {
        // Pre-T-0347 strings are not accepted — breaking change.
        for legacy in ["\"default\"", "\"accept_edits\"", "\"bypass\""] {
            assert!(
                serde_json::from_str::<PermissionMode>(legacy).is_err(),
                "{legacy} should not deserialize"
            );
        }
    }

    #[tokio::test]
    async fn plan_mode_allows_read_only() {
        let checker = PermissionChecker::new(vec![]).with_mode(PermissionMode::Plan);
        assert_eq!(
            checker
                .check("Read", "/foo", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("Glob", "*.rs", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("Think", "hmm", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("WebSearch", "query", PermissionCategory::ReadOnly)
                .await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn plan_mode_denies_writes() {
        let checker = PermissionChecker::new(vec![]).with_mode(PermissionMode::Plan);
        assert_eq!(
            checker
                .check("Edit", "/foo.rs", PermissionCategory::FileWrite)
                .await,
            PermissionDecision::Denied
        );
        assert_eq!(
            checker
                .check("Write", "/bar.rs", PermissionCategory::FileWrite)
                .await,
            PermissionDecision::Denied
        );
        assert_eq!(
            checker.check("Bash", "ls", PermissionCategory::Shell).await,
            PermissionDecision::Denied
        );
        assert_eq!(
            checker
                .check("AgentTool", "", PermissionCategory::Other)
                .await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn plan_mode_allows_plan_meta_tools() {
        let checker = PermissionChecker::new(vec![]).with_mode(PermissionMode::Plan);
        assert_eq!(
            checker
                .check("enter_plan_mode", "", PermissionCategory::Other)
                .await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker
                .check("exit_plan_mode", "", PermissionCategory::Other)
                .await,
            PermissionDecision::Allowed
        );
    }

    // T-0196: explained checks should attribute decisions to the matching
    // rule (or to the mode default when no rule fires) so callers can
    // surface "denied by rule X" instead of an opaque "denied".

    #[tokio::test]
    async fn check_explained_attributes_deny_to_matching_rule() {
        let rules = vec![PermissionRule::parse(
            crate::permissions::rules::RuleKind::Deny,
            "shell(rm -rf *)",
        )];
        let checker = PermissionChecker::new(rules);
        let (decision, reason) = checker
            .check_explained("shell", "rm -rf /tmp/foo", PermissionCategory::Shell)
            .await;
        assert_eq!(decision, PermissionDecision::Denied);
        let display = reason.display();
        assert!(
            display.contains("deny"),
            "expected rule kind in reason: {display}"
        );
        assert!(
            display.contains("shell(rm -rf *)"),
            "expected rule spec: {display}"
        );
    }

    #[tokio::test]
    async fn check_explained_attributes_no_match_to_mode_fallback() {
        // No rules at all → ReadOnly tool in Default mode → allowed via mode fallback.
        let checker = PermissionChecker::new(vec![]);
        let (decision, reason) = checker
            .check_explained("file_read", "{}", PermissionCategory::ReadOnly)
            .await;
        assert_eq!(decision, PermissionDecision::Allowed);
        let display = reason.display();
        assert!(
            display.contains("mode default"),
            "expected mode default reason: {display}"
        );
    }

    #[tokio::test]
    async fn audit_log_records_decisions_in_order_and_caps() {
        let audit = super::new_shared_audit();
        let checker = PermissionChecker::new(vec![]).with_audit(std::sync::Arc::clone(&audit));
        // Drive AUDIT_CAP + 5 distinct decisions; oldest should evict.
        for i in 0..(super::AUDIT_CAP + 5) {
            let _ = checker
                .check(&format!("tool_{i}"), "", PermissionCategory::ReadOnly)
                .await;
        }
        let buf = audit.lock().unwrap();
        assert_eq!(buf.len(), super::AUDIT_CAP);
        // Oldest entries (tool_0..tool_4) should have evicted; first remaining is tool_5.
        assert_eq!(buf.front().unwrap().tool_name, "tool_5");
        // Last entry should be the most recent (tool_AUDIT_CAP+4).
        assert_eq!(
            buf.back().unwrap().tool_name,
            format!("tool_{}", super::AUDIT_CAP + 4)
        );
    }

    #[tokio::test]
    async fn shared_audit_aggregates_across_checkers() {
        // Simulates the production shape: LocalService holds one SharedAudit
        // and constructs per-message PermissionCheckers that all log into it.
        use std::sync::Arc;
        let audit = super::new_shared_audit();
        let checker_a = PermissionChecker::new(vec![]).with_audit(Arc::clone(&audit));
        let checker_b = PermissionChecker::new(vec![]).with_audit(Arc::clone(&audit));
        let _ = checker_a
            .check("tool_a", "", PermissionCategory::ReadOnly)
            .await;
        let _ = checker_b
            .check("tool_b", "", PermissionCategory::ReadOnly)
            .await;
        let buf = audit.lock().unwrap();
        assert_eq!(buf.len(), 2);
        assert_eq!(buf[0].tool_name, "tool_a");
        assert_eq!(buf[1].tool_name, "tool_b");
    }
}

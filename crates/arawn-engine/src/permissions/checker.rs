use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::rules::{PermissionDecision, PermissionRule, RuleMatcher};

/// Permission mode — controls fallback behavior when no explicit rule matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PermissionMode {
    /// Read-only tools auto-allowed, write/shell tools trigger Ask.
    #[default]
    Default,
    /// File tools (read + write) auto-allowed, shell still triggers Ask.
    #[serde(rename = "accept_edits")]
    AcceptEdits,
    /// Everything auto-allowed — power user / CI mode.
    #[serde(rename = "bypass")]
    BypassPermissions,
    /// Plan mode — only side-effect-free tools allowed. All tools with side
    /// effects are denied (not asked — denied outright). The agent can observe
    /// the world but cannot take action until the plan is approved.
    #[serde(rename = "plan")]
    Plan,
}


/// Category of a tool for permission mode fallback decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    /// Side-effect-free / observation only — safe to auto-allow.
    /// These are the only tools permitted in plan mode.
    /// Examples: Read, Glob, Grep, Think, Sleep, WebSearch, WebFetch.
    ReadOnly,
    /// Modifies files but not the broader system (Edit, Write).
    FileWrite,
    /// Executes arbitrary commands — highest risk (Bash/Shell).
    Shell,
    /// Anything else (agents, web fetch, etc.) — treated conservatively.
    Other,
}

/// Determine the category of a tool by name. Data-driven lookup so new tools
/// get a sensible default without modifying the Tool trait.
pub fn tool_category(tool_name: &str) -> ToolCategory {
    match tool_name {
        // Read-only tools
        "Read" | "FileRead" | "file_read" => ToolCategory::ReadOnly,
        "Glob" | "glob" => ToolCategory::ReadOnly,
        "Grep" | "grep" => ToolCategory::ReadOnly,
        "Think" | "think" => ToolCategory::ReadOnly,
        "Sleep" | "sleep" => ToolCategory::ReadOnly,
        "task_list" | "task_get" | "task_create" | "task_update" => ToolCategory::ReadOnly,
        "WebSearch" | "web_search" => ToolCategory::ReadOnly,
        "WebFetch" | "web_fetch" => ToolCategory::ReadOnly,
        "AskUser" | "ask_user" => ToolCategory::ReadOnly,

        // File write tools
        "Edit" | "FileEdit" | "file_edit" => ToolCategory::FileWrite,
        "Write" | "FileWrite" | "file_write" => ToolCategory::FileWrite,

        // Shell tools
        "Bash" | "Shell" | "shell" => ToolCategory::Shell,

        // Everything else
        _ => ToolCategory::Other,
    }
}

impl PermissionMode {
    /// Determine the fallback decision for a tool when no explicit rule matched.
    pub fn fallback(&self, tool_name: &str) -> PermissionDecision {
        match self {
            PermissionMode::Default => match tool_category(tool_name) {
                ToolCategory::ReadOnly => PermissionDecision::Allowed,
                ToolCategory::FileWrite => PermissionDecision::Ask,
                ToolCategory::Shell => PermissionDecision::Ask,
                ToolCategory::Other => PermissionDecision::Ask,
            },
            PermissionMode::AcceptEdits => match tool_category(tool_name) {
                ToolCategory::ReadOnly => PermissionDecision::Allowed,
                ToolCategory::FileWrite => PermissionDecision::Allowed,
                ToolCategory::Shell => PermissionDecision::Ask,
                ToolCategory::Other => PermissionDecision::Ask,
            },
            PermissionMode::BypassPermissions => PermissionDecision::Allowed,
            PermissionMode::Plan => match tool_category(tool_name) {
                ToolCategory::ReadOnly => PermissionDecision::Allowed,
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
/// When a user selects "Allow Always", the tool is added here
/// and won't be prompted again for the rest of the session.
#[derive(Debug, Default)]
pub struct SessionGrants {
    grants: std::collections::HashSet<String>,
}

impl SessionGrants {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a session grant for a tool name.
    pub fn grant(&mut self, tool_name: String) {
        self.grants.insert(tool_name);
    }

    /// Check if a tool has been granted for this session.
    pub fn is_granted(&self, tool_name: &str) -> bool {
        self.grants.contains(tool_name)
    }

    /// Clear all session grants.
    pub fn clear(&mut self) {
        self.grants.clear();
    }
}

/// The central permission checker. Evaluates rules against tool calls,
/// prompts the user when needed, and manages session grants.
pub struct PermissionChecker {
    rules: std::sync::RwLock<Vec<PermissionRule>>,
    mode: std::sync::RwLock<PermissionMode>,
    grants: std::sync::Mutex<SessionGrants>,
    prompter: Option<Box<dyn ModalPrompt>>,
}

impl PermissionChecker {
    /// Create a new permission checker with the given rules and default mode.
    /// Without a prompter, `Ask` decisions are treated as `Denied`.
    pub fn new(rules: Vec<PermissionRule>) -> Self {
        Self {
            rules: std::sync::RwLock::new(rules),
            mode: std::sync::RwLock::new(PermissionMode::Default),
            grants: std::sync::Mutex::new(SessionGrants::new()),
            prompter: None,
        }
    }

    /// Set the permission mode (Default, AcceptEdits, BypassPermissions).
    pub fn with_mode(self, mode: PermissionMode) -> Self {
        if mode == PermissionMode::BypassPermissions {
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

    /// Check if a tool call is permitted.
    ///
    /// Flow:
    /// 1. Evaluate deny rules first (deny always wins, even over session grants)
    /// 2. Check session grants (short-circuit if granted and not denied)
    /// 3. Evaluate remaining rules: allow > ask > no match
    /// 4. If Ask, prompt the user (or deny if no prompter)
    /// 5. NoMatch falls through to permission mode fallback
    pub async fn check(&self, tool_name: &str, tool_input: &str) -> PermissionDecision {
        // 1. Evaluate rules — deny rules always take priority
        let decision = {
            let rules = self.rules.read().unwrap();
            RuleMatcher::evaluate(&rules, tool_name, tool_input)
        };
        debug!(tool_name, ?decision, "permission rule evaluation");

        // Deny rules override everything, including session grants
        if decision == PermissionDecision::Denied {
            warn!(tool_name, "permission denied by rule (overrides session grants)");
            return PermissionDecision::Denied;
        }

        // 2. Session grants short-circuit (only checked after deny rules pass)
        if self.grants.lock().unwrap().is_granted(tool_name) {
            debug!(tool_name, "session grant hit — allowed");
            return PermissionDecision::Allowed;
        }

        // 3. Remaining rule decisions
        match decision {
            PermissionDecision::Allowed => PermissionDecision::Allowed,
            PermissionDecision::Denied => unreachable!("handled above"),
            PermissionDecision::Ask => {
                self.prompt_user(tool_name, tool_input).await
            }
            // NoMatch = no explicit rule applies. Fall back to permission mode.
            PermissionDecision::NoMatch => {
                let mode = *self.mode.read().unwrap();
                let fallback = mode.fallback(tool_name);
                debug!(tool_name, ?fallback, ?mode, "mode fallback");
                match fallback {
                    PermissionDecision::Ask => {
                        self.prompt_user(tool_name, tool_input).await
                    }
                    other => other,
                }
            }
        }
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
                        ModalOption::new("Allow Once").with_description("Allow this tool call only"),
                        ModalOption::new("Allow Always").with_description("Allow for the rest of the session"),
                        ModalOption::new("Deny").with_description("Block this tool call"),
                    ],
                };
                match prompter.prompt(request).await {
                    Some(0) => PermissionDecision::Allowed,                // Allow Once
                    Some(1) => {                                            // Allow Always
                        self.grants.lock().unwrap().grant(tool_name.to_string());
                        PermissionDecision::Allowed
                    }
                    _ => PermissionDecision::Denied,                        // Deny or cancel
                }
            }
            None => {
                warn!(tool_name, "ask decision but no prompter — denying");
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
        fn allow_once() -> Self { Self { index: Some(0) } }
        fn allow_always() -> Self { Self { index: Some(1) } }
        fn deny() -> Self { Self { index: Some(2) } }
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
            checker.check("Read", "/foo").await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn denied_by_rule() {
        let rules = vec![PermissionRule::new(RuleKind::Deny, "Bash")];
        let checker = PermissionChecker::new(rules);
        assert_eq!(
            checker.check("Bash", "rm -rf /").await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn ask_without_prompter_denies() {
        let rules = vec![PermissionRule::new(RuleKind::Ask, "Bash")];
        let checker = PermissionChecker::new(rules);
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn ask_with_allow_once() {
        let rules = vec![PermissionRule::new(RuleKind::Ask, "Bash")];
        let checker = PermissionChecker::new(rules).with_prompter(Box::new(MockPrompter::allow_once()));
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Allowed
        );
        // Not granted for session — next call should ask again
        assert!(!checker.grants.lock().unwrap().is_granted("Bash"));
    }

    #[tokio::test]
    async fn ask_with_allow_always_grants_session() {
        let rules = vec![PermissionRule::new(RuleKind::Ask, "Bash")];
        let checker = PermissionChecker::new(rules).with_prompter(Box::new(MockPrompter::allow_always()));
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Allowed
        );
        // Session grant recorded — subsequent calls skip prompting
        assert!(checker.grants.lock().unwrap().is_granted("Bash"));
        assert_eq!(
            checker.check("Bash", "cargo test").await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn ask_with_deny() {
        let rules = vec![PermissionRule::new(RuleKind::Ask, "Edit")];
        let checker = PermissionChecker::new(rules).with_prompter(Box::new(MockPrompter::deny()));
        assert_eq!(
            checker.check("Edit", "/foo.rs").await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn default_mode_allows_read_only() {
        let checker = PermissionChecker::new(vec![]);
        // Read-only tools auto-allowed in Default mode
        assert_eq!(
            checker.check("Read", "/foo").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("Glob", "*.rs").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("Grep", "pattern").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("Think", "hmm").await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn default_mode_asks_for_writes() {
        // Without a prompter, Ask → Denied
        let checker = PermissionChecker::new(vec![]);
        assert_eq!(
            checker.check("Edit", "/foo.rs").await,
            PermissionDecision::Denied
        );
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Denied
        );
        assert_eq!(
            checker.check("Write", "/bar.rs").await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn accept_edits_mode_allows_file_ops() {
        let checker = PermissionChecker::new(vec![]).with_mode(PermissionMode::AcceptEdits);
        // File ops allowed
        assert_eq!(
            checker.check("Read", "/foo").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("Edit", "/foo.rs").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("Write", "/bar.rs").await,
            PermissionDecision::Allowed
        );
        // Shell still asks (denied without prompter)
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn bypass_mode_allows_everything() {
        let checker = PermissionChecker::new(vec![]).with_mode(PermissionMode::BypassPermissions);
        assert_eq!(
            checker.check("Read", "/foo").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("Edit", "/foo.rs").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("Bash", "rm -rf /").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("UnknownTool", "").await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn explicit_rules_override_mode() {
        // Even in Default mode, an explicit Allow rule for Bash should work
        let rules = vec![PermissionRule::new(RuleKind::Allow, "Bash")];
        let checker = PermissionChecker::new(rules);
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn deny_rules_override_session_grants() {
        let rules = vec![PermissionRule::new(RuleKind::Deny, "Bash")];
        let checker = PermissionChecker::new(rules);
        // Manually grant — deny rule should still win
        checker.grants.lock().unwrap().grant("Bash".to_string());
        assert_eq!(
            checker.check("Bash", "rm -rf /").await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn session_grant_works_for_non_denied_tools() {
        // Allow rule + grant: grant should short-circuit
        let rules = vec![PermissionRule::new(RuleKind::Ask, "think")];
        let checker = PermissionChecker::new(rules);
        checker.grants.lock().unwrap().grant("think".to_string());
        assert_eq!(
            checker.check("think", "").await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn clear_grants_resets() {
        let rules = vec![PermissionRule::new(RuleKind::Deny, "Bash")];
        let checker = PermissionChecker::new(rules);
        checker.grants.lock().unwrap().grant("Bash".to_string());
        checker.clear_grants();
        assert_eq!(
            checker.check("Bash", "ls").await,
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

    #[test]
    fn tool_categories() {
        assert_eq!(tool_category("Read"), ToolCategory::ReadOnly);
        assert_eq!(tool_category("Glob"), ToolCategory::ReadOnly);
        assert_eq!(tool_category("Grep"), ToolCategory::ReadOnly);
        assert_eq!(tool_category("Think"), ToolCategory::ReadOnly);
        assert_eq!(tool_category("Edit"), ToolCategory::FileWrite);
        assert_eq!(tool_category("Write"), ToolCategory::FileWrite);
        assert_eq!(tool_category("Bash"), ToolCategory::Shell);
        assert_eq!(tool_category("Shell"), ToolCategory::Shell);
        assert_eq!(tool_category("WebSearch"), ToolCategory::ReadOnly);
        assert_eq!(tool_category("WebFetch"), ToolCategory::ReadOnly);
        assert_eq!(tool_category("AskUser"), ToolCategory::ReadOnly);
        assert_eq!(tool_category("AgentTool"), ToolCategory::Other);
    }

    #[tokio::test]
    async fn update_rules_hot_reload() {
        // Start with no rules — default mode denies Bash (asks, but no prompter)
        let checker = PermissionChecker::new(vec![]);
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Denied
        );

        // Hot-reload: add an allow rule for Bash
        checker.update_rules(vec![PermissionRule::new(RuleKind::Allow, "Bash")]);
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Allowed
        );

        // Hot-reload again: deny Bash
        checker.update_rules(vec![PermissionRule::new(RuleKind::Deny, "Bash")]);
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn update_mode_hot_reload() {
        let checker = PermissionChecker::new(vec![]);

        // Default mode: Bash denied (asks, no prompter)
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Denied
        );

        // Hot-reload to bypass mode
        checker.update_mode(PermissionMode::BypassPermissions);
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Allowed
        );

        // Hot-reload back to default
        checker.update_mode(PermissionMode::Default);
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Denied
        );
    }

    #[test]
    fn permission_mode_serde() {
        let json = serde_json::to_string(&PermissionMode::AcceptEdits).unwrap();
        assert_eq!(json, "\"accept_edits\"");
        let mode: PermissionMode = serde_json::from_str("\"bypass\"").unwrap();
        assert_eq!(mode, PermissionMode::BypassPermissions);
        let mode: PermissionMode = serde_json::from_str("\"default\"").unwrap();
        assert_eq!(mode, PermissionMode::Default);
        let mode: PermissionMode = serde_json::from_str("\"plan\"").unwrap();
        assert_eq!(mode, PermissionMode::Plan);
    }

    #[tokio::test]
    async fn plan_mode_allows_read_only() {
        let checker = PermissionChecker::new(vec![]).with_mode(PermissionMode::Plan);
        assert_eq!(
            checker.check("Read", "/foo").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("Glob", "*.rs").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("Think", "hmm").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("WebSearch", "query").await,
            PermissionDecision::Allowed
        );
    }

    #[tokio::test]
    async fn plan_mode_denies_writes() {
        let checker = PermissionChecker::new(vec![]).with_mode(PermissionMode::Plan);
        assert_eq!(
            checker.check("Edit", "/foo.rs").await,
            PermissionDecision::Denied
        );
        assert_eq!(
            checker.check("Write", "/bar.rs").await,
            PermissionDecision::Denied
        );
        assert_eq!(
            checker.check("Bash", "ls").await,
            PermissionDecision::Denied
        );
        assert_eq!(
            checker.check("AgentTool", "").await,
            PermissionDecision::Denied
        );
    }

    #[tokio::test]
    async fn plan_mode_allows_plan_meta_tools() {
        let checker = PermissionChecker::new(vec![]).with_mode(PermissionMode::Plan);
        assert_eq!(
            checker.check("enter_plan_mode", "").await,
            PermissionDecision::Allowed
        );
        assert_eq!(
            checker.check("exit_plan_mode", "").await,
            PermissionDecision::Allowed
        );
    }
}

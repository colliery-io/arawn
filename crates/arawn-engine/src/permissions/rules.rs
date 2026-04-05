use serde::{Deserialize, Serialize};

/// The kind of permission rule — what happens when it matches.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleKind {
    Allow,
    Deny,
    Ask,
}

/// A single permission rule: a kind (allow/deny/ask), a tool name pattern,
/// and an optional content pattern for matching tool input.
///
/// # Format
///
/// Rules are specified as strings in settings:
/// - `"Read"` — exact tool name match
/// - `"file_*"` — glob match on tool name
/// - `"Bash(git *)"` — tool name + content pattern (matches Bash calls where input starts with "git")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PermissionRule {
    pub kind: RuleKind,
    pub tool_pattern: String,
    /// Optional content pattern — if present, the tool input must also match this glob.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_pattern: Option<String>,
}

impl PermissionRule {
    pub fn new(kind: RuleKind, tool_pattern: impl Into<String>) -> Self {
        Self {
            kind,
            tool_pattern: tool_pattern.into(),
            content_pattern: None,
        }
    }

    pub fn with_content(mut self, pattern: impl Into<String>) -> Self {
        self.content_pattern = Some(pattern.into());
        self
    }

    /// Parse a rule from the compact string format: `"ToolName"` or `"ToolName(content pattern)"`.
    pub fn parse(kind: RuleKind, spec: &str) -> Self {
        if let Some(paren_pos) = spec.find('(') {
            let tool_pattern = spec[..paren_pos].to_string();
            let inner = &spec[paren_pos + 1..];
            // Strip only the final closing paren (the one wrapping the content pattern)
            let content = if inner.ends_with(')') {
                &inner[..inner.len() - 1]
            } else {
                inner
            };
            Self {
                kind,
                tool_pattern,
                content_pattern: Some(content.to_string()),
            }
        } else {
            Self::new(kind, spec)
        }
    }

    /// Check if this rule matches a given tool name and input.
    pub fn matches(&self, tool_name: &str, tool_input: &str) -> bool {
        if !glob_match(&self.tool_pattern, tool_name) {
            return false;
        }
        match &self.content_pattern {
            Some(pattern) => glob_match(pattern, tool_input),
            None => true,
        }
    }
}

/// The result of evaluating permission rules against a tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Tool is explicitly allowed — proceed with execution.
    Allowed,
    /// Tool is explicitly denied — return error to LLM.
    Denied,
    /// User must be prompted for permission.
    Ask,
    /// No rule matched — fall back to permission mode default.
    NoMatch,
}

/// Evaluates a list of permission rules against a tool call.
///
/// Evaluation order: deny rules first (highest priority), then allow, then ask.
/// First match in each category wins. If no rules match, returns `NoMatch`.
pub struct RuleMatcher;

impl RuleMatcher {
    /// Evaluate rules against a tool call.
    ///
    /// Priority: deny > allow > ask > no match.
    pub fn evaluate(
        rules: &[PermissionRule],
        tool_name: &str,
        tool_input: &str,
    ) -> PermissionDecision {
        // Check deny rules first
        for rule in rules {
            if rule.kind == RuleKind::Deny && rule.matches(tool_name, tool_input) {
                return PermissionDecision::Denied;
            }
        }

        // Check allow rules
        for rule in rules {
            if rule.kind == RuleKind::Allow && rule.matches(tool_name, tool_input) {
                return PermissionDecision::Allowed;
            }
        }

        // Check ask rules
        for rule in rules {
            if rule.kind == RuleKind::Ask && rule.matches(tool_name, tool_input) {
                return PermissionDecision::Ask;
            }
        }

        PermissionDecision::NoMatch
    }
}

/// Simple glob matching supporting `*` (any chars) and `?` (single char).
/// Case-sensitive. No `**` or brace expansion — keep it simple.
fn glob_match(pattern: &str, text: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    glob_match_inner(&pat, &txt)
}

fn glob_match_inner(pat: &[char], txt: &[char]) -> bool {
    let mut pi = 0;
    let mut ti = 0;
    let mut star_pi = usize::MAX;
    let mut star_ti = 0;

    while ti < txt.len() {
        if pi < pat.len() && (pat[pi] == '?' || pat[pi] == txt[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < pat.len() && pat[pi] == '*' {
            star_pi = pi;
            star_ti = ti;
            pi += 1;
        } else if star_pi != usize::MAX {
            pi = star_pi + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    while pi < pat.len() && pat[pi] == '*' {
        pi += 1;
    }

    pi == pat.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- glob_match tests ---

    #[test]
    fn glob_exact_match() {
        assert!(glob_match("Bash", "Bash"));
        assert!(!glob_match("Bash", "Read"));
    }

    #[test]
    fn glob_star_match() {
        assert!(glob_match("file_*", "file_read"));
        assert!(glob_match("file_*", "file_write"));
        assert!(glob_match("file_*", "file_"));
        assert!(!glob_match("file_*", "glob"));
    }

    #[test]
    fn glob_question_mark() {
        assert!(glob_match("Rea?", "Read"));
        assert!(!glob_match("Rea?", "Real!"));
    }

    #[test]
    fn glob_complex_patterns() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*Tool", "ShellTool"));
        assert!(glob_match("A*B*C", "AxBxC"));
        assert!(!glob_match("A*B*C", "AxBx"));
    }

    #[test]
    fn glob_content_patterns() {
        assert!(glob_match("git *", "git push"));
        assert!(glob_match("git *", "git commit -m 'foo'"));
        assert!(!glob_match("git *", "rm -rf /"));
        assert!(glob_match("rm *", "rm -rf /"));
    }

    // --- PermissionRule tests ---

    #[test]
    fn rule_exact_tool_match() {
        let rule = PermissionRule::new(RuleKind::Allow, "Read");
        assert!(rule.matches("Read", ""));
        assert!(!rule.matches("Write", ""));
    }

    #[test]
    fn rule_glob_tool_match() {
        let rule = PermissionRule::new(RuleKind::Allow, "file_*");
        assert!(rule.matches("file_read", ""));
        assert!(rule.matches("file_write", ""));
        assert!(!rule.matches("shell", ""));
    }

    #[test]
    fn rule_with_content_pattern() {
        let rule = PermissionRule::new(RuleKind::Allow, "Bash").with_content("git *");
        assert!(rule.matches("Bash", "git push origin main"));
        assert!(!rule.matches("Bash", "rm -rf /"));
        assert!(!rule.matches("Read", "git push"));
    }

    #[test]
    fn rule_parse_simple() {
        let rule = PermissionRule::parse(RuleKind::Deny, "Bash");
        assert_eq!(rule.tool_pattern, "Bash");
        assert_eq!(rule.content_pattern, None);
    }

    #[test]
    fn rule_parse_with_content() {
        let rule = PermissionRule::parse(RuleKind::Allow, "Bash(git *)");
        assert_eq!(rule.tool_pattern, "Bash");
        assert_eq!(rule.content_pattern, Some("git *".to_string()));
    }

    #[test]
    fn rule_parse_nested_parens() {
        // Content with parens inside — only split on first (
        let rule = PermissionRule::parse(RuleKind::Ask, "Bash(echo foo(bar))");
        assert_eq!(rule.tool_pattern, "Bash");
        assert_eq!(rule.content_pattern, Some("echo foo(bar)".to_string()));
    }

    // --- RuleMatcher tests ---

    #[test]
    fn matcher_deny_takes_priority() {
        let rules = vec![
            PermissionRule::new(RuleKind::Allow, "Bash"),
            PermissionRule::new(RuleKind::Deny, "Bash"),
        ];
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Bash", "ls"),
            PermissionDecision::Denied
        );
    }

    #[test]
    fn matcher_allow_before_ask() {
        let rules = vec![
            PermissionRule::new(RuleKind::Ask, "Bash"),
            PermissionRule::new(RuleKind::Allow, "Bash"),
        ];
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Bash", "ls"),
            PermissionDecision::Allowed
        );
    }

    #[test]
    fn matcher_ask_when_only_ask_rule() {
        let rules = vec![PermissionRule::new(RuleKind::Ask, "Bash")];
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Bash", "ls"),
            PermissionDecision::Ask
        );
    }

    #[test]
    fn matcher_no_match_when_no_rules() {
        assert_eq!(
            RuleMatcher::evaluate(&[], "Bash", "ls"),
            PermissionDecision::NoMatch
        );
    }

    #[test]
    fn matcher_no_match_when_rules_dont_apply() {
        let rules = vec![PermissionRule::new(RuleKind::Allow, "Read")];
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Bash", "ls"),
            PermissionDecision::NoMatch
        );
    }

    #[test]
    fn matcher_content_pattern_deny() {
        let rules = vec![
            PermissionRule::new(RuleKind::Allow, "Bash"),
            PermissionRule::new(RuleKind::Deny, "Bash").with_content("rm *"),
        ];
        // "rm -rf /" matches the deny rule
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Bash", "rm -rf /"),
            PermissionDecision::Denied
        );
        // "ls" doesn't match deny, but matches allow
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Bash", "ls"),
            PermissionDecision::Allowed
        );
    }

    #[test]
    fn matcher_mixed_rules_realistic() {
        let rules = vec![
            // Read-only tools always allowed
            PermissionRule::new(RuleKind::Allow, "Read"),
            PermissionRule::new(RuleKind::Allow, "Glob"),
            PermissionRule::new(RuleKind::Allow, "Grep"),
            PermissionRule::new(RuleKind::Allow, "Think"),
            // Git commands allowed in Bash
            PermissionRule::parse(RuleKind::Allow, "Bash(git *)"),
            // Destructive commands denied
            PermissionRule::parse(RuleKind::Deny, "Bash(rm -rf *)"),
            // Everything else asks
            PermissionRule::new(RuleKind::Ask, "Bash"),
            PermissionRule::new(RuleKind::Ask, "Edit"),
            PermissionRule::new(RuleKind::Ask, "Write"),
        ];

        assert_eq!(
            RuleMatcher::evaluate(&rules, "Read", "/foo"),
            PermissionDecision::Allowed
        );
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Grep", "pattern"),
            PermissionDecision::Allowed
        );
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Bash", "git push"),
            PermissionDecision::Allowed
        );
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Bash", "rm -rf /"),
            PermissionDecision::Denied
        );
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Bash", "cargo test"),
            PermissionDecision::Ask
        );
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Edit", "/foo.rs"),
            PermissionDecision::Ask
        );
        assert_eq!(
            RuleMatcher::evaluate(&rules, "UnknownTool", ""),
            PermissionDecision::NoMatch
        );
    }
}

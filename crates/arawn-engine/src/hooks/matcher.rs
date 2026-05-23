use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Matches hook events by a filterable field value (tool name, source, notification type, etc.)
/// and an optional content pattern.
///
/// # Matcher syntax
///
/// - `"Bash"` — exact match
/// - `"Bash|Edit"` — pipe-separated alternatives (any match)
/// - `"file_*"` — glob match (`*` = any chars, `?` = single char)
/// - `"Bash(git *)"` — tool name + content pattern
/// - `""` or absent — matches everything
///
/// Serializes to/from a plain JSON string (e.g. `"Bash"`, `"Edit(*.rs)"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HookMatcher {
    /// The raw matcher string as specified in config.
    pub raw: String,
}

impl Serialize for HookMatcher {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for HookMatcher {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(HookMatcher { raw: s })
    }
}

impl HookMatcher {
    pub fn new(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }

    /// Check if this matcher matches a given field value and optional content string.
    ///
    /// `field_value` is the matchable field for the event type (e.g. tool_name, source).
    /// `content` is the full tool input or other content for content-pattern matching.
    pub fn matches(&self, field_value: &str, content: &str) -> bool {
        if self.raw.is_empty() {
            return true;
        }

        // Check for content pattern: "ToolName(pattern)"
        if let Some(paren_pos) = self.raw.find('(') {
            let name_part = &self.raw[..paren_pos];
            let inner = &self.raw[paren_pos + 1..];
            let pattern = inner.strip_suffix(')').unwrap_or(inner);

            // Name part can be pipe-separated
            let name_matches = self.matches_alternatives(name_part, field_value);
            let content_matches = glob_match(pattern, content);
            return name_matches && content_matches;
        }

        // No content pattern — match on field value only (supports pipe-separated)
        self.matches_alternatives(&self.raw, field_value)
    }

    /// Check pipe-separated alternatives: "Bash|Edit|Write"
    fn matches_alternatives(&self, spec: &str, value: &str) -> bool {
        if spec.contains('|') {
            spec.split('|').any(|alt| glob_match(alt.trim(), value))
        } else {
            glob_match(spec, value)
        }
    }
}

/// Simple glob matching supporting `*` (any chars) and `?` (single char).
/// Case-sensitive. No `**` or brace expansion.
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
    fn glob_exact() {
        assert!(glob_match("Bash", "Bash"));
        assert!(!glob_match("Bash", "Edit"));
    }

    #[test]
    fn glob_star() {
        assert!(glob_match("file_*", "file_read"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*Tool", "ShellTool"));
    }

    #[test]
    fn glob_question_mark() {
        assert!(glob_match("Rea?", "Read"));
        assert!(!glob_match("Rea?", "Reads"));
    }

    // --- HookMatcher tests ---

    #[test]
    fn empty_matcher_matches_everything() {
        let m = HookMatcher::new("");
        assert!(m.matches("Bash", "ls"));
        assert!(m.matches("Read", "/foo"));
        assert!(m.matches("", ""));
    }

    #[test]
    fn exact_tool_match() {
        let m = HookMatcher::new("Bash");
        assert!(m.matches("Bash", "ls"));
        assert!(!m.matches("Edit", "file.rs"));
    }

    #[test]
    fn pipe_separated_alternatives() {
        let m = HookMatcher::new("Bash|Edit|Write");
        assert!(m.matches("Bash", ""));
        assert!(m.matches("Edit", ""));
        assert!(m.matches("Write", ""));
        assert!(!m.matches("Read", ""));
    }

    #[test]
    fn glob_tool_match() {
        let m = HookMatcher::new("file_*");
        assert!(m.matches("file_read", ""));
        assert!(m.matches("file_write", ""));
        assert!(!m.matches("shell", ""));
    }

    #[test]
    fn content_pattern() {
        let m = HookMatcher::new("Bash(git *)");
        assert!(m.matches("Bash", "git push origin main"));
        assert!(m.matches("Bash", "git commit -m 'foo'"));
        assert!(!m.matches("Bash", "rm -rf /"));
        assert!(!m.matches("Edit", "git push"));
    }

    #[test]
    fn content_pattern_with_pipes() {
        let m = HookMatcher::new("Bash|Edit(*.rs)");
        // "Bash|Edit" as name part, "(*.rs)" as content pattern
        assert!(m.matches("Bash", "main.rs"));
        assert!(m.matches("Edit", "lib.rs"));
        assert!(!m.matches("Bash", "main.py"));
        assert!(!m.matches("Read", "main.rs"));
    }

    #[test]
    fn session_source_matching() {
        let m = HookMatcher::new("startup");
        assert!(m.matches("startup", ""));
        assert!(!m.matches("resume", ""));
    }

    #[test]
    fn wildcard_matches_any_tool() {
        let m = HookMatcher::new("*");
        assert!(m.matches("Bash", ""));
        assert!(m.matches("Read", ""));
        assert!(m.matches("anything", ""));
    }

    #[test]
    fn nested_parens_in_content() {
        let m = HookMatcher::new("Bash(echo foo(bar))");
        assert!(m.matches("Bash", "echo foo(bar)"));
    }
}

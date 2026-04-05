use serde::{Deserialize, Serialize};

/// A skill definition loaded from a markdown file with YAML frontmatter.
///
/// Skills are reusable prompt-based workflows invoked by name (like slash commands).
/// The markdown body is the prompt template injected into the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// Unique skill name (derived from filename or frontmatter).
    pub name: String,
    /// Description of when to use this skill (shown in system prompt).
    pub description: String,
    /// The prompt template (markdown body after frontmatter).
    pub prompt: String,
    /// Optional argument hint shown to the model (e.g. "[-m message]").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub argument_hint: Option<String>,
    /// Allowed tools when this skill is active (None = all tools).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    /// Model override (None = inherit from current session).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Whether users can invoke this skill directly (default: true).
    #[serde(default = "default_true")]
    pub user_invocable: bool,
    /// Source of this skill (project, user, plugin).
    #[serde(skip)]
    pub source: SkillSource,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum SkillSource {
    /// Project-level skill (.arawn/skills/)
    #[default]
    Project,
    /// User-level skill (~/.arawn/skills/)
    User,
    /// Plugin-provided skill
    Plugin(String),
    /// Built-in skill
    BuiltIn,
}

/// Parse a skill definition from a markdown file's content.
///
/// The file must have YAML frontmatter between `---` delimiters.
/// At minimum, `description` is required. `name` is optional (derived from filename if absent).
pub fn parse_skill_markdown(content: &str, default_name: &str) -> Result<SkillDefinition, String> {
    let (frontmatter, body) = split_frontmatter(content)
        .ok_or_else(|| "missing YAML frontmatter (expected --- delimiters)".to_string())?;

    let name = extract_field(&frontmatter, "name").unwrap_or_else(|| default_name.to_string());

    let description = extract_field(&frontmatter, "description")
        .ok_or_else(|| "missing required 'description' field in frontmatter".to_string())?;

    let argument_hint = extract_field(&frontmatter, "argument-hint");
    let allowed_tools = extract_list_field(&frontmatter, "allowed-tools");
    let model = extract_field(&frontmatter, "model")
        .map(|m| if m == "inherit" { None } else { Some(m) })
        .unwrap_or(None);
    let user_invocable = extract_field(&frontmatter, "user-invocable")
        .map(|v| v == "true")
        .unwrap_or(true);

    Ok(SkillDefinition {
        name,
        description,
        prompt: body.trim().to_string(),
        argument_hint,
        allowed_tools,
        model,
        user_invocable,
        source: SkillSource::default(),
    })
}

/// Split content into frontmatter and body at `---` delimiters.
fn split_frontmatter(content: &str) -> Option<(String, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }

    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")?;
    let frontmatter = after_first[..end].trim().to_string();
    let body = after_first[end + 4..].to_string();

    Some((frontmatter, body))
}

/// Extract a simple `key: value` field from YAML frontmatter.
fn extract_field(frontmatter: &str, key: &str) -> Option<String> {
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(value) = rest.strip_prefix(':') {
                let value = value.trim();
                let value = value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                    .unwrap_or(value);
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Extract a YAML list field (either inline `[a, b]` or multi-line `- a\n- b`).
fn extract_list_field(frontmatter: &str, key: &str) -> Option<Vec<String>> {
    let mut lines = frontmatter.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(value) = rest.strip_prefix(':') {
                let value = value.trim();
                // Inline array: [a, b, c]
                if value.starts_with('[') && value.ends_with(']') {
                    let inner = &value[1..value.len() - 1];
                    return Some(
                        inner
                            .split(',')
                            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                            .filter(|s| !s.is_empty())
                            .collect(),
                    );
                }
                // Empty value — look for multi-line list items
                if value.is_empty() {
                    let mut items = Vec::new();
                    while let Some(next_line) = lines.peek() {
                        let next_trimmed = next_line.trim();
                        if let Some(item) = next_trimmed.strip_prefix('-') {
                            let item = item.trim().trim_matches('"').trim_matches('\'');
                            items.push(item.to_string());
                            lines.next();
                        } else {
                            break;
                        }
                    }
                    if !items.is_empty() {
                        return Some(items);
                    }
                }
                // Single value on same line
                if !value.is_empty() {
                    return Some(vec![value.to_string()]);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_skill() {
        let content = r#"---
description: "A simple skill"
---

Do the thing.
"#;
        let skill = parse_skill_markdown(content, "test-skill").unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description, "A simple skill");
        assert_eq!(skill.prompt, "Do the thing.");
        assert!(skill.user_invocable);
        assert!(skill.allowed_tools.is_none());
        assert!(skill.model.is_none());
    }

    #[test]
    fn parse_full_skill() {
        let content = r#"---
name: "commit"
description: "Create a git commit with a conventional message"
argument-hint: "[-m message]"
user-invocable: true
model: sonnet
allowed-tools:
  - "Bash(git *)"
  - "Read"
  - "Grep"
---

# Commit workflow

Review staged changes and create a commit with a conventional message.
"#;
        let skill = parse_skill_markdown(content, "fallback").unwrap();
        assert_eq!(skill.name, "commit");
        assert_eq!(skill.description, "Create a git commit with a conventional message");
        assert_eq!(skill.argument_hint, Some("[-m message]".into()));
        assert_eq!(skill.model, Some("sonnet".into()));
        assert_eq!(
            skill.allowed_tools.as_ref().unwrap(),
            &["Bash(git *)", "Read", "Grep"]
        );
        assert!(skill.prompt.contains("Commit workflow"));
    }

    #[test]
    fn parse_inline_array() {
        let content = r#"---
description: "test"
allowed-tools: ["Bash", "Read"]
---

Body.
"#;
        let skill = parse_skill_markdown(content, "test").unwrap();
        assert_eq!(
            skill.allowed_tools.as_ref().unwrap(),
            &["Bash", "Read"]
        );
    }

    #[test]
    fn parse_model_inherit() {
        let content = r#"---
description: "test"
model: inherit
---

Body.
"#;
        let skill = parse_skill_markdown(content, "test").unwrap();
        assert!(skill.model.is_none());
    }

    #[test]
    fn parse_user_invocable_false() {
        let content = r#"---
description: "test"
user-invocable: false
---

Body.
"#;
        let skill = parse_skill_markdown(content, "test").unwrap();
        assert!(!skill.user_invocable);
    }

    #[test]
    fn parse_missing_description_errors() {
        let content = r#"---
name: "bad"
---

Body.
"#;
        let result = parse_skill_markdown(content, "bad");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("description"));
    }

    #[test]
    fn parse_no_frontmatter_errors() {
        let result = parse_skill_markdown("Just plain text", "test");
        assert!(result.is_err());
    }

    #[test]
    fn name_from_frontmatter_overrides_default() {
        let content = r#"---
name: "custom-name"
description: "test"
---

Body.
"#;
        let skill = parse_skill_markdown(content, "filename-name").unwrap();
        assert_eq!(skill.name, "custom-name");
    }

    #[test]
    fn split_frontmatter_works() {
        let (fm, body) = split_frontmatter("---\nname: test\n---\nBody here").unwrap();
        assert_eq!(fm, "name: test");
        assert!(body.contains("Body here"));
    }

    #[test]
    fn extract_list_multiline() {
        let fm = "allowed-tools:\n  - \"Bash(git *)\"\n  - Read\nother: value";
        let list = extract_list_field(fm, "allowed-tools").unwrap();
        assert_eq!(list, vec!["Bash(git *)", "Read"]);
    }
}

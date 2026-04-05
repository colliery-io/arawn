use std::path::Path;
use std::sync::Arc;

use tracing::{info, warn};

use crate::tool::ToolRegistry;

/// An agent definition — controls system prompt, tool access, and behavior.
#[derive(Debug, Clone)]
pub struct AgentDefinition {
    /// Unique name used as `subagent_type` (e.g., "Explore", "Plan").
    pub name: String,
    /// When the model should use this agent type (shown in AgentTool description).
    pub when_to_use: String,
    /// System prompt for this agent type.
    pub system_prompt: String,
    /// Allowed tool names. `None` or `["*"]` = all tools.
    pub tools: Option<Vec<String>>,
    /// Tool names to exclude (applied after `tools` filter).
    pub disallowed_tools: Option<Vec<String>>,
    /// Model override (None = inherit from parent).
    pub model: Option<String>,
    /// Maximum agentic turns before stopping.
    pub max_turns: Option<usize>,
    /// Source of this definition.
    pub source: AgentSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentSource {
    BuiltIn,
    User,
}

/// Returns the built-in agent definitions.
pub fn built_in_agents() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            name: "general-purpose".into(),
            when_to_use: "General-purpose agent for researching complex questions, \
                searching for code, and executing multi-step tasks. When you are searching \
                for a keyword or file and are not confident that you will find the right \
                match in the first few tries use this agent to perform the search for you."
                .into(),
            system_prompt: "You are a focused sub-agent. Complete the task given to you \
                using the available tools. Complete the task fully — don't gold-plate, \
                but don't leave it half-done. When done, respond with a concise report \
                covering what was done and any key findings."
                .into(),
            tools: None, // all tools
            disallowed_tools: None,
            model: None,
            max_turns: None,
            source: AgentSource::BuiltIn,
        },
        AgentDefinition {
            name: "Explore".into(),
            when_to_use: "Fast agent specialized for exploring codebases. Use this when \
                you need to quickly find files by patterns (eg. \"src/components/**/*.tsx\"), \
                search code for keywords (eg. \"API endpoints\"), or answer questions about \
                the codebase (eg. \"how do API endpoints work?\"). When calling this agent, \
                specify the desired thoroughness level: \"quick\" for basic searches, \
                \"medium\" for moderate exploration, or \"very thorough\" for comprehensive \
                analysis across multiple locations and naming conventions."
                .into(),
            system_prompt: "You are a file search specialist. You excel at thoroughly \
                navigating and exploring codebases.\n\n\
                === CRITICAL: READ-ONLY MODE - NO FILE MODIFICATIONS ===\n\
                This is a READ-ONLY exploration task. You are STRICTLY PROHIBITED from \
                creating, modifying, or deleting files. You do NOT have access to file \
                editing tools.\n\n\
                Your strengths:\n\
                - Rapidly finding files using glob patterns\n\
                - Searching code and text with powerful regex patterns\n\
                - Reading and analyzing file contents\n\n\
                Guidelines:\n\
                - Use glob for broad file pattern matching\n\
                - Use grep for searching file contents with regex\n\
                - Use file_read when you know the specific file path\n\
                - Use shell ONLY for read-only operations (ls, git log, git diff, find)\n\
                - Spawn multiple parallel tool calls for efficiency\n\n\
                Complete the search request efficiently and report findings clearly."
                .into(),
            tools: None,
            disallowed_tools: Some(vec![
                "agent".into(),
                "file_edit".into(),
                "file_write".into(),
            ]),
            model: None,
            max_turns: None,
            source: AgentSource::BuiltIn,
        },
        AgentDefinition {
            name: "Plan".into(),
            when_to_use: "Software architect agent for designing implementation plans. \
                Use this when you need to plan the implementation strategy for a task. \
                Returns step-by-step plans, identifies critical files, and considers \
                architectural trade-offs."
                .into(),
            system_prompt: "You are a software architect and planning specialist. Your \
                role is to explore the codebase and design implementation plans.\n\n\
                === CRITICAL: READ-ONLY MODE - NO FILE MODIFICATIONS ===\n\
                This is a READ-ONLY planning task. You are STRICTLY PROHIBITED from \
                creating, modifying, or deleting files.\n\n\
                ## Your Process\n\
                1. **Understand Requirements**: Focus on the requirements provided.\n\
                2. **Explore Thoroughly**: Find existing patterns and conventions. \
                   Understand the current architecture. Identify similar features.\n\
                3. **Design Solution**: Create implementation approach. Consider \
                   trade-offs and architectural decisions.\n\
                4. **Detail the Plan**: Provide step-by-step implementation strategy. \
                   Identify dependencies and sequencing.\n\n\
                ## Required Output\n\
                End your response with:\n\
                ### Critical Files for Implementation\n\
                List 3-5 files most critical for implementing this plan."
                .into(),
            tools: None,
            disallowed_tools: Some(vec![
                "agent".into(),
                "file_edit".into(),
                "file_write".into(),
            ]),
            model: None,
            max_turns: None,
            source: AgentSource::BuiltIn,
        },
    ]
}

/// Load agent definitions from markdown files in a directory.
///
/// Each file should have YAML frontmatter with fields:
/// - `name` (required): agent type name
/// - `description` (required): when to use this agent
/// - `tools`: comma-separated list of allowed tool names, or "*" for all
/// - `disallowedTools`: comma-separated list of tools to exclude
/// - `model`: model override or "inherit"
/// - `maxTurns`: maximum agentic turns
///
/// The markdown body becomes the system prompt.
pub fn load_agents_dir(dir: &Path) -> Vec<AgentDefinition> {
    let mut agents = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return agents, // directory doesn't exist, that's fine
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        match parse_agent_markdown(&path) {
            Ok(def) => {
                info!(name = %def.name, path = ?path, "loaded agent definition");
                agents.push(def);
            }
            Err(e) => {
                warn!(path = ?path, error = %e, "failed to parse agent definition");
            }
        }
    }

    agents
}

fn parse_agent_markdown(path: &Path) -> Result<AgentDefinition, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;

    // Split frontmatter from body
    let (frontmatter, body) = split_frontmatter(&content)
        .ok_or_else(|| "missing YAML frontmatter (expected --- delimiters)".to_string())?;

    // Parse frontmatter fields
    let name = extract_field(&frontmatter, "name")
        .ok_or_else(|| "missing required 'name' field in frontmatter".to_string())?;

    let description = extract_field(&frontmatter, "description")
        .ok_or_else(|| "missing required 'description' field in frontmatter".to_string())?;

    let tools = extract_field(&frontmatter, "tools").map(|s| parse_list(&s));
    let disallowed_tools = extract_field(&frontmatter, "disallowedTools").map(|s| parse_list(&s));
    let model = extract_field(&frontmatter, "model")
        .map(|m| if m == "inherit" { None } else { Some(m) })
        .unwrap_or(None);
    let max_turns = extract_field(&frontmatter, "maxTurns").and_then(|s| s.parse::<usize>().ok());

    Ok(AgentDefinition {
        name,
        when_to_use: description,
        system_prompt: body.trim().to_string(),
        tools,
        disallowed_tools,
        model,
        max_turns,
        source: AgentSource::User,
    })
}

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

fn extract_field(frontmatter: &str, key: &str) -> Option<String> {
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(value) = rest.strip_prefix(':') {
                let value = value.trim();
                // Strip surrounding quotes
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

fn parse_list(s: &str) -> Vec<String> {
    if s == "*" {
        return vec!["*".into()];
    }
    s.split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Get all agent definitions: built-in + user-defined from a directory.
pub fn get_all_agents(agents_dir: Option<&Path>) -> Vec<AgentDefinition> {
    let mut all = built_in_agents();

    if let Some(dir) = agents_dir {
        let user_agents = load_agents_dir(dir);
        // User agents override built-ins with the same name
        for user_agent in user_agents {
            if let Some(pos) = all.iter().position(|a| a.name == user_agent.name) {
                all[pos] = user_agent;
            } else {
                all.push(user_agent);
            }
        }
    }

    all
}

/// Look up an agent definition by name. Falls back to general-purpose.
pub fn find_agent(agents: &[AgentDefinition], name: &str) -> AgentDefinition {
    agents
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(name))
        .cloned()
        .unwrap_or_else(|| {
            agents
                .iter()
                .find(|a| a.name == "general-purpose")
                .cloned()
                .unwrap_or_else(|| built_in_agents().into_iter().next().unwrap())
        })
}

/// Build a filtered ToolRegistry based on an agent definition's tool constraints.
pub fn build_agent_registry(
    parent_registry: &ToolRegistry,
    definition: &AgentDefinition,
) -> Arc<ToolRegistry> {
    let child = Arc::new(ToolRegistry::new());

    let has_wildcard = definition
        .tools
        .as_ref()
        .map(|t| t.is_empty() || (t.len() == 1 && t[0] == "*"))
        .unwrap_or(true);

    let disallowed: std::collections::HashSet<String> = definition
        .disallowed_tools
        .as_ref()
        .map(|d| d.iter().cloned().collect())
        .unwrap_or_default();

    let tool_defs = parent_registry.tool_definitions();

    for def in &tool_defs {
        // Skip disallowed tools
        if disallowed.contains(&def.name) {
            continue;
        }

        // If not wildcard, only include explicitly listed tools
        if !has_wildcard {
            let allowed = definition.tools.as_ref().unwrap();
            if !allowed.iter().any(|a| a == &def.name) {
                continue;
            }
        }

        // Get the actual tool from the parent registry and re-register
        if let Some(tool_arc) = parent_registry.get(&def.name) {
            child.register_arc(tool_arc);
        }
    }

    child
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn built_in_agents_exist() {
        let agents = built_in_agents();
        assert!(agents.len() >= 3);
        assert!(agents.iter().any(|a| a.name == "general-purpose"));
        assert!(agents.iter().any(|a| a.name == "Explore"));
        assert!(agents.iter().any(|a| a.name == "Plan"));
    }

    #[test]
    fn find_agent_by_name() {
        let agents = built_in_agents();
        let explore = find_agent(&agents, "Explore");
        assert_eq!(explore.name, "Explore");
    }

    #[test]
    fn find_agent_case_insensitive() {
        let agents = built_in_agents();
        let explore = find_agent(&agents, "explore");
        assert_eq!(explore.name, "Explore");
    }

    #[test]
    fn find_agent_unknown_falls_back() {
        let agents = built_in_agents();
        let fallback = find_agent(&agents, "nonexistent");
        assert_eq!(fallback.name, "general-purpose");
    }

    #[test]
    fn parse_agent_markdown_file() {
        let dir = TempDir::new().unwrap();
        let agent_file = dir.path().join("reviewer.md");
        std::fs::write(
            &agent_file,
            r#"---
name: reviewer
description: Code review agent that checks for quality issues
tools: file_read, grep, glob
maxTurns: 15
---

You are a code review specialist. Review the code for:
- Bug risks
- Performance issues
- Style violations
"#,
        )
        .unwrap();

        let agents = load_agents_dir(dir.path());
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        assert_eq!(agent.name, "reviewer");
        assert_eq!(
            agent.when_to_use,
            "Code review agent that checks for quality issues"
        );
        assert!(agent.system_prompt.contains("code review specialist"));
        assert_eq!(
            agent.tools.as_ref().unwrap(),
            &["file_read", "grep", "glob"]
        );
        assert_eq!(agent.max_turns, Some(15));
        assert_eq!(agent.source, AgentSource::User);
    }

    #[test]
    fn parse_agent_with_disallowed_tools() {
        let dir = TempDir::new().unwrap();
        let agent_file = dir.path().join("safe.md");
        std::fs::write(
            &agent_file,
            r#"---
name: safe-explorer
description: Read-only exploration
disallowedTools: file_edit, file_write, shell
---

Explore only. Do not modify files.
"#,
        )
        .unwrap();

        let agents = load_agents_dir(dir.path());
        assert_eq!(agents.len(), 1);
        assert_eq!(
            agents[0].disallowed_tools.as_ref().unwrap(),
            &["file_edit", "file_write", "shell"]
        );
    }

    #[test]
    fn user_agents_override_builtin() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("explore.md"),
            r#"---
name: Explore
description: Custom explore agent
---

My custom explore prompt.
"#,
        )
        .unwrap();

        let all = get_all_agents(Some(dir.path()));
        let explore = find_agent(&all, "Explore");
        assert_eq!(explore.source, AgentSource::User);
        assert!(explore.system_prompt.contains("custom explore"));
    }

    #[test]
    fn load_empty_dir() {
        let dir = TempDir::new().unwrap();
        let agents = load_agents_dir(dir.path());
        assert!(agents.is_empty());
    }

    #[test]
    fn load_nonexistent_dir() {
        let agents = load_agents_dir(Path::new("/nonexistent/path"));
        assert!(agents.is_empty());
    }

    #[test]
    fn split_frontmatter_works() {
        let (fm, body) = split_frontmatter("---\nname: test\n---\nBody here").unwrap();
        assert_eq!(fm, "name: test");
        assert!(body.contains("Body here"));
    }

    #[test]
    fn split_frontmatter_no_delimiters() {
        assert!(split_frontmatter("Just plain text").is_none());
    }

    #[test]
    fn extract_field_quoted() {
        assert_eq!(
            extract_field("name: \"hello world\"", "name"),
            Some("hello world".into())
        );
    }

    #[test]
    fn extract_field_unquoted() {
        assert_eq!(extract_field("maxTurns: 15", "maxTurns"), Some("15".into()));
    }

    #[test]
    fn parse_list_wildcard() {
        assert_eq!(parse_list("*"), vec!["*"]);
    }

    #[test]
    fn parse_list_comma_separated() {
        assert_eq!(
            parse_list("file_read, grep, glob"),
            vec!["file_read", "grep", "glob"]
        );
    }
}

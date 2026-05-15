//! Session-scoped allowlist for the approval system.
//!
//! The allowlist is keyed by `(tool_name, ArgShape)`. `ArgShape` is a
//! deliberately coarse normalisation of the tool's arguments — the
//! point is that "Allow for session" on `file_write` to
//! `~/Desktop/project/foo.txt` grants permission for that *directory
//! shape*, not "any file_write call regardless of target". This is
//! the difference between the old `SessionGrants` (tool-name only)
//! and what T-0276 promises.
//!
//! Persistent (cross-session) allowlists are explicitly out of scope
//! here — see T-0276's "Out of scope" note. Everything in this
//! module lives in memory.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Normalised, hashable shape derived from a tool's arguments. Two
/// calls with the same `(tool_name, ArgShape)` are treated as the
/// "same operation" for approval purposes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArgShape(pub String);

impl ArgShape {
    /// Build the shape for a tool call. `raw_input` is the JSON-
    /// serialised arguments string the permission checker already
    /// passes around (`tool_input: &str`).
    ///
    /// Per-tool extraction:
    /// - **shell** — the literal `command` string. Shell commands
    ///   vary so much that any further normalisation would either
    ///   undercount (collapsing `rm` and `cat`) or overcount
    ///   (parsing argv shape). The exact command is the right grain.
    /// - **file_write / file_edit** — the *parent directory* of the
    ///   target path, with the user's home prefix folded to `~`. A
    ///   single approval covers other files in the same directory.
    /// - **safe_env** — the variable name. Approval for `OPENAI_API_KEY`
    ///   doesn't auto-approve `AWS_SECRET_ACCESS_KEY`.
    /// - **Anything else** — the literal `tool_name`. Approving an
    ///   unrecognised tool grants for any input on that tool.
    pub fn for_tool(tool_name: &str, raw_input: &str) -> Self {
        let parsed: Option<Value> = serde_json::from_str(raw_input).ok();
        let shape = match (tool_name, parsed.as_ref()) {
            ("shell" | "Bash", Some(v)) => shell_shape(v),
            ("file_write" | "file_edit" | "FileWrite" | "FileEdit", Some(v)) => file_shape(v),
            ("safe_env", Some(v)) => env_shape(v),
            _ => format!("{tool_name}:*"),
        };
        Self(shape)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn shell_shape(v: &Value) -> String {
    let cmd = v
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    format!("shell:{cmd}")
}

fn file_shape(v: &Value) -> String {
    let path = v
        .get("path")
        .and_then(|v| v.as_str())
        .or_else(|| v.get("file_path").and_then(|v| v.as_str()))
        .unwrap_or("");
    let parent = Path::new(path)
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let normalised = fold_home(&parent);
    format!("file:{normalised}")
}

fn env_shape(v: &Value) -> String {
    let name = v.get("name").and_then(|v| v.as_str()).unwrap_or("");
    format!("env:{name}")
}

fn fold_home(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        let home_s = home.display().to_string();
        if !home_s.is_empty() && path.starts_with(&home_s) {
            return path.replacen(&home_s, "~", 1);
        }
    }
    path.to_string()
}

/// Per-session set of `(tool_name, ArgShape)` grants. "Allow for
/// session" populates this; `is_granted` consults it. Cleared on
/// session end.
#[derive(Debug, Default)]
pub struct SessionAllowlist {
    entries: HashSet<(String, ArgShape)>,
}

impl SessionAllowlist {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a `(tool, shape)` to the allowlist. Idempotent.
    pub fn grant(&mut self, tool_name: impl Into<String>, shape: ArgShape) {
        self.entries.insert((tool_name.into(), shape));
    }

    /// Test whether a tool call is on the allowlist. The match is
    /// exact on `(tool_name, ArgShape)`.
    pub fn is_granted(&self, tool_name: &str, shape: &ArgShape) -> bool {
        // Cheap key reconstruction — HashSet doesn't borrow well
        // across composite keys without unstable APIs.
        self.entries
            .iter()
            .any(|(t, s)| t == tool_name && s == shape)
    }

    /// Drop every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate the entries — used by the audit/diagnostics path.
    pub fn entries(&self) -> impl Iterator<Item = &(String, ArgShape)> {
        self.entries.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_shape_is_command_verbatim() {
        let shape = ArgShape::for_tool("shell", r#"{"command":"git status"}"#);
        assert_eq!(shape.as_str(), "shell:git status");
    }

    #[test]
    fn shell_shape_distinguishes_distinct_commands() {
        let a = ArgShape::for_tool("shell", r#"{"command":"ls"}"#);
        let b = ArgShape::for_tool("shell", r#"{"command":"rm -rf /"}"#);
        assert_ne!(a, b);
    }

    #[test]
    fn file_shape_collapses_files_in_same_dir() {
        let a = ArgShape::for_tool(
            "file_write",
            r#"{"path":"/tmp/proj/foo.rs","content":"x"}"#,
        );
        let b = ArgShape::for_tool(
            "file_write",
            r#"{"path":"/tmp/proj/bar.rs","content":"y"}"#,
        );
        assert_eq!(a, b);
    }

    #[test]
    fn file_shape_distinguishes_different_dirs() {
        let a = ArgShape::for_tool("file_write", r#"{"path":"/tmp/a/foo"}"#);
        let b = ArgShape::for_tool("file_write", r#"{"path":"/tmp/b/foo"}"#);
        assert_ne!(a, b);
    }

    #[test]
    fn file_shape_handles_file_path_alias() {
        let a = ArgShape::for_tool("file_write", r#"{"path":"/x/y/z"}"#);
        let b = ArgShape::for_tool("file_write", r#"{"file_path":"/x/y/z"}"#);
        assert_eq!(a, b);
    }

    #[test]
    fn env_shape_keyed_by_name() {
        let a = ArgShape::for_tool("safe_env", r#"{"name":"OPENAI_API_KEY"}"#);
        let b = ArgShape::for_tool("safe_env", r#"{"name":"AWS_SECRET_ACCESS_KEY"}"#);
        assert_ne!(a, b);
    }

    #[test]
    fn unknown_tool_collapses_to_any_input() {
        let a = ArgShape::for_tool("Read", r#"{"path":"/a"}"#);
        let b = ArgShape::for_tool("Read", r#"{"path":"/b"}"#);
        assert_eq!(a, b);
        assert_eq!(a.as_str(), "Read:*");
    }

    #[test]
    fn malformed_json_falls_back_to_wildcard() {
        let a = ArgShape::for_tool("shell", "not json");
        assert_eq!(a.as_str(), "shell:*");
    }

    #[test]
    fn allowlist_grant_and_check() {
        let mut allow = SessionAllowlist::new();
        let shape = ArgShape::for_tool("shell", r#"{"command":"ls"}"#);
        assert!(!allow.is_granted("shell", &shape));
        allow.grant("shell", shape.clone());
        assert!(allow.is_granted("shell", &shape));
    }

    #[test]
    fn allowlist_grant_is_specific_to_shape() {
        let mut allow = SessionAllowlist::new();
        let safe_shape = ArgShape::for_tool("shell", r#"{"command":"ls"}"#);
        let dangerous_shape = ArgShape::for_tool("shell", r#"{"command":"rm -rf /"}"#);
        allow.grant("shell", safe_shape.clone());
        assert!(allow.is_granted("shell", &safe_shape));
        assert!(!allow.is_granted("shell", &dangerous_shape));
    }

    #[test]
    fn allowlist_clear_drops_entries() {
        let mut allow = SessionAllowlist::new();
        let s = ArgShape::for_tool("shell", r#"{"command":"ls"}"#);
        allow.grant("shell", s.clone());
        allow.clear();
        assert!(!allow.is_granted("shell", &s));
        assert!(allow.is_empty());
    }
}

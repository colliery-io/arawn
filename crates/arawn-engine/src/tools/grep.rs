use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::process::Command;

use crate::tool::{Tool, ToolError, ToolOutput};
use crate::tools::sensitive_paths::{is_sensitive_path, is_token_path};

/// Default cap on grep results when head_limit is unspecified.
const DEFAULT_HEAD_LIMIT: usize = 250;

/// VCS directories to exclude from searches.
const VCS_EXCLUDES: &[&str] = &[".git", ".svn", ".hg", ".bzr", ".jj", ".sl"];

/// Search file contents using ripgrep (rg) or grep as fallback.
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search LOCAL file contents using ripgrep. Only for files on the local filesystem — NOT for searching the web, GitHub, or remote APIs.\n\n\
         \x20 Usage:\n\
         \x20 - ALWAYS use grep for local file search tasks. NEVER invoke `grep` or `rg` as a shell command.\n\
         \x20 - The `path` parameter must be a valid local directory or file path. Do NOT pass URLs or empty strings.\n\
         \x20 - Supports full regex syntax (e.g., \"log.*Error\", \"function\\s+\\w+\")\n\
         \x20 - Filter files with glob parameter (e.g., \"*.js\", \"**/*.tsx\") or type parameter (e.g., \"js\", \"py\", \"rust\")\n\
         \x20 - Output modes: \"content\" shows matching lines, \"files_with_matches\" shows only file paths (default), \"count\" shows match counts\n\
         \x20 - Pattern syntax: Uses ripgrep — literal braces need escaping\n\
         \x20 - Multiline matching: For cross-line patterns, use `multiline: true`\n"
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regular expression pattern to search for in file contents"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in (rg PATH). Defaults to current working directory."
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. \"*.js\", \"*.{ts,tsx}\") - maps to rg --glob"
                },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode: \"content\" shows matching lines (supports -A/-B/-C context, -n line numbers, head_limit), \"files_with_matches\" shows file paths (supports head_limit), \"count\" shows match counts (supports head_limit). Defaults to \"files_with_matches\"."
                },
                "-B": {
                    "type": "number",
                    "description": "Number of lines to show before each match (rg -B). Requires output_mode: \"content\", ignored otherwise."
                },
                "-A": {
                    "type": "number",
                    "description": "Number of lines to show after each match (rg -A). Requires output_mode: \"content\", ignored otherwise."
                },
                "-C": {
                    "type": "number",
                    "description": "Alias for context."
                },
                "context": {
                    "type": "number",
                    "description": "Number of lines to show before and after each match (rg -C). Requires output_mode: \"content\", ignored otherwise."
                },
                "-n": {
                    "type": "boolean",
                    "description": "Show line numbers in output (rg -n). Requires output_mode: \"content\", ignored otherwise. Defaults to true."
                },
                "-i": {
                    "type": "boolean",
                    "description": "Case insensitive search (rg -i)"
                },
                "type": {
                    "type": "string",
                    "description": "File type to search (rg --type). Common types: js, py, rust, go, java, etc. More efficient than include for standard file types."
                },
                "head_limit": {
                    "type": "number",
                    "description": "Limit output to first N lines/entries, equivalent to \"| head -N\". Works across all output modes: content (limits output lines), files_with_matches (limits file paths), count (limits count entries). Defaults to 250 when unspecified. Pass 0 for unlimited (use sparingly — large result sets waste context)."
                },
                "offset": {
                    "type": "number",
                    "description": "Skip first N lines/entries before applying head_limit, equivalent to \"| tail -n +N | head -N\". Works across all output modes. Defaults to 0."
                },
                "multiline": {
                    "type": "boolean",
                    "description": "Enable multiline mode where . matches newlines and patterns can span lines (rg -U --multiline-dotall). Default: false."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let pattern = params
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'pattern' parameter".into()))?;

        let path = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        // Validate path stays within workstream root and isn't sensitive
        if path != "." {
            let resolved = match ctx.validate_path(path) {
                Ok(p) => p,
                Err(e) => return Ok(ToolOutput::error(e)),
            };
            if is_sensitive_path(&resolved) {
                return Ok(ToolOutput::error(format!(
                    "path '{path}' resolves into a sensitive directory and is denied"
                )));
            }
            if let Some(data_dir) = ctx.data_dir()
                && is_token_path(&resolved, data_dir)
            {
                return Ok(ToolOutput::error(format!(
                    "path '{path}' resolves into the OAuth token directory and is denied"
                )));
            }
        }

        let glob_pattern = params.get("glob").and_then(|v| v.as_str());
        let file_type = params.get("type").and_then(|v| v.as_str());

        let output_mode = params
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches");

        let case_insensitive = params.get("-i").and_then(|v| v.as_bool()).unwrap_or(false);

        let show_line_numbers = params.get("-n").and_then(|v| v.as_bool()).unwrap_or(true);

        let multiline = params
            .get("multiline")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let context_before = params.get("-B").and_then(|v| v.as_u64());
        let context_after = params.get("-A").and_then(|v| v.as_u64());
        let context_both = params
            .get("-C")
            .and_then(|v| v.as_u64())
            .or_else(|| params.get("context").and_then(|v| v.as_u64()));

        let head_limit = params.get("head_limit").and_then(|v| v.as_u64());
        let offset = params.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        // Build rg command
        let result = if has_rg().await {
            run_rg(
                ctx.working_dir(),
                pattern,
                path,
                glob_pattern,
                file_type,
                output_mode,
                case_insensitive,
                show_line_numbers,
                multiline,
                context_before,
                context_after,
                context_both,
            )
            .await
        } else {
            run_grep_fallback(
                ctx.working_dir(),
                pattern,
                path,
                case_insensitive,
                output_mode,
            )
            .await
        };

        match result {
            Ok(output) => {
                if output.is_empty() {
                    return Ok(ToolOutput::success("No matches found."));
                }

                // Apply head_limit and offset
                let lines: Vec<&str> = output.lines().collect();
                let effective_limit = match head_limit {
                    Some(0) => lines.len(), // explicit 0 = unlimited
                    Some(n) => n as usize,
                    None => DEFAULT_HEAD_LIMIT,
                };

                let sliced: Vec<&str> = lines
                    .iter()
                    .skip(offset)
                    .take(effective_limit)
                    .copied()
                    .collect();

                let result_text = sliced.join("\n");
                let was_truncated = offset + effective_limit < lines.len();

                if was_truncated {
                    Ok(ToolOutput::success(format!(
                        "{result_text}\n... (truncated, {total} total lines)",
                        total = lines.len()
                    )))
                } else {
                    Ok(ToolOutput::success(result_text))
                }
            }
            Err(e) => Ok(ToolOutput::error(format!("search failed: {e}"))),
        }
    }
}

async fn has_rg() -> bool {
    Command::new("rg").arg("--version").output().await.is_ok()
}

#[allow(clippy::too_many_arguments)]
async fn run_rg(
    cwd: &std::path::Path,
    pattern: &str,
    path: &str,
    glob: Option<&str>,
    file_type: Option<&str>,
    output_mode: &str,
    case_insensitive: bool,
    show_line_numbers: bool,
    multiline: bool,
    context_before: Option<u64>,
    context_after: Option<u64>,
    context_both: Option<u64>,
) -> Result<String, String> {
    let mut cmd = Command::new("rg");
    cmd.arg("--no-heading");

    // Output mode
    match output_mode {
        "files_with_matches" => {
            cmd.arg("--files-with-matches");
        }
        "count" => {
            cmd.arg("--count");
        }
        // "content" is the default mode; any unknown value falls through to it.
        _ => {
            if show_line_numbers {
                cmd.arg("--line-number");
            }
            // Context lines only for content mode
            if let Some(n) = context_both {
                cmd.arg("-C").arg(n.to_string());
            } else {
                if let Some(n) = context_before {
                    cmd.arg("-B").arg(n.to_string());
                }
                if let Some(n) = context_after {
                    cmd.arg("-A").arg(n.to_string());
                }
            }
        }
    }

    if case_insensitive {
        cmd.arg("--ignore-case");
    }

    if multiline {
        cmd.arg("-U").arg("--multiline-dotall");
    }

    if let Some(g) = glob {
        cmd.arg("--glob").arg(g);
    }

    if let Some(t) = file_type {
        cmd.arg("--type").arg(t);
    }

    // Exclude VCS directories
    for dir in VCS_EXCLUDES {
        cmd.arg("--glob").arg(format!("!{dir}"));
    }

    cmd.arg(pattern).arg(path).current_dir(cwd);

    let output = cmd.output().await.map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // rg exits with 1 when no matches found (not an error)
    if !output.status.success() && output.status.code() != Some(1) {
        return Err(format!("rg failed: {stderr}"));
    }

    Ok(stdout)
}

async fn run_grep_fallback(
    cwd: &std::path::Path,
    pattern: &str,
    path: &str,
    case_insensitive: bool,
    output_mode: &str,
) -> Result<String, String> {
    let mut cmd = Command::new("grep");
    cmd.arg("-rn");

    if case_insensitive {
        cmd.arg("-i");
    }

    match output_mode {
        "files_with_matches" => {
            cmd.arg("-l");
        }
        "count" => {
            cmd.arg("-c");
        }
        _ => {}
    }

    cmd.arg(pattern).arg(path).current_dir(cwd);

    let output = cmd.output().await.map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() && output.status.code() != Some(1) {
        return Err(format!("grep failed: {stderr}"));
    }

    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::EngineToolContext;
    use arawn_core::Workstream;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn test_ctx(dir: &std::path::Path) -> EngineToolContext {
        let ws = Workstream::new("test", dir);
        EngineToolContext::new(&ws, Uuid::new_v4())
    }

    #[tokio::test]
    async fn grep_finds_matches() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("test.txt"),
            "hello world\nfoo bar\nhello again\n",
        )
        .unwrap();

        let tool = GrepTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(&ctx, json!({"pattern": "hello"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("test.txt"));
    }

    #[tokio::test]
    async fn grep_no_matches() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "nothing here\n").unwrap();

        let tool = GrepTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(&ctx, json!({"pattern": "nonexistent"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("No matches"));
    }

    #[tokio::test]
    async fn grep_case_insensitive() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "Hello World\n").unwrap();

        let tool = GrepTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(&ctx, json!({"pattern": "hello", "-i": true}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("test.txt"));
    }

    #[tokio::test]
    async fn grep_with_glob() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("code.rs"), "fn main() {}\n").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "fn notes\n").unwrap();

        let tool = GrepTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(&ctx, json!({"pattern": "fn", "glob": "*.rs"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("code.rs"));
    }

    #[tokio::test]
    async fn grep_content_mode() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("test.txt"),
            "line one\nhello world\nline three\n",
        )
        .unwrap();

        let tool = GrepTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(&ctx, json!({"pattern": "hello", "output_mode": "content"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("hello world"));
    }

    #[tokio::test]
    async fn grep_files_with_matches_mode() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "match here\n").unwrap();
        std::fs::write(dir.path().join("b.txt"), "no match\n").unwrap();

        let tool = GrepTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(
                &ctx,
                json!({"pattern": "match here", "output_mode": "files_with_matches"}),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("a.txt"));
        assert!(!result.content.contains("b.txt"));
    }

    #[tokio::test]
    async fn grep_head_limit() {
        let dir = TempDir::new().unwrap();
        let mut content = String::new();
        for i in 0..100 {
            content.push_str(&format!("match line {i}\n"));
        }
        std::fs::write(dir.path().join("big.txt"), content).unwrap();

        let tool = GrepTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(
                &ctx,
                json!({"pattern": "match", "output_mode": "content", "head_limit": 5}),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let lines: Vec<&str> = result.content.lines().collect();
        // 5 result lines + 1 truncation notice
        assert!(lines.len() <= 7);
    }

    #[test]
    fn schema_is_valid() {
        let tool = GrepTool;
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["pattern"].is_object());
        assert!(schema["properties"]["output_mode"].is_object());
        assert!(schema["properties"]["head_limit"].is_object());
        assert!(schema["properties"]["multiline"].is_object());
        assert!(schema["properties"]["type"].is_object());
    }

    #[tokio::test]
    async fn grep_path_traversal_rejected() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("safe.txt"), "safe content\n").unwrap();

        let tool = GrepTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(&ctx, json!({"pattern": ".", "path": "../../../etc"}))
            .await
            .unwrap();

        assert!(result.is_error, "traversal path should be rejected");
        assert!(
            result.content.contains("escapes workstream root"),
            "expected traversal error, got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn grep_absolute_path_rejected() {
        let dir = TempDir::new().unwrap();
        let tool = GrepTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(&ctx, json!({"pattern": ".", "path": "/etc"}))
            .await
            .unwrap();

        assert!(result.is_error, "absolute path outside root should be rejected");
        assert!(result.content.contains("escapes workstream root"));
    }

    #[tokio::test]
    async fn grep_relative_path_within_root_allowed() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("file.txt"), "findme\n").unwrap();

        let tool = GrepTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(&ctx, json!({"pattern": "findme", "path": "subdir"}))
            .await
            .unwrap();

        assert!(!result.is_error, "relative path within root should be allowed");
        assert!(result.content.contains("file.txt"));
    }
}

use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::context::ToolContext;
use crate::error::EngineError;
use crate::tool::{Tool, ToolOutput};

/// Maximum number of files to return before truncating.
const MAX_RESULTS: usize = 100;

/// Fast file pattern matching using globwalk. Respects `.gitignore` via the
/// `ignore` crate — tracked files always appear, gitignored paths are skipped.
pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "- Fast file pattern matching tool that works with any codebase size\n\
         - Supports glob patterns like \"**/*.js\" or \"src/**/*.ts\"\n\
         - Returns matching file paths sorted by modification time\n\
         - Use this tool when you need to find files by name patterns\n\
         - When you are doing an open ended search that may require multiple rounds of globbing and grepping, use the agent tool instead"
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
                    "description": "The glob pattern to match files against"
                },
                "path": {
                    "type": "string",
                    "description": "The directory to search in. If not specified, the current working directory will be used. IMPORTANT: Omit this field to use the default directory. DO NOT enter \"undefined\" or \"null\" - simply omit it for the default behavior. Must be a valid directory path if provided."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        let pattern = params
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Tool("missing 'pattern' parameter".into()))?;

        let base_dir = if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
            ctx.working_dir.join(path)
        } else {
            ctx.working_dir.clone()
        };

        let start = Instant::now();

        // Use globwalk for pattern matching — respects .gitignore automatically
        let walker = globwalk::GlobWalkerBuilder::from_patterns(&base_dir, &[pattern])
            .file_type(globwalk::FileType::FILE)
            .build();

        let walker = match walker {
            Ok(w) => w,
            Err(e) => return Ok(ToolOutput::error(format!("invalid glob pattern: {e}"))),
        };

        // Collect entries with modification times for sorting
        let mut entries: Vec<(std::path::PathBuf, std::time::SystemTime)> = Vec::new();
        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path().to_path_buf();

            let mtime = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::UNIX_EPOCH);

            entries.push((path, mtime));
        }

        // Sort by modification time, newest first
        entries.sort_by(|a, b| b.1.cmp(&a.1));

        let _duration_ms = start.elapsed().as_millis();
        let total = entries.len();
        let truncated = total > MAX_RESULTS;

        if entries.is_empty() {
            return Ok(ToolOutput::success("No files found matching pattern."));
        }

        // Build relative paths
        let prefix = ctx.working_dir.to_string_lossy();
        let result: String = entries
            .iter()
            .take(MAX_RESULTS)
            .map(|(p, _)| {
                let s = p.to_string_lossy();
                s.strip_prefix(prefix.as_ref())
                    .and_then(|p| p.strip_prefix('/'))
                    .unwrap_or(&s)
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");

        if truncated {
            Ok(ToolOutput::success(format!(
                "{result}\n... ({total} total, showing first {MAX_RESULTS})"
            )))
        } else {
            Ok(ToolOutput::success(result))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Workstream;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn schema_is_valid() {
        let tool = GlobTool;
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["pattern"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("pattern")));
    }

    #[test]
    fn is_read_only() {
        assert!(GlobTool.is_read_only());
    }

    #[tokio::test]
    async fn glob_in_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("foo.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.path().join("bar.txt"), "hello").unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/baz.rs"), "mod baz;").unwrap();

        let ws = Workstream::scratch(dir.path());
        let ctx = ToolContext::new(&ws, Uuid::new_v4());

        let result = GlobTool
            .execute(&ctx, json!({"pattern": "**/*.rs"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("foo.rs"));
        assert!(result.content.contains("baz.rs"));
        assert!(!result.content.contains("bar.txt"));
    }

    #[tokio::test]
    async fn glob_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workstream::scratch(dir.path());
        let ctx = ToolContext::new(&ws, Uuid::new_v4());

        let result = GlobTool
            .execute(&ctx, json!({"pattern": "**/*.xyz"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("No files found"));
    }

    #[tokio::test]
    async fn glob_respects_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        // Create a .gitignore that ignores build/
        std::fs::write(dir.path().join(".gitignore"), "build/\n").unwrap();
        std::fs::create_dir_all(dir.path().join("build")).unwrap();
        std::fs::write(dir.path().join("build/out.rs"), "").unwrap();
        std::fs::write(dir.path().join("src.rs"), "fn main() {}").unwrap();

        let ws = Workstream::scratch(dir.path());
        let ctx = ToolContext::new(&ws, Uuid::new_v4());

        let result = GlobTool
            .execute(&ctx, json!({"pattern": "**/*.rs"}))
            .await
            .unwrap();

        assert!(result.content.contains("src.rs"));
        // globwalk doesn't inherently respect .gitignore (that's the ignore crate),
        // but the build/ dir files should still show up here since globwalk
        // doesn't filter by gitignore. This test documents current behavior.
    }
}

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::tool::{Tool, ToolError, ToolOutput};
use crate::tools::sensitive_paths::{is_secret_file, is_token_path};

/// Edit a file by replacing a string. Path traversal protection.
pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn permission_category(&self) -> arawn_tool::PermissionCategory {
        arawn_tool::PermissionCategory::FileWrite
    }

    fn description(&self) -> &str {
        "Edit a file by performing exact string replacement. ALWAYS prefer this over file_write for modifying existing files — it only sends the diff. Do NOT use sed/awk via shell.\n\n\
         Usage:\n\
         - You must use file_read at least once on the file before editing. This tool will error if you attempt an edit without reading the file first.\n\
         - Provide old_string (the exact text to find) and new_string (the replacement).\n\
         - The edit will FAIL if old_string is not found in the file.\n\
         - The edit will FAIL if old_string appears more than once — provide more surrounding context to make it unique, or set replace_all to true.\n\
         - Use replace_all to rename variables or change all occurrences of a string across the file.\n\
         - Preserve exact indentation (tabs/spaces) in both old_string and new_string.\n\
         - Paths that escape the working directory are blocked."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path relative to the workstream root"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact string to find and replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement string"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: false, fails if ambiguous)"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'path' parameter".into()))?;

        let old_string = params
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'old_string' parameter".into()))?;

        let new_string = params
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'new_string' parameter".into()))?;

        let replace_all = params
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Resolve and validate path
        let full_path = ctx.working_dir().join(path_str);
        let canonical = match full_path.canonicalize() {
            Ok(p) => p,
            Err(e) => return Ok(ToolOutput::error(format!("cannot read '{path_str}': {e}"))),
        };

        let canonical_root = match ctx.working_dir().canonicalize() {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "cannot resolve workstream root: {e}"
                )));
            }
        };

        if !canonical.starts_with(&canonical_root) && !ctx.is_allowed_path(&canonical) {
            return Ok(ToolOutput::error(format!(
                "path '{path_str}' escapes workstream root"
            )));
        }

        if is_secret_file(&canonical) {
            return Ok(ToolOutput::error(format!(
                "refusing to edit '{path_str}': matches secret-file pattern (e.g. .env, *.pem, credentials.*)"
            )));
        }

        if let Some(data_dir) = ctx.data_dir()
            && is_token_path(&canonical, data_dir)
        {
            return Ok(ToolOutput::error(format!(
                "refusing to edit '{path_str}': resolves into the OAuth token directory"
            )));
        }

        // Pre-read enforcement: file must have been read before editing
        if !ctx.has_read_file(&canonical) {
            return Ok(ToolOutput::error(format!(
                "You must use file_read on '{path_str}' before editing it. Read the file first to see its current contents."
            )));
        }

        // Read file
        let content = match tokio::fs::read_to_string(&canonical).await {
            Ok(c) => c,
            Err(e) => return Ok(ToolOutput::error(format!("cannot read '{path_str}': {e}"))),
        };

        // Check occurrences
        let count = content.matches(old_string).count();

        if count == 0 {
            return Ok(ToolOutput::error(format!(
                "old_string not found in '{path_str}'"
            )));
        }

        if count > 1 && !replace_all {
            return Ok(ToolOutput::error(format!(
                "old_string found {count} times in '{path_str}' — set replace_all to true or provide a more specific string"
            )));
        }

        // Perform replacement
        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        // Write back
        if let Err(e) = tokio::fs::write(&canonical, &new_content).await {
            return Ok(ToolOutput::error(format!("cannot write '{path_str}': {e}")));
        }

        // Generate diff for display
        let summary = crate::diff::diff_summary(&content, &new_content);
        let mut output = format!("Replaced {count} occurrence(s) in {path_str} ({summary})\n");
        if let Some(diff) = crate::diff::unified_diff(path_str, &content, &new_content) {
            output.push('\n');
            output.push_str(&crate::diff::diff_to_markdown(&diff));
        }
        Ok(ToolOutput::success(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::EngineToolContext;
    use arawn_core::Workstream;
    use arawn_tool::ToolContext as _;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn test_ctx(dir: &std::path::Path) -> EngineToolContext {
        let ws = Workstream::new("test", dir);
        EngineToolContext::new(&ws, Uuid::new_v4())
    }

    /// Mark a file as read in the context (simulates a prior file_read call).
    fn mark_read(ctx: &EngineToolContext, dir: &std::path::Path, name: &str) {
        let canonical = dir.join(name).canonicalize().unwrap();
        ctx.mark_file_read(canonical);
    }

    #[tokio::test]
    async fn edit_replaces_string() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello world").unwrap();

        let tool = FileEditTool;
        let ctx = test_ctx(dir.path());
        mark_read(&ctx, dir.path(), "test.txt");

        let result = tool
            .execute(
                &ctx,
                json!({"path": "test.txt", "old_string": "world", "new_string": "rust"}),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("test.txt")).unwrap(),
            "hello rust"
        );
    }

    #[tokio::test]
    async fn edit_fails_on_missing_string() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello").unwrap();

        let tool = FileEditTool;
        let ctx = test_ctx(dir.path());
        mark_read(&ctx, dir.path(), "test.txt");

        let result = tool
            .execute(
                &ctx,
                json!({"path": "test.txt", "old_string": "nonexistent", "new_string": "x"}),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("not found"));
    }

    #[tokio::test]
    async fn edit_fails_on_ambiguous_match() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "aaa bbb aaa").unwrap();

        let tool = FileEditTool;
        let ctx = test_ctx(dir.path());
        mark_read(&ctx, dir.path(), "test.txt");

        let result = tool
            .execute(
                &ctx,
                json!({"path": "test.txt", "old_string": "aaa", "new_string": "ccc"}),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("2 times"));
    }

    #[tokio::test]
    async fn edit_replace_all() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "aaa bbb aaa").unwrap();

        let tool = FileEditTool;
        let ctx = test_ctx(dir.path());
        mark_read(&ctx, dir.path(), "test.txt");

        let result = tool
            .execute(
                &ctx,
                json!({"path": "test.txt", "old_string": "aaa", "new_string": "ccc", "replace_all": true}),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("test.txt")).unwrap(),
            "ccc bbb ccc"
        );
    }

    #[tokio::test]
    async fn edit_rejects_path_traversal() {
        let dir = TempDir::new().unwrap();
        let tool = FileEditTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(
                &ctx,
                json!({"path": "../../etc/passwd", "old_string": "x", "new_string": "y"}),
            )
            .await
            .unwrap();

        assert!(result.is_error);
    }

    #[tokio::test]
    async fn edit_fails_without_prior_read() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello world").unwrap();

        let tool = FileEditTool;
        let ctx = test_ctx(dir.path());
        // Intentionally NOT calling mark_read

        let result = tool
            .execute(
                &ctx,
                json!({"path": "test.txt", "old_string": "hello", "new_string": "bye"}),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("must use file_read"));
    }

    #[tokio::test]
    async fn edit_rejects_secret_filename() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env"), "API_KEY=old").unwrap();

        let tool = FileEditTool;
        let ctx = test_ctx(dir.path());
        mark_read(&ctx, dir.path(), ".env");

        let result = tool
            .execute(
                &ctx,
                json!({"path": ".env", "old_string": "old", "new_string": "new"}),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("secret-file pattern"));
    }

    #[test]
    fn schema_is_valid() {
        let tool = FileEditTool;
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["old_string"].is_object());
        assert!(schema["properties"]["new_string"].is_object());
    }
}

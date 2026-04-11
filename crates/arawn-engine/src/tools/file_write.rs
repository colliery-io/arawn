use async_trait::async_trait;
use serde_json::{Value, json};

use crate::tool::{Tool, ToolError, ToolOutput};

/// Write content to a file within the workstream's working directory.
/// Creates parent directories if needed. Path traversal protection.
pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Create or overwrite a file with the given content. Creates parent directories if needed.\n\n\
         Usage:\n\
         - If the file already exists, you MUST use file_read first. This tool will error if you attempt to overwrite a file without reading it first.\n\
         - The path is relative to the session's working directory.\n\
         - This will OVERWRITE existing files entirely. Prefer file_edit for modifying existing files — it only changes the specific part you need.\n\
         - Use this tool to create new files or when you need a complete rewrite.\n\
         - Do NOT use shell echo/cat heredoc to write files — use this tool instead.\n\
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
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'path' parameter".into()))?;

        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'content' parameter".into()))?;

        // Path traversal protection: for new files we can't canonicalize the full path,
        // so we canonicalize the root and normalize the relative path to check for .. escapes
        let canonical_root = match ctx.working_dir().canonicalize() {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "cannot resolve workstream root: {e}"
                )));
            }
        };

        let full_path = canonical_root.join(path_str);
        let normalized = normalize_path(&full_path);

        if !normalized.starts_with(&canonical_root) && !ctx.is_allowed_path(&normalized) {
            return Ok(ToolOutput::error(format!(
                "path '{path_str}' escapes workstream root"
            )));
        }

        // Pre-read enforcement: if file exists, it must have been read first
        if full_path.exists() && !ctx.has_read_file(&normalized) {
            return Ok(ToolOutput::error(format!(
                "File '{path_str}' already exists. You must use file_read on it before overwriting. Read the file first to see its current contents."
            )));
        }

        // Create parent directories
        if let Some(parent) = full_path.parent()
            && let Err(e) = tokio::fs::create_dir_all(parent).await
        {
            return Ok(ToolOutput::error(format!(
                "cannot create directories for '{path_str}': {e}"
            )));
        }

        // Read existing content for diff (if file exists)
        let old_content = if full_path.exists() {
            tokio::fs::read_to_string(&full_path).await.ok()
        } else {
            None
        };

        // Write file
        if let Err(e) = tokio::fs::write(&full_path, content).await {
            return Ok(ToolOutput::error(format!("cannot write '{path_str}': {e}")));
        }

        // Generate diff for display
        let line_count = content.lines().count();
        let mut output = match &old_content {
            Some(old) => {
                let summary = crate::diff::diff_summary(old, content);
                format!("Updated {path_str} ({line_count} lines, {summary})\n")
            }
            None => format!("Created {path_str} ({line_count} lines)\n"),
        };

        match &old_content {
            Some(old) => {
                if let Some(diff) = crate::diff::unified_diff(path_str, old, content) {
                    output.push('\n');
                    output.push_str(&crate::diff::diff_to_markdown(&diff));
                }
            }
            None => {
                // New file — show creation diff (truncated for large files)
                let diff = crate::diff::creation_diff(path_str, content, 20);
                output.push('\n');
                output.push_str(&crate::diff::diff_to_markdown(&diff));
            }
        }

        Ok(ToolOutput::success(output))
    }
}

fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
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

    fn mark_read(ctx: &EngineToolContext, path: &std::path::Path) {
        let canonical = path.canonicalize().unwrap();
        ctx.mark_file_read(canonical);
    }

    #[tokio::test]
    async fn write_creates_file() {
        let dir = TempDir::new().unwrap();
        let tool = FileWriteTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(&ctx, json!({"path": "hello.txt", "content": "world"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Created hello.txt"));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("hello.txt")).unwrap(),
            "world"
        );
    }

    #[tokio::test]
    async fn write_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let tool = FileWriteTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(
                &ctx,
                json!({"path": "deep/nested/file.txt", "content": "deep content"}),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(dir.path().join("deep/nested/file.txt").exists());
    }

    #[tokio::test]
    async fn write_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("existing.txt"), "old").unwrap();

        let tool = FileWriteTool;
        let ctx = test_ctx(dir.path());
        mark_read(&ctx, &dir.path().join("existing.txt"));

        let result = tool
            .execute(&ctx, json!({"path": "existing.txt", "content": "new"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("existing.txt")).unwrap(),
            "new"
        );
    }

    #[tokio::test]
    async fn write_rejects_path_traversal() {
        let dir = TempDir::new().unwrap();
        let tool = FileWriteTool;
        let ctx = test_ctx(dir.path());

        let result = tool
            .execute(
                &ctx,
                json!({"path": "../../etc/evil.txt", "content": "bad"}),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("escapes workstream root"));
    }

    #[tokio::test]
    async fn write_new_file_without_read_ok() {
        let dir = TempDir::new().unwrap();
        let tool = FileWriteTool;
        let ctx = test_ctx(dir.path());
        // New file — no prior read needed
        let result = tool
            .execute(&ctx, json!({"path": "brand_new.txt", "content": "fresh"}))
            .await
            .unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn write_existing_file_without_read_fails() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("existing.txt"), "old content").unwrap();

        let tool = FileWriteTool;
        let ctx = test_ctx(dir.path());
        // Intentionally NOT reading first

        let result = tool
            .execute(&ctx, json!({"path": "existing.txt", "content": "new"}))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("must use file_read"));
    }

    #[test]
    fn schema_is_valid() {
        let tool = FileWriteTool;
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["path"].is_object());
        assert!(schema["properties"]["content"].is_object());
    }
}

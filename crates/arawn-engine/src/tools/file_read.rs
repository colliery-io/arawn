use async_trait::async_trait;
use serde_json::{Value, json};

use crate::tool::{Tool, ToolError, ToolOutput};
use crate::tools::sensitive_paths::{is_secret_file, is_token_path};

/// Read a file within the workstream's working directory.
/// Rejects paths that escape the workstream root (path traversal protection).
pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. ALWAYS use this instead of cat/head/tail via shell.\n\n\
         Usage:\n\
         - The path is relative to the session's working directory.\n\
         - By default reads the entire file. Use offset and limit for large files to read specific sections.\n\
         - Results include line numbers for reference.\n\
         - Cannot read directories — use shell 'ls' for that.\n\
         - Paths that escape the working directory (e.g., '../../../etc/passwd') are blocked.\n\
         - If a file does not exist, an error is returned."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path relative to the workstream root"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (0-based)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'path' parameter".into()))?;

        let offset = params.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        // Resolve the path and check for traversal
        let full_path = ctx.working_dir().join(path_str);
        let canonical = match full_path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                let hint = if e.kind() == std::io::ErrorKind::NotFound {
                    " Use the glob tool to find the correct path."
                } else {
                    ""
                };
                return Ok(ToolOutput::error(format!("cannot read '{path_str}': {e}.{hint}")));
            }
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
                "refusing to read '{path_str}': matches secret-file pattern (e.g. .env, *.pem, credentials.*)"
            )));
        }

        if let Some(data_dir) = ctx.data_dir()
            && is_token_path(&canonical, data_dir)
        {
            return Ok(ToolOutput::error(format!(
                "refusing to read '{path_str}': resolves into the OAuth token directory"
            )));
        }

        // Check if path is a directory
        if canonical.is_dir() {
            return Ok(ToolOutput::error(format!(
                "'{path_str}' is a directory, not a file. Use the glob tool to list directory contents (e.g., glob with pattern \"**/*\" and path \"{path_str}\")."
            )));
        }

        // Read the file
        let content = match tokio::fs::read_to_string(&canonical).await {
            Ok(c) => c,
            Err(e) => return Ok(ToolOutput::error(format!("cannot read '{path_str}': {e}"))),
        };

        // Track that this file has been read
        ctx.mark_file_read(canonical.clone());

        // Apply offset and limit
        let lines: Vec<&str> = content.lines().collect();
        let start = offset.min(lines.len());
        let end = match limit {
            Some(l) => (start + l).min(lines.len()),
            None => lines.len(),
        };

        let result = lines[start..end].join("\n");
        Ok(ToolOutput::success(result))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::EngineToolContext;
    use arawn_core::Workstream;
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn test_ctx_with_dir(dir: &Path) -> EngineToolContext {
        let ws = Workstream::new("test", dir);
        EngineToolContext::new(&ws, Uuid::new_v4())
    }

    #[tokio::test]
    async fn read_existing_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("hello.txt");
        std::fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();

        let tool = FileReadTool;
        let ctx = test_ctx_with_dir(dir.path());
        let result = tool
            .execute(&ctx, json!({"path": "hello.txt"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("line 1"));
        assert!(result.content.contains("line 3"));
    }

    #[tokio::test]
    async fn read_with_offset_and_limit() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("lines.txt");
        std::fs::write(&file_path, "a\nb\nc\nd\ne\n").unwrap();

        let tool = FileReadTool;
        let ctx = test_ctx_with_dir(dir.path());
        let result = tool
            .execute(&ctx, json!({"path": "lines.txt", "offset": 1, "limit": 2}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content, "b\nc");
    }

    #[tokio::test]
    async fn read_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let tool = FileReadTool;
        let ctx = test_ctx_with_dir(dir.path());
        let result = tool
            .execute(&ctx, json!({"path": "nope.txt"}))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("cannot read"));
    }

    #[tokio::test]
    async fn path_traversal_rejected() {
        let dir = TempDir::new().unwrap();
        // Create a file outside the workstream root to attempt traversal against
        let parent = dir.path().parent().unwrap();
        let outside_file = parent.join("outside.txt");
        let mut f = std::fs::File::create(&outside_file).unwrap();
        write!(f, "secret").unwrap();

        let tool = FileReadTool;
        let ctx = test_ctx_with_dir(dir.path());
        let result = tool
            .execute(&ctx, json!({"path": "../outside.txt"}))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("escapes workstream root"));

        // Cleanup
        let _ = std::fs::remove_file(outside_file);
    }

    #[tokio::test]
    async fn missing_path_param() {
        let dir = TempDir::new().unwrap();
        let tool = FileReadTool;
        let ctx = test_ctx_with_dir(dir.path());
        let result = tool.execute(&ctx, json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn schema_is_valid() {
        let tool = FileReadTool;
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["path"].is_object());
    }

    #[tokio::test]
    async fn refuses_token_dir_path() {
        // Workstream root doubles as data_dir; tokens/ lives inside it.
        let dir = TempDir::new().unwrap();
        let tokens = dir.path().join("tokens");
        std::fs::create_dir_all(&tokens).unwrap();
        std::fs::write(tokens.join("google.json.enc"), b"encrypted").unwrap();

        let tool = FileReadTool;
        let ws = Workstream::new("test", dir.path());
        let ctx = EngineToolContext::new(&ws, Uuid::new_v4())
            .with_data_dir(dir.path().to_path_buf());

        let result = tool
            .execute(&ctx, json!({"path": "tokens/google.json.enc"}))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("OAuth token directory"), "got: {}", result.content);
    }

    #[tokio::test]
    async fn refuses_dotenv_in_workstream() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env"), "API_KEY=secret").unwrap();

        let tool = FileReadTool;
        let ctx = test_ctx_with_dir(dir.path());
        let result = tool.execute(&ctx, json!({"path": ".env"})).await.unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("secret-file pattern"));
    }

    #[tokio::test]
    async fn allows_legitimate_env_rs() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("env.rs"), "fn main() {}").unwrap();

        let tool = FileReadTool;
        let ctx = test_ctx_with_dir(dir.path());
        let result = tool
            .execute(&ctx, json!({"path": "env.rs"}))
            .await
            .unwrap();

        assert!(!result.is_error, "env.rs is a Rust file, not a secret");
    }

}

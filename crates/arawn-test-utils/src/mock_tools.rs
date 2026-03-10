//! Mock tools for E2E testing.
//!
//! These tools implement the `Tool` trait from `arawn-agent` and can be
//! registered in a `ToolRegistry` for integration tests that exercise
//! the full tool-execution pipeline.

use async_trait::async_trait;
use serde_json::json;

use arawn_agent::{Tool, ToolContext, ToolResult};

// ─────────────────────────────────────────────────────────────────────────────
// EchoTool
// ─────────────────────────────────────────────────────────────────────────────

/// A tool that echoes its input back. Useful for verifying tool execution
/// round-trips through the agent loop.
#[derive(Debug)]
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echoes the input message back. Useful for testing tool execution."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo back"
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> arawn_agent::Result<ToolResult> {
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("(no message)");
        Ok(ToolResult::text(format!("Echo: {}", message)))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MockReadFileTool
// ─────────────────────────────────────────────────────────────────────────────

/// A mock file reader that returns canned content based on path.
#[derive(Debug)]
pub struct MockReadFileTool {
    /// Map of path → content for canned responses.
    files: std::collections::HashMap<String, String>,
}

impl MockReadFileTool {
    /// Create a new mock file reader with default files.
    pub fn new() -> Self {
        let mut files = std::collections::HashMap::new();
        files.insert(
            "/test/hello.txt".to_string(),
            "Hello from the test file!".to_string(),
        );
        files.insert(
            "/test/data.json".to_string(),
            r#"{"key": "value", "count": 42}"#.to_string(),
        );
        Self { files }
    }

    /// Add a file to the mock filesystem.
    pub fn with_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.files.insert(path.into(), content.into());
        self
    }
}

impl Default for MockReadFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MockReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> arawn_agent::Result<ToolResult> {
        let path = params.get("path").and_then(|v| v.as_str()).unwrap_or("");

        match self.files.get(path) {
            Some(content) => Ok(ToolResult::text(content)),
            None => Ok(ToolResult::error(format!("File not found: {}", path))),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FailTool
// ─────────────────────────────────────────────────────────────────────────────

/// A tool that always returns an error. Useful for testing error recovery.
#[derive(Debug)]
pub struct FailTool;

#[async_trait]
impl Tool for FailTool {
    fn name(&self) -> &str {
        "fail_tool"
    }

    fn description(&self) -> &str {
        "A tool that always fails. Used for testing error handling."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "reason": {
                    "type": "string",
                    "description": "The reason for the failure"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> arawn_agent::Result<ToolResult> {
        let reason = params
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("Tool failed as expected");
        Ok(ToolResult::error(reason))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SlowTool
// ─────────────────────────────────────────────────────────────────────────────

/// A tool that sleeps for a configurable duration before responding.
/// Useful for testing timeouts, cancellation, and concurrent tool execution.
#[derive(Debug)]
pub struct SlowTool {
    delay: std::time::Duration,
}

impl SlowTool {
    pub fn new(delay: std::time::Duration) -> Self {
        Self { delay }
    }
}

#[async_trait]
impl Tool for SlowTool {
    fn name(&self) -> &str {
        "slow_tool"
    }

    fn description(&self) -> &str {
        "A tool that takes time to respond. Used for testing timeouts."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to return after delay"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> arawn_agent::Result<ToolResult> {
        tokio::time::sleep(self.delay).await;
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("done");
        Ok(ToolResult::text(format!("Slow result: {}", message)))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LargeOutputTool
// ─────────────────────────────────────────────────────────────────────────────

/// A tool that returns a very large output. Useful for testing output
/// sanitization and truncation.
#[derive(Debug)]
pub struct LargeOutputTool {
    output_size: usize,
}

impl LargeOutputTool {
    pub fn new(output_size: usize) -> Self {
        Self { output_size }
    }
}

#[async_trait]
impl Tool for LargeOutputTool {
    fn name(&self) -> &str {
        "large_output"
    }

    fn description(&self) -> &str {
        "A tool that returns a very large output."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(
        &self,
        _params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> arawn_agent::Result<ToolResult> {
        let output = "x".repeat(self.output_size);
        Ok(ToolResult::text(output))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: create a registry with all mock tools
// ─────────────────────────────────────────────────────────────────────────────

/// Create a `ToolRegistry` pre-loaded with all mock tools (echo, read_file, fail_tool).
pub fn mock_tool_registry() -> arawn_agent::ToolRegistry {
    let mut registry = arawn_agent::ToolRegistry::new();
    registry.register(EchoTool);
    registry.register(MockReadFileTool::new());
    registry.register(FailTool);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_agent::{SessionId, TurnId};

    fn test_context() -> ToolContext {
        ToolContext::new(SessionId::new(), TurnId::new())
    }

    #[tokio::test]
    async fn test_echo_tool() {
        let tool = EchoTool;
        let ctx = test_context();
        let result = tool
            .execute(json!({"message": "hello world"}), &ctx)
            .await
            .unwrap();
        assert!(result.is_success());
        assert_eq!(result.to_llm_content(), "Echo: hello world");
    }

    #[tokio::test]
    async fn test_read_file_found() {
        let tool = MockReadFileTool::new();
        let ctx = test_context();
        let result = tool
            .execute(json!({"path": "/test/hello.txt"}), &ctx)
            .await
            .unwrap();
        assert!(result.is_success());
        assert_eq!(result.to_llm_content(), "Hello from the test file!");
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let tool = MockReadFileTool::new();
        let ctx = test_context();
        let result = tool
            .execute(json!({"path": "/nonexistent"}), &ctx)
            .await
            .unwrap();
        assert!(result.is_error());
    }

    #[tokio::test]
    async fn test_read_file_custom() {
        let tool = MockReadFileTool::new().with_file("/custom.rs", "fn main() {}");
        let ctx = test_context();
        let result = tool
            .execute(json!({"path": "/custom.rs"}), &ctx)
            .await
            .unwrap();
        assert!(result.is_success());
        assert_eq!(result.to_llm_content(), "fn main() {}");
    }

    #[tokio::test]
    async fn test_fail_tool() {
        let tool = FailTool;
        let ctx = test_context();
        let result = tool
            .execute(json!({"reason": "test failure"}), &ctx)
            .await
            .unwrap();
        assert!(result.is_error());
        assert!(result.to_llm_content().contains("test failure"));
    }

    #[test]
    fn test_mock_registry() {
        let registry = mock_tool_registry();
        assert_eq!(registry.len(), 3);
        assert!(registry.contains("echo"));
        assert!(registry.contains("read_file"));
        assert!(registry.contains("fail_tool"));
    }
}

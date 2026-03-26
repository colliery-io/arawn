//! Filesystem gate enforcement for tool execution.
//!
//! Validates file paths and routes shell commands through the OS-level sandbox.

use arawn_types::SharedFsGate;

use super::command_validator::{CommandValidation, CommandValidator};
use super::context::{GatedParam, Tool, ToolContext, ToolResult};
use super::output::OutputConfig;
use super::registry::ToolRegistry;
use crate::error::Result;

/// Validate and rewrite filesystem paths in tool params against the gate.
///
/// Iterates over the tool's declared `GatedParam`s, validating each
/// path parameter against the filesystem gate. Returns `Ok(params)` with
/// canonicalized paths on success, or `Err(ToolResult)` on access denial.
pub(super) fn enforce_gate_params(
    gated: &[GatedParam],
    mut params: serde_json::Value,
    gate: &SharedFsGate,
) -> std::result::Result<serde_json::Value, ToolResult> {
    for gp in gated {
        match gp {
            GatedParam::ReadPath(name) => {
                if let Some(path_str) = params.get(*name).and_then(|v| v.as_str()) {
                    let path = std::path::Path::new(path_str);
                    match gate.validate_read(path) {
                        Ok(canonical) => {
                            params[*name] =
                                serde_json::Value::String(canonical.to_string_lossy().to_string());
                        }
                        Err(e) => {
                            return Err(ToolResult::error(format!("Access denied: {}", e)));
                        }
                    }
                }
            }
            GatedParam::WritePath(name) => {
                if let Some(path_str) = params.get(*name).and_then(|v| v.as_str()) {
                    let path = std::path::Path::new(path_str);
                    match gate.validate_write(path) {
                        Ok(canonical) => {
                            params[*name] =
                                serde_json::Value::String(canonical.to_string_lossy().to_string());
                        }
                        Err(e) => {
                            return Err(ToolResult::error(format!("Access denied: {}", e)));
                        }
                    }
                }
            }
            GatedParam::WorkingDir(name) => {
                if let Some(path_str) = params.get(*name).and_then(|v| v.as_str()) {
                    let path = std::path::Path::new(path_str);
                    match gate.validate_read(path) {
                        Ok(canonical) => {
                            params[*name] =
                                serde_json::Value::String(canonical.to_string_lossy().to_string());
                        }
                        Err(e) => {
                            return Err(ToolResult::error(format!("Access denied: {}", e)));
                        }
                    }
                } else {
                    // Default to working directory
                    let wd = gate.working_dir();
                    params[*name] = serde_json::Value::String(wd.to_string_lossy().to_string());
                }
            }
        }
    }
    Ok(params)
}

impl ToolRegistry {
    /// Execute a shell tool through the OS-level sandbox.
    ///
    /// Validates the command against blocked patterns before passing it to
    /// the OS-level sandbox for execution.
    pub(super) async fn execute_shell_sandboxed(
        &self,
        _tool: &dyn Tool,
        params: &serde_json::Value,
        _ctx: &ToolContext,
        gate: &SharedFsGate,
        output_config: &OutputConfig,
    ) -> Result<ToolResult> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        // Validate command before sandbox execution
        let validator = CommandValidator::default();
        if let CommandValidation::Blocked(reason) = validator.validate(command) {
            return Ok(ToolResult::error(format!(
                "Command not allowed: {}",
                reason
            )));
        }

        let timeout = params
            .get("timeout")
            .and_then(|v| v.as_u64())
            .map(std::time::Duration::from_secs);

        match gate.sandbox_execute(command, timeout).await {
            Ok(output) => {
                let content = if output.stderr.is_empty() {
                    output.stdout
                } else if output.stdout.is_empty() {
                    output.stderr
                } else {
                    format!("{}\n\n--- stderr ---\n{}", output.stdout, output.stderr)
                };

                let result = if output.success {
                    ToolResult::text(content)
                } else {
                    ToolResult::error(format!(
                        "Command exited with code {}\n{}",
                        output.exit_code, content
                    ))
                };
                Ok(result.sanitize(output_config))
            }
            Err(e) => Ok(ToolResult::error(format!("Sandbox error: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;

    use super::super::context::{GatedParam, ToolContext, ToolResult};
    use super::super::output::OutputConfig;
    use super::super::registry::{MockTool, ToolRegistry};
    use arawn_types::is_gated_tool;

    // ─────────────────────────────────────────────────────────────────────────
    // Filesystem Gate Enforcement Tests
    // ─────────────────────────────────────────────────────────────────────────

    /// Mock filesystem gate for testing enforcement logic.
    struct MockFsGate {
        /// Paths that are allowed for read access.
        allowed_read: Vec<std::path::PathBuf>,
        /// Paths that are allowed for write access.
        allowed_write: Vec<std::path::PathBuf>,
        /// Working directory to report.
        work_dir: std::path::PathBuf,
        /// Shell command results to return.
        shell_result: std::sync::Mutex<Option<arawn_types::SandboxOutput>>,
    }

    impl MockFsGate {
        fn new(work_dir: impl Into<std::path::PathBuf>) -> Self {
            Self {
                allowed_read: Vec::new(),
                allowed_write: Vec::new(),
                work_dir: work_dir.into(),
                shell_result: std::sync::Mutex::new(None),
            }
        }

        fn allow_read(mut self, path: impl Into<std::path::PathBuf>) -> Self {
            self.allowed_read.push(path.into());
            self
        }

        fn allow_write(mut self, path: impl Into<std::path::PathBuf>) -> Self {
            self.allowed_write.push(path.into());
            self
        }

        fn with_shell_result(self, result: arawn_types::SandboxOutput) -> Self {
            *self.shell_result.lock().unwrap() = Some(result);
            self
        }
    }

    #[async_trait]
    impl arawn_types::FsGate for MockFsGate {
        fn validate_read(
            &self,
            path: &std::path::Path,
        ) -> std::result::Result<std::path::PathBuf, arawn_types::FsGateError> {
            for allowed in &self.allowed_read {
                if path.starts_with(allowed) || path == allowed {
                    return Ok(path.to_path_buf());
                }
            }
            Err(arawn_types::FsGateError::AccessDenied {
                path: path.to_path_buf(),
                reason: "path outside allowed read paths".to_string(),
            })
        }

        fn validate_write(
            &self,
            path: &std::path::Path,
        ) -> std::result::Result<std::path::PathBuf, arawn_types::FsGateError> {
            for allowed in &self.allowed_write {
                if path.starts_with(allowed) || path == allowed {
                    return Ok(path.to_path_buf());
                }
            }
            Err(arawn_types::FsGateError::AccessDenied {
                path: path.to_path_buf(),
                reason: "path outside allowed write paths".to_string(),
            })
        }

        fn working_dir(&self) -> &std::path::Path {
            &self.work_dir
        }

        async fn sandbox_execute(
            &self,
            _command: &str,
            _timeout: Option<std::time::Duration>,
        ) -> std::result::Result<arawn_types::SandboxOutput, arawn_types::FsGateError> {
            match self.shell_result.lock().unwrap().take() {
                Some(result) => Ok(result),
                None => Ok(arawn_types::SandboxOutput {
                    stdout: "sandboxed output".to_string(),
                    stderr: String::new(),
                    exit_code: 0,
                    success: true,
                }),
            }
        }
    }

    fn ctx_with_gate(gate: impl arawn_types::FsGate + 'static) -> ToolContext {
        ToolContext {
            fs_gate: Some(Arc::new(gate)),
            ..ToolContext::default()
        }
    }

    #[test]
    fn test_is_gated_tool() {
        assert!(is_gated_tool("file_read"));
        assert!(is_gated_tool("file_write"));
        assert!(is_gated_tool("glob"));
        assert!(is_gated_tool("grep"));
        assert!(is_gated_tool("shell"));

        assert!(!is_gated_tool("think"));
        assert!(!is_gated_tool("web_search"));
        assert!(!is_gated_tool("delegate"));
        assert!(!is_gated_tool("memory_store"));
        assert!(!is_gated_tool(""));
    }

    #[tokio::test]
    async fn test_gate_deny_by_default_no_gate() {
        let mock = Arc::new(
            MockTool::new("file_read")
                .with_gated_params(vec![GatedParam::ReadPath("path")])
                .with_response(ToolResult::text("secret")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        // ToolContext with no fs_gate (default)
        let ctx = ToolContext::default();
        let params = serde_json::json!({"path": "/etc/passwd"});

        let result = registry
            .execute_with_config("file_read", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_error());
        assert!(
            result
                .to_llm_content()
                .contains("requires a filesystem gate")
        );
        assert_eq!(mock.call_count(), 0, "Tool should not have been called");
    }

    #[tokio::test]
    async fn test_gate_deny_by_default_all_gated_tools() {
        let ctx = ToolContext::default();

        // Each gated tool name with its corresponding gated params
        let gated_tools: Vec<(&str, Vec<GatedParam>)> = vec![
            ("file_read", vec![GatedParam::ReadPath("path")]),
            ("file_write", vec![GatedParam::WritePath("path")]),
            ("glob", vec![GatedParam::ReadPath("directory")]),
            ("grep", vec![GatedParam::ReadPath("directory")]),
            ("shell", vec![GatedParam::WorkingDir("cwd")]),
            ("web_fetch", vec![GatedParam::WritePath("download")]),
        ];

        for (tool_name, gated_params) in gated_tools {
            let mut registry = ToolRegistry::new();
            registry.register(MockTool::new(tool_name).with_gated_params(gated_params));

            let result = registry
                .execute_with_config(
                    tool_name,
                    serde_json::json!({}),
                    &ctx,
                    &OutputConfig::default(),
                )
                .await
                .unwrap();

            assert!(
                result.is_error(),
                "Tool '{}' should be denied without gate",
                tool_name
            );
            assert!(
                result
                    .to_llm_content()
                    .contains("requires a filesystem gate"),
                "Tool '{}' error should mention gate requirement",
                tool_name
            );
        }
    }

    #[tokio::test]
    async fn test_gate_non_gated_tool_passes_through_without_gate() {
        let mock = Arc::new(MockTool::new("think").with_response(ToolResult::text("thought")));
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        // No gate set — non-gated tool should work fine
        let ctx = ToolContext::default();
        let result = registry
            .execute_with_config(
                "think",
                serde_json::json!({}),
                &ctx,
                &OutputConfig::default(),
            )
            .await
            .unwrap();

        assert!(result.is_success());
        assert_eq!(result.to_llm_content(), "thought");
        assert_eq!(mock.call_count(), 1, "Tool should have been called once");
    }

    #[tokio::test]
    async fn test_gate_file_read_allowed() {
        let mock = Arc::new(
            MockTool::new("file_read")
                .with_gated_params(vec![GatedParam::ReadPath("path")])
                .with_response(ToolResult::text("file contents")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work").allow_read("/work");
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"path": "/work/src/main.rs"});

        let result = registry
            .execute_with_config("file_read", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_success());
        assert_eq!(result.to_llm_content(), "file contents");
        assert_eq!(mock.call_count(), 1, "Tool should have been called once");
    }

    #[tokio::test]
    async fn test_gate_file_read_denied() {
        let mock = Arc::new(
            MockTool::new("file_read")
                .with_gated_params(vec![GatedParam::ReadPath("path")])
                .with_response(ToolResult::text("should not see")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work").allow_read("/work");
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"path": "/etc/passwd"});

        let result = registry
            .execute_with_config("file_read", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_error());
        assert!(result.to_llm_content().contains("Access denied"));
        assert_eq!(mock.call_count(), 0, "Tool should not have been called");
    }

    #[tokio::test]
    async fn test_gate_file_write_allowed() {
        let mock = Arc::new(
            MockTool::new("file_write")
                .with_gated_params(vec![GatedParam::WritePath("path")])
                .with_response(ToolResult::text("written")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work").allow_write("/work");
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"path": "/work/output.txt", "content": "hello"});

        let result = registry
            .execute_with_config("file_write", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_success());
        assert_eq!(mock.call_count(), 1, "Tool should have been called once");
    }

    #[tokio::test]
    async fn test_gate_file_write_denied() {
        let mock = Arc::new(
            MockTool::new("file_write")
                .with_gated_params(vec![GatedParam::WritePath("path")])
                .with_response(ToolResult::text("should not")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work").allow_write("/work");
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"path": "/etc/shadow", "content": "malicious"});

        let result = registry
            .execute_with_config("file_write", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_error());
        assert!(result.to_llm_content().contains("Access denied"));
        assert_eq!(mock.call_count(), 0, "Tool should not have been called");
    }

    #[tokio::test]
    async fn test_gate_glob_allowed() {
        let mock = Arc::new(
            MockTool::new("glob")
                .with_gated_params(vec![GatedParam::ReadPath("directory")])
                .with_response(ToolResult::text("file1.rs\nfile2.rs")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work").allow_read("/work");
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"directory": "/work/src"});

        let result = registry
            .execute_with_config("glob", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_success());
        assert_eq!(mock.call_count(), 1, "Tool should have been called once");
    }

    #[tokio::test]
    async fn test_gate_glob_denied() {
        let mock = Arc::new(
            MockTool::new("glob")
                .with_gated_params(vec![GatedParam::ReadPath("directory")])
                .with_response(ToolResult::text("should not")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work").allow_read("/work");
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"pattern": "*.rs", "directory": "/home/user/.ssh"});

        let result = registry
            .execute_with_config("glob", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(
            result.is_error(),
            "Glob to /home/user/.ssh should be denied"
        );
        assert!(result.to_llm_content().contains("Access denied"));
        assert_eq!(mock.call_count(), 0, "Tool should not have been called");
    }

    #[tokio::test]
    async fn test_gate_grep_denied() {
        let mock = Arc::new(
            MockTool::new("grep")
                .with_gated_params(vec![GatedParam::ReadPath("directory")])
                .with_response(ToolResult::text("should not")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work").allow_read("/work");
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"pattern": "password", "directory": "/var/log/secure"});

        let result = registry
            .execute_with_config("grep", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(
            result.is_error(),
            "Grep in /var/log/secure should be denied"
        );
        assert!(result.to_llm_content().contains("Access denied"));
        assert_eq!(mock.call_count(), 0, "Tool should not have been called");
    }

    #[tokio::test]
    async fn test_gate_shell_routed_through_sandbox() {
        let mut registry = ToolRegistry::new();
        // The mock tool should NOT be called — shell goes through sandbox
        registry.register(
            MockTool::new("shell")
                .with_gated_params(vec![GatedParam::WorkingDir("cwd")])
                .with_response(ToolResult::text("SHOULD NOT SEE")),
        );

        let gate = MockFsGate::new("/work")
            .allow_read("/work")
            .with_shell_result(arawn_types::SandboxOutput {
                stdout: "sandboxed ls output".to_string(),
                stderr: String::new(),
                exit_code: 0,
                success: true,
            });
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"command": "ls -la"});

        let result = registry
            .execute_with_config("shell", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_success());
        assert!(result.to_llm_content().contains("sandboxed ls output"));
        // Verify the mock tool was NOT called directly — sandbox bypasses tool.execute()
        assert!(
            !result.to_llm_content().contains("SHOULD NOT SEE"),
            "Shell should route through sandbox, not direct execution"
        );
    }

    #[tokio::test]
    async fn test_gate_shell_sandbox_failure() {
        let mut registry = ToolRegistry::new();
        registry.register(
            MockTool::new("shell").with_gated_params(vec![GatedParam::WorkingDir("cwd")]),
        );

        let gate = MockFsGate::new("/work")
            .allow_read("/work")
            .with_shell_result(arawn_types::SandboxOutput {
                stdout: String::new(),
                stderr: "permission denied".to_string(),
                exit_code: 1,
                success: false,
            });
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"command": "cat /etc/shadow"});

        let result = registry
            .execute_with_config("shell", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_error());
        assert!(result.to_llm_content().contains("permission denied"));
    }

    #[tokio::test]
    async fn test_gate_execute_raw_deny_by_default() {
        let mock = Arc::new(
            MockTool::new("file_read").with_gated_params(vec![GatedParam::ReadPath("path")]),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let ctx = ToolContext::default();
        let result = registry
            .execute_raw(
                "file_read",
                serde_json::json!({"path": "/etc/passwd"}),
                &ctx,
            )
            .await
            .unwrap();

        assert!(result.is_error());
        assert!(
            result
                .to_llm_content()
                .contains("requires a filesystem gate")
        );
        assert_eq!(mock.call_count(), 0, "Tool should not have been called");
    }

    #[tokio::test]
    async fn test_gate_execute_raw_allowed_with_gate() {
        let mock = Arc::new(
            MockTool::new("file_read")
                .with_gated_params(vec![GatedParam::ReadPath("path")])
                .with_response(ToolResult::text("raw contents")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work").allow_read("/work");
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"path": "/work/file.txt"});

        let result = registry
            .execute_raw("file_read", params, &ctx)
            .await
            .unwrap();

        assert!(result.is_success());
        // execute_raw skips sanitization, so raw content comes through
        assert_eq!(result.to_llm_content(), "raw contents");
        assert_eq!(mock.call_count(), 1, "Tool should have been called once");
    }

    #[tokio::test]
    async fn test_gate_execute_raw_non_gated_passes_through() {
        let mock = Arc::new(MockTool::new("think").with_response(ToolResult::text("deep thought")));
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let ctx = ToolContext::default();
        let result = registry
            .execute_raw("think", serde_json::json!({}), &ctx)
            .await
            .unwrap();

        assert!(result.is_success());
        assert_eq!(result.to_llm_content(), "deep thought");
        assert_eq!(mock.call_count(), 1, "Tool should have been called once");
    }

    #[tokio::test]
    async fn test_gate_file_read_no_path_param_passes_through() {
        let mock = Arc::new(
            MockTool::new("file_read")
                .with_gated_params(vec![GatedParam::ReadPath("path")])
                .with_response(ToolResult::text("ok")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        // Gate is present but params have no "path" key — validation is skipped
        let gate = MockFsGate::new("/work").allow_read("/work");
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"content": "something"});

        let result = registry
            .execute_with_config("file_read", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        // Tool executes without path validation (no path to validate)
        assert!(result.is_success());
        assert_eq!(mock.call_count(), 1, "Tool should have been called once");
    }

    #[tokio::test]
    async fn test_gate_shell_sandbox_combined_output() {
        let mut registry = ToolRegistry::new();
        registry.register(
            MockTool::new("shell").with_gated_params(vec![GatedParam::WorkingDir("cwd")]),
        );

        let gate = MockFsGate::new("/work")
            .allow_read("/work")
            .with_shell_result(arawn_types::SandboxOutput {
                stdout: "stdout content".to_string(),
                stderr: "stderr content".to_string(),
                exit_code: 0,
                success: true,
            });
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"command": "make build"});

        let result = registry
            .execute_with_config("shell", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_success());
        let content = result.to_llm_content();
        assert!(content.contains("stdout content"));
        assert!(content.contains("stderr content"));
        assert!(content.contains("--- stderr ---"));
    }

    #[tokio::test]
    async fn test_gate_shell_timeout_passed() {
        let mut registry = ToolRegistry::new();
        registry.register(
            MockTool::new("shell").with_gated_params(vec![GatedParam::WorkingDir("cwd")]),
        );

        let gate = MockFsGate::new("/work").allow_read("/work");
        let ctx = ctx_with_gate(gate);
        // The timeout param is extracted from params and passed to sandbox_execute
        let params = serde_json::json!({"command": "sleep 100", "timeout": 30});

        let result = registry
            .execute_with_config("shell", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        // Default MockFsGate returns success
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_gate_shell_blocked_command_rejected() {
        let mock =
            Arc::new(MockTool::new("shell").with_gated_params(vec![GatedParam::WorkingDir("cwd")]));
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work")
            .allow_read("/work")
            .with_shell_result(arawn_types::SandboxOutput {
                stdout: "SHOULD NOT REACH".to_string(),
                stderr: String::new(),
                exit_code: 0,
                success: true,
            });
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"command": "rm -rf /"});

        let result = registry
            .execute_with_config("shell", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_error());
        assert!(result.to_llm_content().contains("not allowed"));
        assert!(!result.to_llm_content().contains("SHOULD NOT REACH"));
        assert_eq!(mock.call_count(), 0, "Tool should not have been called");
    }

    #[tokio::test]
    async fn test_gate_shell_blocked_command_case_bypass() {
        let mock =
            Arc::new(MockTool::new("shell").with_gated_params(vec![GatedParam::WorkingDir("cwd")]));
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work").allow_read("/work");
        let ctx = ctx_with_gate(gate);
        // Try to bypass with mixed case
        let params = serde_json::json!({"command": "RM -RF /"});

        let result = registry
            .execute_with_config("shell", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_error());
        assert!(result.to_llm_content().contains("not allowed"));
        assert_eq!(mock.call_count(), 0, "Tool should not have been called");
    }

    #[tokio::test]
    async fn test_gate_shell_blocked_command_whitespace_bypass() {
        let mock =
            Arc::new(MockTool::new("shell").with_gated_params(vec![GatedParam::WorkingDir("cwd")]));
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work").allow_read("/work");
        let ctx = ctx_with_gate(gate);
        // Try to bypass with extra whitespace
        let params = serde_json::json!({"command": "rm   -rf   /"});

        let result = registry
            .execute_with_config("shell", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_error());
        assert!(result.to_llm_content().contains("not allowed"));
        assert_eq!(mock.call_count(), 0, "Tool should not have been called");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Gated Params Declaration Enforcement Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_gated_params_custom_param_name_drives_enforcement() {
        // Tool declares gated_params with a custom param name "custom_dir"
        // Proves the gate reads the declaration, not a hardcoded name
        let mock = Arc::new(
            MockTool::new("custom_reader")
                .with_gated_params(vec![GatedParam::ReadPath("custom_dir")])
                .with_response(ToolResult::text("should not see")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let gate = MockFsGate::new("/work").allow_read("/work");
        let ctx = ctx_with_gate(gate);
        let params = serde_json::json!({"custom_dir": "/etc/passwd"});

        let result = registry
            .execute_with_config("custom_reader", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(
            result.is_error(),
            "Access to /etc/passwd should be denied via custom_dir param"
        );
        assert!(result.to_llm_content().contains("Access denied"));
        assert_eq!(mock.call_count(), 0, "Tool should not have been called");
    }

    #[tokio::test]
    async fn test_empty_gated_params_tool_passes_with_any_paths() {
        // Tool with empty gated_params() — ungated tool with path-like params
        let mock = Arc::new(
            MockTool::new("ungated_tool")
                // No .with_gated_params() — default is empty vec
                .with_response(ToolResult::text("passed")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let ctx = ToolContext::default(); // No gate needed for ungated tools
        let params = serde_json::json!({
            "path": "/etc/passwd",
            "directory": "/root/.ssh"
        });

        let result = registry
            .execute_with_config("ungated_tool", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(
            result.is_success(),
            "Ungated tool should pass regardless of path-like params"
        );
        assert_eq!(result.to_llm_content(), "passed");
        assert_eq!(mock.call_count(), 1, "Tool should have been called once");
    }

    #[tokio::test]
    async fn test_gated_params_no_fs_gate_denied_with_message() {
        // Tool with gated_params but no ctx.fs_gate — should deny with specific message
        let mock = Arc::new(
            MockTool::new("gated_no_gate")
                .with_gated_params(vec![GatedParam::ReadPath("target")])
                .with_response(ToolResult::text("should not see")),
        );
        let mut registry = ToolRegistry::new();
        registry.register_arc(mock.clone());

        let ctx = ToolContext::default(); // No fs_gate
        let params = serde_json::json!({"target": "/work/file.txt"});

        let result = registry
            .execute_with_config("gated_no_gate", params, &ctx, &OutputConfig::default())
            .await
            .unwrap();

        assert!(result.is_error());
        let msg = result.to_llm_content();
        assert!(
            msg.contains("requires a filesystem gate"),
            "Error should mention missing gate, got: {}",
            msg
        );
        assert_eq!(mock.call_count(), 0, "Tool should not have been called");
    }
}

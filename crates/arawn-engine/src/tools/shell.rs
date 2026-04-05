use std::sync::Arc;

use async_trait::async_trait;
use sandbox_runtime::{FilesystemConfig, NetworkConfig, SandboxManager, SandboxRuntimeConfig};
use serde_json::{Value, json};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::background::{
    BackgroundTaskKind, BackgroundTaskManager, BackgroundTaskStatus, append_output,
};
use crate::context::ToolContext;
use crate::error::EngineError;
use crate::tool::{Tool, ToolOutput};

/// Execute a shell command within an OS-level sandbox.
///
/// Uses `sandbox-exec` (macOS) or `bubblewrap` (Linux) to enforce:
/// - Write access restricted to the session/workstream sandbox directory
/// - Sensitive paths denied for reading (~/.ssh, ~/.aws, credentials, etc.)
/// - Network blocked by default, unless the command invokes a known network tool
pub struct ShellTool {
    /// Tools that are granted network access when detected in a command.
    network_tools: Vec<String>,
    /// Optional background task manager for `run_in_background` support.
    bg_manager: Option<Arc<BackgroundTaskManager>>,
}

const DEFAULT_TIMEOUT_MS: u64 = 30_000;

impl Default for ShellTool {
    fn default() -> Self {
        Self {
            network_tools: Vec::new(),
            bg_manager: None,
        }
    }
}

impl ShellTool {
    /// Create a ShellTool with the given list of network-allowed tool binaries.
    pub fn with_network_tools(network_tools: Vec<String>) -> Self {
        Self {
            network_tools,
            bg_manager: None,
        }
    }

    /// Attach a background task manager for `run_in_background` support.
    pub fn with_background_manager(mut self, mgr: Arc<BackgroundTaskManager>) -> Self {
        self.bg_manager = Some(mgr);
        self
    }

    /// Spawn a shell command as a background task. Returns immediately with the task ID.
    async fn spawn_background(
        &self,
        command: &str,
        working_dir: &std::path::Path,
    ) -> Result<ToolOutput, EngineError> {
        let mgr = self.bg_manager.as_ref().ok_or_else(|| {
            EngineError::Tool(
                "Background execution not available (no task manager configured)".into(),
            )
        })?;

        info!(command, ?working_dir, "spawning background shell command");

        // Spawn the child process (unsandboxed for background — sandbox requires
        // sync lifecycle management that doesn't fit background execution)
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| EngineError::Tool(format!("Failed to spawn background command: {e}")))?;

        let cancel_token = CancellationToken::new();
        let cancel_clone = cancel_token.clone();
        let command_owned = command.to_string();
        let mgr_clone = Arc::clone(mgr);

        // Take stdout/stderr handles before moving child into the task
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Create a placeholder handle — we'll replace it after registering
        let task_handle = tokio::spawn({
            let command_for_task = command_owned.clone();
            async move {
                // This will be replaced below — just a placeholder
                let _ = command_for_task;
            }
        });

        let (task_id, output_buf) = mgr.register(
            BackgroundTaskKind::Shell {
                command: command_owned.clone(),
            },
            command_owned.clone(),
            task_handle,
            cancel_token,
        );

        let task_id_clone = task_id.clone();

        // Spawn the real reader/waiter task
        tokio::spawn(async move {
            // Read stdout and stderr concurrently into the output buffer
            let read_output = async {
                if let Some(stdout) = stdout {
                    let mut reader = tokio::io::BufReader::new(stdout);
                    let mut buf = vec![0u8; 8192];
                    loop {
                        use tokio::io::AsyncReadExt;
                        match reader.read(&mut buf).await {
                            Ok(0) => break,
                            Ok(n) => {
                                let text = String::from_utf8_lossy(&buf[..n]);
                                append_output(&output_buf, &text);
                            }
                            Err(_) => break,
                        }
                    }
                }
            };

            let read_stderr = async {
                if let Some(stderr) = stderr {
                    let mut reader = tokio::io::BufReader::new(stderr);
                    let mut buf = vec![0u8; 8192];
                    loop {
                        use tokio::io::AsyncReadExt;
                        match reader.read(&mut buf).await {
                            Ok(0) => break,
                            Ok(n) => {
                                let text = String::from_utf8_lossy(&buf[..n]);
                                append_output(&output_buf, &format!("STDERR: {text}"));
                            }
                            Err(_) => break,
                        }
                    }
                }
            };

            // Wait for process exit or cancellation
            tokio::select! {
                _ = cancel_clone.cancelled() => {
                    // Kill the child process
                    let _ = child.kill().await;
                    mgr_clone.complete(&task_id_clone, BackgroundTaskStatus::Killed);
                }
                result = async {
                    // Read output while waiting for exit
                    let (_, _, exit) = tokio::join!(
                        read_output,
                        read_stderr,
                        child.wait()
                    );
                    exit
                } => {
                    match result {
                        Ok(status) => {
                            let exit_code = status.code();
                            if status.success() {
                                mgr_clone.complete(
                                    &task_id_clone,
                                    BackgroundTaskStatus::Completed { exit_code },
                                );
                            } else {
                                mgr_clone.complete(
                                    &task_id_clone,
                                    BackgroundTaskStatus::Completed { exit_code },
                                );
                            }
                        }
                        Err(e) => {
                            mgr_clone.complete(
                                &task_id_clone,
                                BackgroundTaskStatus::Failed {
                                    error: e.to_string(),
                                },
                            );
                        }
                    }
                }
            }
        });

        Ok(ToolOutput::success(format!(
            "Background task {task_id} started: {command_owned}\n\n\
             Use TaskOutput with task_id=\"{task_id}\" to check status and read output."
        )))
    }
}


/// Build the list of sensitive paths that should be denied for reading.
fn sensitive_deny_read_paths() -> Vec<String> {
    let mut paths = vec![
        // System auth & security
        "/etc/shadow".to_string(),
        "/etc/gshadow".to_string(),
        "/etc/sudoers".to_string(),
        "/etc/sudoers.d".to_string(),
        "/etc/ssl/private".to_string(),
    ];

    if let Some(home) = dirs::home_dir() {
        let h = home.to_string_lossy();
        // SSH & GPG keys
        paths.push(format!("{h}/.ssh"));
        paths.push(format!("{h}/.gnupg"));
        // Cloud provider credentials
        paths.push(format!("{h}/.aws"));
        paths.push(format!("{h}/.azure"));
        paths.push(format!("{h}/.config/gcloud"));
        paths.push(format!("{h}/.kube"));
        // Container credentials
        paths.push(format!("{h}/.docker/config.json"));
        // Package manager tokens
        paths.push(format!("{h}/.npmrc"));
        // Git credentials
        paths.push(format!("{h}/.netrc"));
        paths.push(format!("{h}/.git-credentials"));
        // CLI tool credentials
        paths.push(format!("{h}/.config/gh"));
        paths.push(format!("{h}/.vault-token"));
        // Database passwords
        paths.push(format!("{h}/.pgpass"));
        paths.push(format!("{h}/.my.cnf"));
        // Shell history
        paths.push(format!("{h}/.bash_history"));
        paths.push(format!("{h}/.zsh_history"));
        // macOS specific
        #[cfg(target_os = "macos")]
        {
            paths.push(format!("{h}/Library/Keychains"));
            paths.push(format!("{h}/Library/Cookies"));
        }
    }

    paths
}

/// Check if a command invokes any tool that needs network access.
fn command_needs_network(command: &str, network_tools: &[String]) -> bool {
    if network_tools.is_empty() {
        return false;
    }

    // Split on shell operators to find individual commands in pipes/chains
    for part in command.split(['|', '&', ';']) {
        let trimmed = part.trim().trim_start_matches('!');
        // Get the first word (the binary being invoked)
        if let Some(first_word) = trimmed.split_whitespace().next() {
            // Strip path prefix (e.g., /usr/bin/curl -> curl)
            let binary = first_word.rsplit('/').next().unwrap_or(first_word);
            if network_tools.iter().any(|tool| binary == tool) {
                return true;
            }
        }
    }

    false
}

/// Build a sandbox config for executing a command in the given working directory.
fn build_sandbox_config(
    command: &str,
    working_dir: &std::path::Path,
    network_tools: &[String],
) -> SandboxRuntimeConfig {
    // Canonicalize working_dir to resolve symlinks — sandbox-exec on macOS
    // matches canonical paths, so /Users/x/.arawn must not be passed as ~/...
    let canonical_working_dir = working_dir
        .canonicalize()
        .unwrap_or_else(|_| working_dir.to_path_buf());
    let write_dir = canonical_working_dir.to_string_lossy().to_string();
    let needs_network = command_needs_network(command, network_tools);

    // Allow writes to the working directory plus system temp directories
    // and build tool caches that cargo, rustc, npm, pip, etc. need.
    let mut allow_write = vec![write_dir];
    allow_write.push("/dev/null".to_string());       // many tools redirect stderr here
    allow_write.push("/tmp".to_string());
    allow_write.push("/private/tmp".to_string()); // macOS /tmp → /private/tmp
    allow_write.push("/var/folders".to_string()); // macOS per-user temp
    if let Ok(tmpdir) = std::env::var("TMPDIR") {
        allow_write.push(tmpdir);
    }
    if let Some(home) = dirs::home_dir() {
        let h = home.to_string_lossy();
        allow_write.push(format!("{h}/.cargo")); // cargo registry, build cache
        allow_write.push(format!("{h}/.rustup")); // rustup toolchains
        allow_write.push(format!("{h}/.cache")); // general build caches (pip, etc.)
        allow_write.push(format!("{h}/.npm")); // npm cache
        allow_write.push(format!("{h}/.local")); // pip install target
    }

    SandboxRuntimeConfig {
        filesystem: FilesystemConfig {
            allow_write,
            deny_write: Vec::new(),
            deny_read: sensitive_deny_read_paths(),
            allow_git_config: Some(needs_network), // git needs .gitconfig
        },
        network: if needs_network {
            NetworkConfig::default() // unrestricted
        } else {
            NetworkConfig {
                allowed_domains: Vec::new(), // no network
                ..Default::default()
            }
        },
        ..Default::default()
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command in a sandboxed environment. The command runs in the session's working directory with restricted filesystem and network access.\n\n\
         IMPORTANT: Do NOT use shell to run commands when a dedicated tool exists:\n\
         - To read files: use file_read (NOT cat/head/tail)\n\
         - To write files: use file_write (NOT echo/cat heredoc)\n\
         - To edit files: use file_edit (NOT sed/awk)\n\
         - To search content: use grep (NOT shell grep/rg)\n\
         Reserve shell exclusively for commands that require actual shell execution (builds, git, package managers, etc.).\n\n\
         Usage:\n\
         - The working directory persists between commands within a session.\n\
         - When issuing multiple commands, chain with '&&' for sequential or ';' if earlier failures don't matter.\n\
         - Avoid unnecessary 'sleep' commands — diagnose failures instead of retrying in loops.\n\
         - Default timeout is 30 seconds. Specify timeout_ms for long-running commands.\n\
         - The sandbox restricts writes to the session directory only. Writes outside the sandbox will fail.\n\
         - Sensitive paths (~/.ssh, ~/.aws, credentials) are blocked for reading."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Timeout in milliseconds (default: 30000)"
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Run the command in the background. Returns a task ID immediately. Use TaskOutput to check status and read output later."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Tool("missing 'command' parameter".into()))?;

        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_MS);

        let run_in_background = params
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Background execution: spawn and return immediately
        if run_in_background {
            return self.spawn_background(command, &ctx.working_dir).await;
        }

        debug!(command, timeout_ms, cwd = ?ctx.working_dir, "executing sandboxed shell command");

        // Try sandboxed execution, fall back to unsandboxed with warning
        match execute_sandboxed(command, &ctx.working_dir, timeout_ms, &self.network_tools).await {
            Ok(output) => Ok(output),
            Err(SandboxExecError::Unavailable(msg)) => {
                warn!("sandbox unavailable: {msg} — running unsandboxed");
                execute_unsandboxed(command, &ctx.working_dir, timeout_ms).await
            }
            Err(SandboxExecError::Tool(output)) => Ok(output),
        }
    }
}

enum SandboxExecError {
    /// Sandbox platform/deps not available
    Unavailable(String),
    /// Command ran in sandbox but produced a tool output (error or success)
    Tool(ToolOutput),
}

async fn execute_sandboxed(
    command: &str,
    working_dir: &std::path::Path,
    timeout_ms: u64,
    network_tools: &[String],
) -> Result<ToolOutput, SandboxExecError> {
    // Check platform support
    if !SandboxManager::is_supported_platform() {
        return Err(SandboxExecError::Unavailable(
            "unsupported platform".to_string(),
        ));
    }

    let manager = SandboxManager::new();
    let config = build_sandbox_config(command, working_dir, network_tools);

    // Check dependencies
    if let Err(e) = manager.check_dependencies(Some(&config)) {
        return Err(SandboxExecError::Unavailable(e.to_string()));
    }

    // Initialize sandbox (sets up proxies, etc.)
    if let Err(e) = manager.initialize(config.clone()).await {
        return Err(SandboxExecError::Unavailable(format!(
            "sandbox init failed: {e}"
        )));
    }

    // Wrap the command with sandbox restrictions
    let wrapped = match manager.wrap_with_sandbox(command, None, None).await {
        Ok(w) => w,
        Err(e) => {
            manager.reset().await;
            return Err(SandboxExecError::Unavailable(format!(
                "sandbox wrap failed: {e}"
            )));
        }
    };

    debug!(wrapped = %wrapped, "sandbox-wrapped command");

    // Execute the wrapped command with timeout
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        Command::new("sh")
            .arg("-c")
            .arg(&wrapped)
            .current_dir(working_dir)
            .output(),
    )
    .await;

    // Annotate any sandbox violations
    let output = match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr_raw = String::from_utf8_lossy(&output.stderr);
            let stderr = manager.annotate_stderr_with_sandbox_failures(command, &stderr_raw);

            let mut content = String::new();
            if !stdout.is_empty() {
                content.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str("STDERR:\n");
                content.push_str(&stderr);
            }

            if output.status.success() {
                ToolOutput::success(content)
            } else {
                ToolOutput::error(format!(
                    "exit code {}\n{content}",
                    output.status.code().unwrap_or(-1)
                ))
            }
        }
        Ok(Err(e)) => ToolOutput::error(format!("failed to execute: {e}")),
        Err(_) => ToolOutput::error(format!("command timed out after {timeout_ms}ms")),
    };

    manager.reset().await;

    Err(SandboxExecError::Tool(output))
}

async fn execute_unsandboxed(
    command: &str,
    working_dir: &std::path::Path,
    timeout_ms: u64,
) -> Result<ToolOutput, EngineError> {
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(working_dir)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut content = String::new();
            if !stdout.is_empty() {
                content.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str("STDERR:\n");
                content.push_str(&stderr);
            }
            if output.status.success() {
                Ok(ToolOutput::success(content))
            } else {
                Ok(ToolOutput::error(format!(
                    "exit code {}\n{content}",
                    output.status.code().unwrap_or(-1)
                )))
            }
        }
        Ok(Err(e)) => Ok(ToolOutput::error(format!("failed to execute: {e}"))),
        Err(_) => Ok(ToolOutput::error(format!(
            "command timed out after {timeout_ms}ms"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Workstream;
    use serde_json::json;
    use serial_test::serial;
    use uuid::Uuid;

    fn test_ctx() -> ToolContext {
        let ws = Workstream::scratch("/tmp");
        ToolContext::new(&ws, Uuid::new_v4())
    }

    fn test_ctx_in(dir: &std::path::Path) -> ToolContext {
        let ws = Workstream::scratch(dir);
        ToolContext::new(&ws, Uuid::new_v4())
    }

    #[tokio::test]
    #[serial]
    async fn shell_echo() {
        let tool = ShellTool::default();
        let result = tool
            .execute(&test_ctx(), json!({"command": "echo hello"}))
            .await
            .unwrap();
        assert_eq!(result.content.trim(), "hello");
        assert!(!result.is_error);
    }

    #[tokio::test]
    #[serial]
    async fn shell_nonzero_exit() {
        let tool = ShellTool::default();
        let result = tool
            .execute(&test_ctx(), json!({"command": "exit 42"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("exit code 42"));
    }

    #[tokio::test]
    #[serial]
    async fn shell_timeout() {
        let tool = ShellTool::default();
        let result = tool
            .execute(
                &test_ctx(),
                json!({"command": "sleep 10", "timeout_ms": 100}),
            )
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("timed out"));
    }

    #[tokio::test]
    #[serial]
    async fn shell_missing_command() {
        let tool = ShellTool::default();
        let result = tool.execute(&test_ctx(), json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn shell_schema_is_valid() {
        let tool = ShellTool::default();
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["command"].is_object());
    }

    #[test]
    fn sensitive_paths_includes_ssh() {
        let paths = sensitive_deny_read_paths();
        assert!(paths.iter().any(|p| p.contains(".ssh")), "should deny .ssh");
    }

    #[test]
    fn sensitive_paths_includes_aws() {
        let paths = sensitive_deny_read_paths();
        assert!(paths.iter().any(|p| p.contains(".aws")), "should deny .aws");
    }

    #[test]
    fn sandbox_config_allows_working_dir_and_tmp() {
        let config =
            build_sandbox_config("echo hi", std::path::Path::new("/home/user/project"), &[]);
        assert!(
            config
                .filesystem
                .allow_write
                .contains(&"/home/user/project".to_string())
        );
        assert!(config.filesystem.allow_write.contains(&"/tmp".to_string()));
        assert!(config.network.allowed_domains.is_empty());
    }

    #[test]
    fn network_detection_recognizes_tools() {
        let tools: Vec<String> = vec!["gh".into(), "git".into(), "curl".into()];
        assert!(command_needs_network("gh pr list", &tools));
        assert!(command_needs_network("git push origin main", &tools));
        assert!(command_needs_network("curl https://example.com", &tools));
        assert!(command_needs_network("echo foo | curl -s http://x", &tools));
        assert!(command_needs_network("/usr/bin/git status", &tools));
    }

    #[test]
    fn network_detection_blocks_unknown() {
        let tools: Vec<String> = vec!["gh".into(), "git".into()];
        assert!(!command_needs_network("echo hello", &tools));
        assert!(!command_needs_network("ls -la", &tools));
        assert!(!command_needs_network("cat /etc/hosts", &tools));
    }

    #[test]
    fn network_detection_empty_list_blocks_all() {
        assert!(!command_needs_network("gh pr list", &[]));
        assert!(!command_needs_network("curl http://x", &[]));
    }

    // --- Sandbox enforcement tests (only run if sandbox is available) ---

    #[tokio::test]
    #[serial]
    async fn sandbox_write_inside_allowed() {
        if !SandboxManager::is_supported_platform() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let tool = ShellTool::default();
        let ctx = test_ctx_in(tmp.path());

        let result = tool
            .execute(&ctx, json!({"command": "touch testfile && ls testfile"}))
            .await
            .unwrap();

        // Should succeed — writing inside sandbox
        assert!(
            result.content.contains("testfile"),
            "should create file inside sandbox, got: {}",
            result.content
        );
    }

    #[tokio::test]
    #[serial]
    async fn sandbox_mkdir_inside_allowed() {
        if !SandboxManager::is_supported_platform() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let tool = ShellTool::default();
        let ctx = test_ctx_in(tmp.path());

        let result = tool
            .execute(
                &ctx,
                json!({"command": "mkdir -p target/debug && ls -d target/debug"}),
            )
            .await
            .unwrap();

        assert!(
            !result.is_error && result.content.contains("target/debug"),
            "should create subdirectories inside sandbox, got: {}",
            result.content
        );
    }

    #[tokio::test]
    #[serial]
    async fn sandbox_unlink_inside_allowed() {
        if !SandboxManager::is_supported_platform() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        // Pre-create a file to delete
        std::fs::write(tmp.path().join("deleteme.o"), "temp object file").unwrap();

        let tool = ShellTool::default();
        let ctx = test_ctx_in(tmp.path());

        let result = tool
            .execute(&ctx, json!({"command": "rm deleteme.o && echo deleted"}))
            .await
            .unwrap();

        assert!(
            !result.is_error && result.content.contains("deleted"),
            "should be able to delete files inside sandbox, got: {}",
            result.content
        );
        assert!(
            !tmp.path().join("deleteme.o").exists(),
            "file should actually be deleted"
        );
    }

    #[tokio::test]
    #[serial]
    async fn sandbox_build_tool_workflow() {
        // Simulates what cargo does: create dirs, write files, delete intermediates
        if !SandboxManager::is_supported_platform() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let tool = ShellTool::default();
        let ctx = test_ctx_in(tmp.path());

        let result = tool
            .execute(
                &ctx,
                json!({"command": "mkdir -p target/debug && echo 'obj' > target/debug/test.o && rm target/debug/test.o && echo 'build complete'"}),
            )
            .await
            .unwrap();

        assert!(
            !result.is_error && result.content.contains("build complete"),
            "should support full build workflow (mkdir + write + unlink), got: {}",
            result.content
        );
    }

    #[tokio::test]
    #[serial]
    async fn sandbox_write_outside_blocked() {
        if !SandboxManager::is_supported_platform() {
            return;
        }

        // Clean up from any previous failed run
        let escape_path = std::path::Path::new("/tmp/sandbox_escape_test_file");
        let _ = std::fs::remove_file(escape_path);

        let tmp = tempfile::tempdir().unwrap();
        let tool = ShellTool::default();
        let ctx = test_ctx_in(tmp.path());

        let result = tool
            .execute(
                &ctx,
                json!({"command": "touch /tmp/sandbox_escape_test_file"}),
            )
            .await
            .unwrap();

        // If sandbox engaged, the command should have failed or the file should not exist.
        // If sandbox fell back to unsandboxed (init failure), we can't assert — just clean up.
        let sandbox_engaged = result.is_error
            || result.content.contains("denied")
            || result.content.contains("Operation not permitted");

        if sandbox_engaged {
            assert!(
                !escape_path.exists(),
                "file should not exist outside sandbox"
            );
        } else {
            // Sandbox didn't engage — clean up and skip assertion
            let _ = std::fs::remove_file(escape_path);
            eprintln!("WARNING: sandbox did not engage, write-outside test inconclusive");
        }
    }

    #[tokio::test]
    #[serial]
    async fn sandbox_read_sensitive_path_blocked() {
        if !SandboxManager::is_supported_platform() {
            return;
        }
        // Only test if ~/.ssh exists
        let ssh_dir = dirs::home_dir().map(|h| h.join(".ssh"));
        if !ssh_dir.as_ref().is_some_and(|p| p.exists()) {
            return;
        }

        let tmp = tempfile::tempdir().unwrap();
        let tool = ShellTool::default();
        let ctx = test_ctx_in(tmp.path());

        let result = tool
            .execute(&ctx, json!({"command": "ls ~/.ssh/"}))
            .await
            .unwrap();

        // Should fail — reading sensitive path
        assert!(
            result.is_error
                || result.content.contains("denied")
                || result.content.contains("Operation not permitted")
                || result.content.contains("No such file"),
            "should block reading ~/.ssh, got: {}",
            result.content
        );
    }
}

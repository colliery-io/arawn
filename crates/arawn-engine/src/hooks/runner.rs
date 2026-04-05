use std::path::PathBuf;

use tracing::{debug, warn};

use super::config::{AggregatedHookResult, HookConfig};
use super::events::HookInput;
use super::executor::CommandHookExecutor;

/// Orchestrates hook matching, execution, and result aggregation.
///
/// Given a `HookConfig`, the runner:
/// 1. Finds all hooks matching the event + field value
/// 2. Executes them in parallel via `CommandHookExecutor`
/// 3. Aggregates results (any Block → overall Block)
pub struct HookRunner {
    config: HookConfig,
    /// Working directory for hook subprocesses.
    cwd: PathBuf,
}

impl HookRunner {
    pub fn new(config: HookConfig, cwd: PathBuf) -> Self {
        Self { config, cwd }
    }

    /// Run all matching hooks for the given input and return the aggregated result.
    pub async fn run(&self, input: &HookInput) -> AggregatedHookResult {
        let event = input.event();
        let field_value = input.matcher_value();

        // For tool events, content is the JSON-serialized tool input
        let content = self.extract_content(input);

        let matching_hooks = self.config.matching_hooks(event, field_value, &content);

        if matching_hooks.is_empty() {
            return AggregatedHookResult::default();
        }

        debug!(
            event = ?event,
            field_value,
            hook_count = matching_hooks.len(),
            "running hooks"
        );

        // Execute all matching hooks in parallel
        let futures: Vec<_> = matching_hooks
            .into_iter()
            .map(|hook_def| CommandHookExecutor::execute(hook_def, input, &self.cwd))
            .collect();

        let results = futures::future::join_all(futures).await;

        let mut aggregated = AggregatedHookResult::default();
        for result in results {
            aggregated.add(result);
        }

        if aggregated.blocked {
            warn!(
                event = ?event,
                reason = aggregated.block_reason.as_deref().unwrap_or("unknown"),
                "hook blocked operation"
            );
        }

        aggregated
    }

    /// Check if any hooks are configured (useful for fast-path skipping).
    pub fn has_hooks(&self) -> bool {
        !self.config.is_empty()
    }

    /// Extract the content string used for content-pattern matching.
    fn extract_content(&self, input: &HookInput) -> String {
        match input {
            HookInput::PreToolUse { tool_input, .. }
            | HookInput::PostToolUse { tool_input, .. }
            | HookInput::PostToolUseFailure { tool_input, .. }
            | HookInput::PermissionRequest { tool_input, .. }
            | HookInput::PermissionDenied { tool_input, .. } => {
                // For tool events, use the stringified tool input for content matching
                tool_input.to_string()
            }
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_blocking_hook() -> HookConfig {
        serde_json::from_value(serde_json::json!({
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        { "type": "command", "command": "echo 'blocked' >&2; exit 2", "timeout": 5 }
                    ]
                }
            ]
        }))
        .unwrap()
    }

    fn config_with_allowing_hook() -> HookConfig {
        serde_json::from_value(serde_json::json!({
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        { "type": "command", "command": "exit 0" }
                    ]
                }
            ],
            "PostToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        { "type": "command", "command": "exit 0" }
                    ]
                }
            ]
        }))
        .unwrap()
    }

    fn cwd() -> PathBuf {
        std::env::current_dir().unwrap()
    }

    #[tokio::test]
    async fn no_hooks_returns_default() {
        let runner = HookRunner::new(HookConfig::default(), cwd());
        let input = HookInput::PreToolUse {
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({"command": "ls"}),
        };
        let result = runner.run(&input).await;
        assert!(!result.blocked);
    }

    #[tokio::test]
    async fn blocking_hook_blocks() {
        let runner = HookRunner::new(config_with_blocking_hook(), cwd());
        let input = HookInput::PreToolUse {
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({"command": "rm -rf /"}),
        };
        let result = runner.run(&input).await;
        assert!(result.blocked);
        assert!(result.block_reason.unwrap().contains("blocked"));
    }

    #[tokio::test]
    async fn allowing_hook_allows() {
        let runner = HookRunner::new(config_with_allowing_hook(), cwd());
        let input = HookInput::PreToolUse {
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({"command": "ls"}),
        };
        let result = runner.run(&input).await;
        assert!(!result.blocked);
    }

    #[tokio::test]
    async fn non_matching_tool_skips_hooks() {
        let runner = HookRunner::new(config_with_blocking_hook(), cwd());
        let input = HookInput::PreToolUse {
            tool_name: "Read".into(),
            tool_input: serde_json::json!({"file_path": "/foo"}),
        };
        let result = runner.run(&input).await;
        assert!(!result.blocked);
    }

    #[tokio::test]
    async fn post_tool_use_runs() {
        let runner = HookRunner::new(config_with_allowing_hook(), cwd());
        let input = HookInput::PostToolUse {
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({"command": "ls"}),
            tool_output: "file1\nfile2".into(),
        };
        let result = runner.run(&input).await;
        assert!(!result.blocked);
    }

    #[tokio::test]
    async fn has_hooks_true_when_configured() {
        let runner = HookRunner::new(config_with_allowing_hook(), cwd());
        assert!(runner.has_hooks());
    }

    #[tokio::test]
    async fn has_hooks_false_when_empty() {
        let runner = HookRunner::new(HookConfig::default(), cwd());
        assert!(!runner.has_hooks());
    }

    #[tokio::test]
    async fn multiple_hooks_any_block_wins() {
        let config: HookConfig = serde_json::from_value(serde_json::json!({
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        { "type": "command", "command": "exit 0" },
                        { "type": "command", "command": "echo 'nope' >&2; exit 2" }
                    ]
                }
            ]
        }))
        .unwrap();

        let runner = HookRunner::new(config, cwd());
        let input = HookInput::PreToolUse {
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({"command": "ls"}),
        };
        let result = runner.run(&input).await;
        assert!(result.blocked);
    }
}

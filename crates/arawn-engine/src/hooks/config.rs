use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::events::HookEvent;
use super::matcher::HookMatcher;

/// Top-level hook configuration: maps event types to lists of hook groups.
///
/// Deserialized from the `hooks` key in settings.json:
/// ```json
/// {
///   "hooks": {
///     "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "..." }] }],
///     "SessionStart": [{ "hooks": [{ "type": "command", "command": "..." }] }]
///   }
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookConfig {
    /// Map from event name to list of hook groups.
    /// Uses a HashMap<String, ...> for flexible deserialization, then validated against HookEvent.
    #[serde(flatten)]
    pub events: HashMap<String, Vec<HookGroup>>,
}

impl HookConfig {
    /// Get all hook groups for a given event type.
    pub fn groups_for(&self, event: HookEvent) -> Vec<&HookGroup> {
        let key = event_to_key(event);
        self.events
            .get(key)
            .map(|groups| groups.iter().collect())
            .unwrap_or_default()
    }

    /// Get all command hook definitions that match a given event and field value.
    pub fn matching_hooks(
        &self,
        event: HookEvent,
        field_value: &str,
        content: &str,
    ) -> Vec<&CommandHookDef> {
        self.groups_for(event)
            .into_iter()
            .filter(|group| {
                group
                    .matcher
                    .as_ref()
                    .is_none_or(|m| m.matches(field_value, content))
            })
            .flat_map(|group| group.hooks.iter())
            .collect()
    }

    /// Merge another config into this one. Hooks from `other` are appended.
    pub fn merge(&mut self, other: HookConfig) {
        for (event_key, groups) in other.events {
            self.events
                .entry(event_key)
                .or_default()
                .extend(groups);
        }
    }

    /// Returns true if this config has no hooks defined.
    pub fn is_empty(&self) -> bool {
        self.events.values().all(|groups| groups.is_empty())
    }
}

/// A group of hooks sharing a common matcher.
///
/// ```json
/// {
///   "matcher": "Bash|Edit",
///   "hooks": [
///     { "type": "command", "command": "echo checking", "timeout": 5 }
///   ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookGroup {
    /// Optional matcher — if absent, hooks fire for all events of this type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<HookMatcher>,

    /// The hooks in this group.
    pub hooks: Vec<CommandHookDef>,
}

/// Definition of a command hook: a shell command to execute when the event fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHookDef {
    /// Hook type — currently only "command" is supported.
    #[serde(rename = "type")]
    pub hook_type: String,

    /// The shell command to execute.
    pub command: String,

    /// Timeout in seconds. Defaults to 10 if not specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// The result of executing a single hook.
#[derive(Debug, Clone)]
pub enum HookResult {
    /// Hook allowed the operation (exit code 0).
    Allow {
        stdout: String,
    },
    /// Hook blocked the operation (exit code 2).
    Block {
        reason: String,
        stderr: String,
    },
    /// Hook produced a warning but didn't block (other exit codes).
    Warn {
        message: String,
        stderr: String,
    },
}

impl HookResult {
    pub fn is_block(&self) -> bool {
        matches!(self, HookResult::Block { .. })
    }
}

/// Aggregated result from running all matching hooks for an event.
#[derive(Debug, Clone, Default)]
pub struct AggregatedHookResult {
    /// Whether any hook blocked the operation.
    pub blocked: bool,
    /// The blocking reason (from the first blocking hook).
    pub block_reason: Option<String>,
    /// Warning messages from non-blocking hooks.
    pub warnings: Vec<String>,
}

impl AggregatedHookResult {
    /// Merge a single hook result into the aggregate.
    pub fn add(&mut self, result: HookResult) {
        match result {
            HookResult::Block { reason, .. } => {
                if !self.blocked {
                    self.blocked = true;
                    self.block_reason = Some(reason);
                }
            }
            HookResult::Warn { message, .. } => {
                self.warnings.push(message);
            }
            HookResult::Allow { .. } => {}
        }
    }
}

/// Map a HookEvent to its config key string.
fn event_to_key(event: HookEvent) -> &'static str {
    match event {
        HookEvent::PreToolUse => "PreToolUse",
        HookEvent::PostToolUse => "PostToolUse",
        HookEvent::PostToolUseFailure => "PostToolUseFailure",
        HookEvent::PermissionRequest => "PermissionRequest",
        HookEvent::PermissionDenied => "PermissionDenied",
        HookEvent::SessionStart => "SessionStart",
        HookEvent::SessionEnd => "SessionEnd",
        HookEvent::Setup => "Setup",
        HookEvent::Stop => "Stop",
        HookEvent::StopFailure => "StopFailure",
        HookEvent::UserPromptSubmit => "UserPromptSubmit",
        HookEvent::SubagentStart => "SubagentStart",
        HookEvent::SubagentStop => "SubagentStop",
        HookEvent::PreCompact => "PreCompact",
        HookEvent::PostCompact => "PostCompact",
        HookEvent::Notification => "Notification",
        HookEvent::WorktreeCreate => "WorktreeCreate",
        HookEvent::WorktreeRemove => "WorktreeRemove",
        HookEvent::CwdChanged => "CwdChanged",
        HookEvent::FileChanged => "FileChanged",
        HookEvent::TeammateIdle => "TeammateIdle",
        HookEvent::TaskCreated => "TaskCreated",
        HookEvent::TaskCompleted => "TaskCompleted",
        HookEvent::Elicitation => "Elicitation",
        HookEvent::ElicitationResult => "ElicitationResult",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> HookConfig {
        let json = serde_json::json!({
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        { "type": "command", "command": "exit 0", "timeout": 5 }
                    ]
                },
                {
                    "matcher": "Edit",
                    "hooks": [
                        { "type": "command", "command": "echo lint" }
                    ]
                }
            ],
            "SessionStart": [
                {
                    "hooks": [
                        { "type": "command", "command": "echo started" }
                    ]
                }
            ]
        });
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn deserialize_config() {
        let config = sample_config();
        assert_eq!(config.groups_for(HookEvent::PreToolUse).len(), 2);
        assert_eq!(config.groups_for(HookEvent::SessionStart).len(), 1);
        assert_eq!(config.groups_for(HookEvent::PostToolUse).len(), 0);
    }

    #[test]
    fn matching_hooks_by_tool_name() {
        let config = sample_config();
        let hooks = config.matching_hooks(HookEvent::PreToolUse, "Bash", "ls");
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].command, "exit 0");

        let hooks = config.matching_hooks(HookEvent::PreToolUse, "Edit", "/foo.rs");
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].command, "echo lint");

        let hooks = config.matching_hooks(HookEvent::PreToolUse, "Read", "/foo");
        assert!(hooks.is_empty());
    }

    #[test]
    fn session_start_no_matcher() {
        let config = sample_config();
        // SessionStart group has no matcher — should match any source
        let hooks = config.matching_hooks(HookEvent::SessionStart, "startup", "");
        assert_eq!(hooks.len(), 1);
    }

    #[test]
    fn merge_configs() {
        let mut config1 = sample_config();
        let config2: HookConfig = serde_json::from_value(serde_json::json!({
            "PreToolUse": [
                {
                    "matcher": "Write",
                    "hooks": [
                        { "type": "command", "command": "echo write" }
                    ]
                }
            ],
            "Stop": [
                {
                    "hooks": [
                        { "type": "command", "command": "echo done" }
                    ]
                }
            ]
        }))
        .unwrap();

        config1.merge(config2);

        // PreToolUse should now have 3 groups (2 original + 1 merged)
        assert_eq!(config1.groups_for(HookEvent::PreToolUse).len(), 3);
        // Stop should have 1 group
        assert_eq!(config1.groups_for(HookEvent::Stop).len(), 1);
    }

    #[test]
    fn empty_config() {
        let config = HookConfig::default();
        assert!(config.is_empty());
        assert!(config.groups_for(HookEvent::PreToolUse).is_empty());
    }

    #[test]
    fn hook_result_aggregation() {
        let mut agg = AggregatedHookResult::default();

        agg.add(HookResult::Allow {
            stdout: "ok".into(),
        });
        assert!(!agg.blocked);
        assert!(agg.warnings.is_empty());

        agg.add(HookResult::Warn {
            message: "careful".into(),
            stderr: "warn".into(),
        });
        assert!(!agg.blocked);
        assert_eq!(agg.warnings.len(), 1);

        agg.add(HookResult::Block {
            reason: "not allowed".into(),
            stderr: "err".into(),
        });
        assert!(agg.blocked);
        assert_eq!(agg.block_reason.as_deref(), Some("not allowed"));
    }

    #[test]
    fn first_block_wins() {
        let mut agg = AggregatedHookResult::default();
        agg.add(HookResult::Block {
            reason: "first".into(),
            stderr: "".into(),
        });
        agg.add(HookResult::Block {
            reason: "second".into(),
            stderr: "".into(),
        });
        assert_eq!(agg.block_reason.as_deref(), Some("first"));
    }

    #[test]
    fn command_hook_def_timeout() {
        let hook: CommandHookDef =
            serde_json::from_value(serde_json::json!({
                "type": "command",
                "command": "echo hi"
            }))
            .unwrap();
        assert_eq!(hook.timeout, None);

        let hook: CommandHookDef = serde_json::from_value(serde_json::json!({
            "type": "command",
            "command": "echo hi",
            "timeout": 30
        }))
        .unwrap();
        assert_eq!(hook.timeout, Some(30));
    }
}

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// All 25 hook event types matching Claude Code's surface area.
///
/// Events for features not yet built (worktrees, subagents, etc.) are defined
/// but won't fire until those features land. This keeps the event surface
/// stable and forward-compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    // --- Tool lifecycle ---
    /// Before a tool executes. Can block/allow/modify the call.
    PreToolUse,
    /// After a tool executes successfully.
    PostToolUse,
    /// After a tool execution fails with an error.
    PostToolUseFailure,

    // --- Permissions ---
    /// When a tool triggers an "ask" permission prompt.
    PermissionRequest,
    /// When the user denies a permission prompt.
    PermissionDenied,

    // --- Session lifecycle ---
    /// When a new session starts.
    SessionStart,
    /// When a session is shutting down.
    SessionEnd,
    /// On first-time initialization/setup.
    Setup,

    // --- Stop / completion ---
    /// When the model produces a final response (end of turn).
    Stop,
    /// When the model's stop turn fails.
    StopFailure,

    // --- User input ---
    /// When the user submits a message.
    UserPromptSubmit,

    // --- Subagents ---
    /// When a subagent is spawned.
    SubagentStart,
    /// When a subagent completes.
    SubagentStop,

    // --- Compaction ---
    /// Before context window compaction.
    PreCompact,
    /// After context window compaction.
    PostCompact,

    // --- Notifications ---
    /// On system notifications.
    Notification,

    // --- Workspace ---
    /// When a git worktree is created for agent isolation.
    WorktreeCreate,
    /// When a git worktree is cleaned up.
    WorktreeRemove,
    /// When the working directory changes.
    CwdChanged,
    /// When a watched file changes on disk.
    FileChanged,

    // --- Tasks / agents ---
    /// When a teammate agent becomes idle.
    TeammateIdle,
    /// When a task is created.
    TaskCreated,
    /// When a task completes.
    TaskCompleted,

    // --- Elicitation ---
    /// When the model requests structured input from the user.
    Elicitation,
    /// When the user responds to an elicitation.
    ElicitationResult,

    // --- Safety ---
    /// When the prompt-injection guard sanitises or blocks an
    /// inbound text payload.
    PromptInjectionVerdict,
}

impl HookEvent {
    /// All event variants, for iteration.
    pub const ALL: &'static [HookEvent] = &[
        HookEvent::PreToolUse,
        HookEvent::PostToolUse,
        HookEvent::PostToolUseFailure,
        HookEvent::PermissionRequest,
        HookEvent::PermissionDenied,
        HookEvent::SessionStart,
        HookEvent::SessionEnd,
        HookEvent::Setup,
        HookEvent::Stop,
        HookEvent::StopFailure,
        HookEvent::UserPromptSubmit,
        HookEvent::SubagentStart,
        HookEvent::SubagentStop,
        HookEvent::PreCompact,
        HookEvent::PostCompact,
        HookEvent::Notification,
        HookEvent::WorktreeCreate,
        HookEvent::WorktreeRemove,
        HookEvent::CwdChanged,
        HookEvent::FileChanged,
        HookEvent::TeammateIdle,
        HookEvent::TaskCreated,
        HookEvent::TaskCompleted,
        HookEvent::Elicitation,
        HookEvent::ElicitationResult,
        HookEvent::PromptInjectionVerdict,
    ];

    /// Whether this event can block execution (PreToolUse, PermissionRequest, UserPromptSubmit).
    pub fn can_block(&self) -> bool {
        matches!(
            self,
            HookEvent::PreToolUse | HookEvent::PermissionRequest | HookEvent::UserPromptSubmit
        )
    }

    /// The field name that matchers filter on for this event type.
    pub fn matcher_field(&self) -> &'static str {
        match self {
            // Tool events match on tool_name
            HookEvent::PreToolUse
            | HookEvent::PostToolUse
            | HookEvent::PostToolUseFailure
            | HookEvent::PermissionRequest
            | HookEvent::PermissionDenied => "tool_name",

            // Session events match on source (startup, resume, clear, compact)
            HookEvent::SessionStart => "source",

            // Notification matches on notification_type
            HookEvent::Notification => "notification_type",

            // Everything else has no matcher field (hooks fire unconditionally)
            _ => "",
        }
    }

    /// Human-readable summary of when this event fires.
    pub fn summary(&self) -> &'static str {
        match self {
            HookEvent::PreToolUse => "Before tool execution",
            HookEvent::PostToolUse => "After successful tool execution",
            HookEvent::PostToolUseFailure => "After tool execution failure",
            HookEvent::PermissionRequest => "When a tool triggers a permission prompt",
            HookEvent::PermissionDenied => "When the user denies a permission prompt",
            HookEvent::SessionStart => "When a new session starts",
            HookEvent::SessionEnd => "When a session shuts down",
            HookEvent::Setup => "On first-time initialization",
            HookEvent::Stop => "When the model produces a final response",
            HookEvent::StopFailure => "When the model's stop turn fails",
            HookEvent::UserPromptSubmit => "When the user submits a message",
            HookEvent::SubagentStart => "When a subagent is spawned",
            HookEvent::SubagentStop => "When a subagent completes",
            HookEvent::PreCompact => "Before context window compaction",
            HookEvent::PostCompact => "After context window compaction",
            HookEvent::Notification => "On system notifications",
            HookEvent::WorktreeCreate => "When a git worktree is created",
            HookEvent::WorktreeRemove => "When a git worktree is removed",
            HookEvent::CwdChanged => "When the working directory changes",
            HookEvent::FileChanged => "When a watched file changes",
            HookEvent::TeammateIdle => "When a teammate agent becomes idle",
            HookEvent::TaskCreated => "When a task is created",
            HookEvent::TaskCompleted => "When a task completes",
            HookEvent::Elicitation => "When the model requests structured user input",
            HookEvent::ElicitationResult => "When the user responds to an elicitation",
            HookEvent::PromptInjectionVerdict => {
                "When the prompt-injection guard sanitises or blocks inbound text"
            }
        }
    }
}

/// Input data passed to hooks when they fire.
///
/// Each variant carries the fields relevant to its event type.
/// Serialized as JSON and piped to command hooks on stdin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event")]
pub enum HookInput {
    // --- Tool lifecycle ---
    PreToolUse {
        tool_name: String,
        tool_input: serde_json::Value,
    },
    PostToolUse {
        tool_name: String,
        tool_input: serde_json::Value,
        tool_output: String,
    },
    PostToolUseFailure {
        tool_name: String,
        tool_input: serde_json::Value,
        error: String,
    },

    // --- Permissions ---
    PermissionRequest {
        tool_name: String,
        tool_input: serde_json::Value,
        rule: String,
    },
    PermissionDenied {
        tool_name: String,
        tool_input: serde_json::Value,
        reason: String,
    },

    // --- Session lifecycle ---
    SessionStart {
        session_id: String,
        cwd: String,
        /// Source of session start: "startup", "resume", "clear", "compact"
        source: String,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        metadata: HashMap<String, serde_json::Value>,
    },
    SessionEnd {
        session_id: String,
        reason: String,
    },
    Setup {},

    // --- Stop / completion ---
    Stop {
        stop_reason: String,
    },
    StopFailure {
        error: String,
    },

    // --- User input ---
    UserPromptSubmit {
        message: String,
    },

    // --- Subagents ---
    SubagentStart {
        agent_name: String,
        agent_type: String,
        prompt: String,
    },
    SubagentStop {
        agent_name: String,
        result_summary: String,
    },

    // --- Compaction ---
    PreCompact {
        reason: String,
        message_count: usize,
    },
    PostCompact {
        messages_before: usize,
        messages_after: usize,
        tokens_before: usize,
        tokens_after: usize,
    },

    // --- Notifications ---
    Notification {
        notification_type: String,
        message: String,
    },

    // --- Workspace ---
    WorktreeCreate {
        worktree_path: String,
        branch: String,
    },
    WorktreeRemove {
        worktree_path: String,
    },
    CwdChanged {
        old_cwd: String,
        new_cwd: String,
    },
    FileChanged {
        file_path: String,
        change_type: String,
    },

    // --- Tasks / agents ---
    TeammateIdle {
        teammate_id: String,
    },
    TaskCreated {
        task_id: String,
        description: String,
    },
    TaskCompleted {
        task_id: String,
        result: String,
    },

    // --- Elicitation ---
    Elicitation {
        prompt: String,
        schema: serde_json::Value,
    },
    ElicitationResult {
        response: serde_json::Value,
    },

    // --- Safety ---
    /// Fired by inbound-text boundaries when the prompt-injection
    /// guard returns a non-Allow verdict. `context` is the source
    /// tag (e.g. `"web_fetch"`), `verdict` is `"sanitize"` or
    /// `"block"`, and `reasons` lists every finding that fired.
    PromptInjectionVerdict {
        context: String,
        verdict: String,
        reasons: Vec<String>,
    },
}

impl HookInput {
    /// Get the event type for this input.
    pub fn event(&self) -> HookEvent {
        match self {
            HookInput::PreToolUse { .. } => HookEvent::PreToolUse,
            HookInput::PostToolUse { .. } => HookEvent::PostToolUse,
            HookInput::PostToolUseFailure { .. } => HookEvent::PostToolUseFailure,
            HookInput::PermissionRequest { .. } => HookEvent::PermissionRequest,
            HookInput::PermissionDenied { .. } => HookEvent::PermissionDenied,
            HookInput::SessionStart { .. } => HookEvent::SessionStart,
            HookInput::SessionEnd { .. } => HookEvent::SessionEnd,
            HookInput::Setup { .. } => HookEvent::Setup,
            HookInput::Stop { .. } => HookEvent::Stop,
            HookInput::StopFailure { .. } => HookEvent::StopFailure,
            HookInput::UserPromptSubmit { .. } => HookEvent::UserPromptSubmit,
            HookInput::SubagentStart { .. } => HookEvent::SubagentStart,
            HookInput::SubagentStop { .. } => HookEvent::SubagentStop,
            HookInput::PreCompact { .. } => HookEvent::PreCompact,
            HookInput::PostCompact { .. } => HookEvent::PostCompact,
            HookInput::Notification { .. } => HookEvent::Notification,
            HookInput::WorktreeCreate { .. } => HookEvent::WorktreeCreate,
            HookInput::WorktreeRemove { .. } => HookEvent::WorktreeRemove,
            HookInput::CwdChanged { .. } => HookEvent::CwdChanged,
            HookInput::FileChanged { .. } => HookEvent::FileChanged,
            HookInput::TeammateIdle { .. } => HookEvent::TeammateIdle,
            HookInput::TaskCreated { .. } => HookEvent::TaskCreated,
            HookInput::TaskCompleted { .. } => HookEvent::TaskCompleted,
            HookInput::Elicitation { .. } => HookEvent::Elicitation,
            HookInput::ElicitationResult { .. } => HookEvent::ElicitationResult,
            HookInput::PromptInjectionVerdict { .. } => HookEvent::PromptInjectionVerdict,
        }
    }

    /// Get the matcher field value for this input (the value that matchers filter on).
    pub fn matcher_value(&self) -> &str {
        match self {
            HookInput::PreToolUse { tool_name, .. }
            | HookInput::PostToolUse { tool_name, .. }
            | HookInput::PostToolUseFailure { tool_name, .. }
            | HookInput::PermissionRequest { tool_name, .. }
            | HookInput::PermissionDenied { tool_name, .. } => tool_name,
            HookInput::SessionStart { source, .. } => source,
            HookInput::Notification {
                notification_type, ..
            } => notification_type,
            _ => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_events_count() {
        assert_eq!(HookEvent::ALL.len(), 26);
    }

    #[test]
    fn blocking_events() {
        assert!(HookEvent::PreToolUse.can_block());
        assert!(HookEvent::PermissionRequest.can_block());
        assert!(HookEvent::UserPromptSubmit.can_block());
        assert!(!HookEvent::PostToolUse.can_block());
        assert!(!HookEvent::SessionStart.can_block());
        assert!(!HookEvent::Stop.can_block());
    }

    #[test]
    fn hook_input_event_roundtrip() {
        let input = HookInput::PreToolUse {
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({"command": "ls"}),
        };
        assert_eq!(input.event(), HookEvent::PreToolUse);
        assert_eq!(input.matcher_value(), "Bash");
    }

    #[test]
    fn hook_input_serialization() {
        let input = HookInput::PreToolUse {
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"hook_event\":\"PreToolUse\""));
        assert!(json.contains("\"tool_name\":\"Bash\""));

        // Round-trip
        let parsed: HookInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event(), HookEvent::PreToolUse);
    }

    #[test]
    fn session_start_matcher_value() {
        let input = HookInput::SessionStart {
            session_id: "abc".into(),
            cwd: "/tmp".into(),
            source: "startup".into(),
            metadata: HashMap::new(),
        };
        assert_eq!(input.matcher_value(), "startup");
    }

    #[test]
    fn non_matchable_event_returns_empty() {
        let input = HookInput::Stop {
            stop_reason: "end_turn".into(),
        };
        assert_eq!(input.matcher_value(), "");
    }
}

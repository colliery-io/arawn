use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::context::ToolContext;
use crate::error::ToolError;
use crate::llm_preference::LlmPreference;

/// Category of a tool — used for permission checking, context filtering, and
/// tool grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    /// Core tools always included (think, shell, file ops, glob, grep, skill)
    Core,
    /// Task management tools (task_create, task_update, etc.)
    Task,
    /// Agent/sub-agent tools
    Agent,
    /// Web tools (web_fetch, web_search)
    Web,
    /// Memory tools (memory_store, memory_search)
    Memory,
    /// Planning tools (enter_plan_mode, exit_plan_mode)
    Plan,
    /// Workstream management tools
    Workstream,
    /// Always-included utility tools (ask_user, sleep)
    Utility,
    /// Background task management (task_output, task_stop)
    BackgroundTask,
}

/// Risk class of a tool — used by the permission system to decide fallback
/// behaviour when no explicit rule matches. Orthogonal to [`ToolCategory`]
/// (which groups tools by feature area for context filtering).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionCategory {
    /// Side-effect-free / observation only — safe to auto-allow.
    /// These are the only tools permitted in plan mode.
    /// Examples: file_read, glob, grep, think, sleep, web_search, web_fetch.
    ReadOnly,
    /// Modifies files but not the broader system (file_edit, file_write).
    FileWrite,
    /// Executes arbitrary commands — highest risk (shell).
    Shell,
    /// Anything else (agents, memory, etc.) — treated conservatively.
    Other,
}

/// Output from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
}

impl ToolOutput {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
        }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: true,
        }
    }
}

/// A tool that can be invoked by the LLM.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute(
        &self,
        ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError>;

    /// Whether this tool is side-effect-free (observation only).
    fn is_read_only(&self) -> bool {
        false
    }

    /// Tool category for context filtering and feature-area grouping.
    fn category(&self) -> ToolCategory {
        ToolCategory::Core
    }

    /// Permission risk class for permission-mode fallback decisions.
    /// Default is `Other` (conservative — requires explicit allow).
    /// Read-only tools should return `ReadOnly` so they auto-allow in
    /// Default mode and are permitted in Plan mode. File-writing and shell
    /// tools should return their matching variant.
    fn permission_category(&self) -> PermissionCategory {
        // Default: conservative. Tools that are observation-only should
        // override; also tied to `is_read_only()` below — if a tool is
        // read-only it is by definition `PermissionCategory::ReadOnly`.
        if self.is_read_only() {
            PermissionCategory::ReadOnly
        } else {
            PermissionCategory::Other
        }
    }

    /// Optional preferred LLM for this tool. The engine resolves this against
    /// the context's resolver before calling [`Tool::execute`] and makes
    /// the resolved client available via [`ToolContext::preferred_llm`].
    /// Defaults to `None` — most tools don't need an LLM at all.
    fn llm_preference(&self) -> Option<LlmPreference> {
        None
    }
}

// Re-export fidius so plugin crates access it through us (white-label).
// The `crate = "crate"` on plugin_interface makes generated code resolve
// fidius types through `crate::fidius::` — this re-export makes that work.
pub use fidius;
pub use fidius::PluginError;
pub use fidius::plugin_impl;

use serde::{Deserialize, Serialize};

/// The plugin interface for Arawn tools.
///
/// Plugin authors implement this trait via:
/// ```ignore
/// use arawn_tool_plugin::{plugin_impl, ArawnTool, __fidius_ArawnTool, ToolExecuteOutput};
///
/// pub struct MyTool;
///
/// #[plugin_impl(ArawnTool, crate = "arawn_tool_plugin::fidius")]
/// impl ArawnTool for MyTool { ... }
///
/// arawn_tool_plugin::fidius::fidius_plugin_registry!();
/// ```
///
/// Plugin crates depend only on `arawn-tool-plugin` — no direct `fidius` dependency needed.
#[fidius::plugin_interface(version = 1, buffer = PluginAllocated, crate = "crate::fidius")]
pub trait ArawnTool: Send + Sync {
    /// Return the tool's unique name (e.g., "web_fetch").
    fn name(&self) -> String;

    /// Return a human-readable description of what the tool does.
    fn description(&self) -> String;

    /// Return the JSON Schema for the tool's parameters as a JSON string.
    fn parameters_schema(&self) -> String;

    /// Execute the tool with the given context and parameters.
    /// `context_json`: JSON-serialized ToolContext (working_dir, session_id, workstream_name).
    /// `params_json`: JSON-serialized parameters from the LLM.
    fn execute(&self, context_json: String, params_json: String) -> ToolExecuteOutput;
}

/// Output from the `execute` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecuteOutput {
    pub content: String,
    pub is_error: bool,
}

impl ToolExecuteOutput {
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

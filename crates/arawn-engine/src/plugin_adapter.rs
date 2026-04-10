//! DEPRECATED: Legacy WASM plugin adapter.
//!
//! This module is superseded by the new plugin system in `plugins/`.
//! It is gated behind the `legacy-plugins` feature flag and will be
//! removed in a future version.

use async_trait::async_trait;
use fidius_host::PluginHandle;
use serde_json::Value;

use arawn_tool_plugin::ToolExecuteOutput;

use crate::context::ToolContext;
use crate::error::EngineError;
use crate::tool::{Tool, ToolOutput};

/// Adapts a fides PluginHandle into an arawn Tool.
/// Caches name/description/schema on construction so they're not called per-turn.
pub struct PluginToolAdapter {
    handle: PluginHandle,
    cached_name: String,
    cached_description: String,
    cached_schema: Value,
}

impl PluginToolAdapter {
    /// Create an adapter by calling the plugin's metadata methods once.
    pub fn new(handle: PluginHandle) -> Result<Self, EngineError> {
        use arawn_tool_plugin::__fidius_ArawnTool::*;

        // 0-arg methods use () as input with 0.0.5 tuple encoding
        let name: String = handle
            .call_method(METHOD_NAME, &())
            .map_err(|e| EngineError::Tool(format!("plugin name() failed: {e}")))?;

        let description: String = handle
            .call_method(METHOD_DESCRIPTION, &())
            .map_err(|e| EngineError::Tool(format!("plugin description() failed: {e}")))?;

        let schema_str: String = handle
            .call_method(METHOD_PARAMETERS_SCHEMA, &())
            .map_err(|e| EngineError::Tool(format!("plugin parameters_schema() failed: {e}")))?;

        let schema: Value = serde_json::from_str(&schema_str)
            .unwrap_or_else(|_| serde_json::json!({"type": "object", "properties": {}}));

        Ok(Self {
            handle,
            cached_name: name,
            cached_description: description,
            cached_schema: schema,
        })
    }
}

#[async_trait]
impl Tool for PluginToolAdapter {
    fn name(&self) -> &str {
        &self.cached_name
    }

    fn description(&self) -> &str {
        &self.cached_description
    }

    fn parameters_schema(&self) -> Value {
        self.cached_schema.clone()
    }

    async fn execute(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        use arawn_tool_plugin::__fidius_ArawnTool::*;

        let context_json = serde_json::to_string(&ContextForPlugin {
            working_dir: ctx.working_dir.to_string_lossy().to_string(),
            session_id: ctx.session_id.to_string(),
            workstream_name: ctx.workstream_name().to_string(),
        })
        .unwrap_or_default();

        let params_json = serde_json::to_string(&params).unwrap_or_default();

        // Multi-arg: execute(context_json, params_json) encoded as (String, String) tuple
        let result = tokio::task::block_in_place(|| {
            self.handle
                .call_method::<(String, String), ToolExecuteOutput>(
                    METHOD_EXECUTE,
                    &(context_json, params_json),
                )
        });

        match result {
            Ok(output) => {
                if output.is_error {
                    Ok(ToolOutput::error(output.content))
                } else {
                    Ok(ToolOutput::success(output.content))
                }
            }
            Err(e) => Ok(ToolOutput::error(format!("plugin execution error: {e}"))),
        }
    }
}

/// Serializable context sent to plugins across FFI.
#[derive(serde::Serialize)]
struct ContextForPlugin {
    working_dir: String,
    session_id: String,
    workstream_name: String,
}

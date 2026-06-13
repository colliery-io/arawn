//! McpToolAdapter — wraps an MCP tool as an arawn Tool impl.

use std::sync::Arc;

use async_trait::async_trait;
use rmcp::model::{CallToolRequestParams, RawContent, Tool as McpTool};
use rmcp::service::{Peer, RoleClient};
use serde_json::Value;
use tracing::{debug, warn};

use arawn_tool::{Tool, ToolError, ToolOutput};

/// An arawn Tool backed by an MCP server tool.
pub struct McpToolAdapter {
    /// Full arawn tool name: mcp__<server>__<tool>
    arawn_name: String,
    /// Original MCP tool name (for calling the server)
    mcp_name: String,
    /// MCP tool metadata (description, input schema)
    mcp_tool: McpTool,
    /// The connected MCP client peer for calling tools
    peer: Arc<Peer<RoleClient>>,
}

impl McpToolAdapter {
    pub fn new(server_name: &str, mcp_tool: McpTool, peer: Arc<Peer<RoleClient>>) -> Self {
        let normalized_server = normalize_name(server_name);
        let normalized_tool = normalize_name(&mcp_tool.name);
        let arawn_name = format!("mcp__{normalized_server}__{normalized_tool}");
        let mcp_name = mcp_tool.name.to_string();

        Self {
            arawn_name,
            mcp_name,
            mcp_tool,
            peer,
        }
    }

    /// Get the arawn tool name (for logging before registration).
    pub fn tool_name(&self) -> &str {
        &self.arawn_name
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.arawn_name
    }

    fn description(&self) -> &str {
        self.mcp_tool.description.as_deref().unwrap_or("MCP tool")
    }

    fn parameters_schema(&self) -> Value {
        serde_json::to_value(&self.mcp_tool.input_schema).unwrap_or_else(|_| {
            serde_json::json!({
                "type": "object",
                "properties": {}
            })
        })
    }

    fn is_read_only(&self) -> bool {
        self.mcp_tool
            .annotations
            .as_ref()
            .and_then(|a| a.read_only_hint)
            .unwrap_or(false)
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        debug!(tool = %self.arawn_name, mcp_name = %self.mcp_name, "calling MCP tool");

        let mut request = CallToolRequestParams::new(self.mcp_name.clone());
        if let Some(obj) = params.as_object()
            && !obj.is_empty()
        {
            request = request.with_arguments(obj.clone());
        }

        match self.peer.call_tool(request).await {
            Ok(result) => {
                // Convert MCP content to string
                let content: String = result
                    .content
                    .iter()
                    .map(|c| match &c.raw {
                        RawContent::Text(text) => text.text.clone(),
                        RawContent::Image(_) => "[Image content]".to_string(),
                        RawContent::Audio(_) => "[Audio content]".to_string(),
                        RawContent::Resource(res) => {
                            // Try to extract text from embedded resource
                            serde_json::to_string(res).unwrap_or_else(|_| "[Resource]".to_string())
                        }
                        RawContent::ResourceLink(link) => {
                            format!("[Resource: {}]", link.uri)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                // A tool that ran but reported failure (`is_error`) is a
                // *tool* error — surface it as Ok(ToolOutput::error) so the
                // content (the tool's own message) flows back to the model.
                let is_error = result.is_error.unwrap_or(false);
                if is_error {
                    Ok(ToolOutput::error(content))
                } else {
                    Ok(ToolOutput::success(content))
                }
            }
            // The RPC itself failed (server crashed, transport dropped, bad
            // framing) — the tool never ran. That's distinct from a tool
            // reporting failure: return Err so the engine routes it through the
            // failure path (PostToolUseFailure hook, distinct logging) instead
            // of presenting a transport outage as ordinary tool output.
            Err(e) => {
                warn!(tool = %self.arawn_name, error = %e, "MCP tool call transport/RPC failure");
                Err(ToolError::ExecutionFailed(format!(
                    "MCP transport/RPC error calling '{}': {e}",
                    self.mcp_name
                )))
            }
        }
    }
}

/// Normalize a name for use in tool naming — replace non-alphanumeric chars with _
fn normalize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_simple() {
        assert_eq!(normalize_name("sqlite"), "sqlite");
        assert_eq!(normalize_name("my-server"), "my-server");
    }

    #[test]
    fn normalize_special_chars() {
        assert_eq!(normalize_name("my.server"), "my_server");
        assert_eq!(normalize_name("my server"), "my_server");
        assert_eq!(normalize_name("@org/pkg"), "_org_pkg");
    }
}

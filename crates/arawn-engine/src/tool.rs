use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::context::ToolContext;
use crate::error::EngineError;

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
    async fn execute(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>;

    /// Whether this tool is side-effect-free (observation only). Side-effect-free
    /// tools can be executed concurrently when the LLM requests multiple tool calls
    /// in one turn, and are the only tools allowed during plan mode.
    /// Examples: reading files, searching code, querying APIs, thinking.
    /// Default: false (conservative — assumes side effects).
    fn is_read_only(&self) -> bool {
        false
    }
}

/// Registry of available tools. Supports hot-reload via register/unregister at runtime.
/// The engine queries this fresh each turn so changes take effect immediately.
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
    /// Names of tools loaded from plugins (vs built-in tools).
    plugin_tools: RwLock<HashSet<String>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            plugin_tools: RwLock::new(HashSet::new()),
        }
    }

    /// Register a built-in tool.
    pub fn register(&self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.write().unwrap().insert(name, Arc::from(tool));
    }

    /// Register a plugin-provided tool (tracked for hot-reload).
    pub fn register_plugin(&self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools
            .write()
            .unwrap()
            .insert(name.clone(), Arc::from(tool));
        self.plugin_tools.write().unwrap().insert(name);
    }

    /// Register an already-Arc'd tool (used when building filtered registries).
    pub fn register_arc(&self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.write().unwrap().insert(name, tool);
    }

    pub fn unregister(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.plugin_tools.write().unwrap().remove(name);
        self.tools.write().unwrap().remove(name)
    }

    /// Returns the names of all currently loaded plugin tools.
    pub fn plugin_tool_names(&self) -> Vec<String> {
        self.plugin_tools.read().unwrap().iter().cloned().collect()
    }

    /// Get a tool by name. Returns a cloned Arc — no lock held after return.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().unwrap().get(name).cloned()
    }

    pub fn tool_definitions(&self) -> Vec<arawn_llm::ToolDefinition> {
        let tools = self.tools.read().unwrap();
        tools
            .values()
            .map(|t| arawn_llm::ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.tools.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Unregister all tools whose names start with the given prefix.
    /// Used for bulk removal of MCP server tools (e.g. `mcp__server_name__`).
    pub fn unregister_by_prefix(&self, prefix: &str) -> Vec<String> {
        let names: Vec<String> = self
            .tools
            .read()
            .unwrap()
            .keys()
            .filter(|name| name.starts_with(prefix))
            .cloned()
            .collect();

        for name in &names {
            self.unregister(name);
        }

        names
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    /// A minimal test tool for unit testing the registry.
    struct DummyTool {
        tool_name: String,
    }

    impl DummyTool {
        fn new(name: &str) -> Self {
            Self {
                tool_name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &str {
            &self.tool_name
        }

        fn description(&self) -> &str {
            "A dummy tool for testing"
        }

        fn parameters_schema(&self) -> Value {
            json!({"type": "object", "properties": {}})
        }

        async fn execute(
            &self,
            _ctx: &ToolContext,
            _params: Value,
        ) -> Result<ToolOutput, EngineError> {
            Ok(ToolOutput::success("dummy result"))
        }
    }

    #[test]
    fn registry_starts_empty() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn register_and_get_tool() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(DummyTool::new("test_tool")));

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let tool = registry.get("test_tool");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "test_tool");
    }

    #[test]
    fn get_nonexistent_tool_returns_none() {
        let registry = ToolRegistry::new();
        assert!(registry.get("nope").is_none());
    }

    #[test]
    fn unregister_tool() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(DummyTool::new("removable")));
        assert_eq!(registry.len(), 1);

        let removed = registry.unregister("removable");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name(), "removable");
        assert!(registry.is_empty());
    }

    #[test]
    fn unregister_nonexistent_returns_none() {
        let registry = ToolRegistry::new();
        assert!(registry.unregister("nope").is_none());
    }

    #[test]
    fn hot_reload_register_unregister_cycle() {
        let registry = ToolRegistry::new();

        // Register two tools
        registry.register(Box::new(DummyTool::new("tool_a")));
        registry.register(Box::new(DummyTool::new("tool_b")));
        assert_eq!(registry.len(), 2);

        // Unregister one
        registry.unregister("tool_a");
        assert_eq!(registry.len(), 1);
        assert!(registry.get("tool_a").is_none());
        assert!(registry.get("tool_b").is_some());

        // Register a new one
        registry.register(Box::new(DummyTool::new("tool_c")));
        assert_eq!(registry.len(), 2);
        assert!(registry.get("tool_c").is_some());
    }

    #[test]
    fn tool_definitions_reflects_registered_tools() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(DummyTool::new("alpha")));
        registry.register(Box::new(DummyTool::new("beta")));

        let defs = registry.tool_definitions();
        assert_eq!(defs.len(), 2);

        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
    }

    #[test]
    fn tool_definitions_updates_after_unregister() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(DummyTool::new("keep")));
        registry.register(Box::new(DummyTool::new("remove")));
        registry.unregister("remove");

        let defs = registry.tool_definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "keep");
    }

    #[test]
    fn registry_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ToolRegistry>();
    }

    #[test]
    fn concurrent_access() {
        let registry = Arc::new(ToolRegistry::new());
        registry.register(Box::new(DummyTool::new("shared")));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let reg = Arc::clone(&registry);
                std::thread::spawn(move || {
                    let tool = reg.get("shared");
                    assert!(tool.is_some());
                    assert_eq!(tool.unwrap().name(), "shared");
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn unregister_by_prefix_removes_matching() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(DummyTool::new("mcp__sqlite__query")));
        registry.register(Box::new(DummyTool::new("mcp__sqlite__list")));
        registry.register(Box::new(DummyTool::new("mcp__github__pr")));
        registry.register(Box::new(DummyTool::new("Read")));
        assert_eq!(registry.len(), 4);

        let removed = registry.unregister_by_prefix("mcp__sqlite__");
        assert_eq!(removed.len(), 2);
        assert_eq!(registry.len(), 2);
        assert!(registry.get("mcp__sqlite__query").is_none());
        assert!(registry.get("mcp__sqlite__list").is_none());
        assert!(registry.get("mcp__github__pr").is_some());
        assert!(registry.get("Read").is_some());
    }

    #[test]
    fn unregister_by_prefix_no_match() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(DummyTool::new("Read")));
        let removed = registry.unregister_by_prefix("mcp__");
        assert!(removed.is_empty());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn tool_output_success() {
        let output = ToolOutput::success("hello");
        assert_eq!(output.content, "hello");
        assert!(!output.is_error);
    }

    #[test]
    fn tool_output_error() {
        let output = ToolOutput::error("bad thing");
        assert_eq!(output.content, "bad thing");
        assert!(output.is_error);
    }
}

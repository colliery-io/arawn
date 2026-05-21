use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use crate::tool::Tool;

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
                parameters: inject_timeout_secs(t.parameters_schema()),
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

/// Inject an optional `timeout_secs` property into a tool's JSON schema so
/// the LLM knows it can override the per-call wall-clock budget. The actual
/// enforcement lives in `arawn_engine::tool_timeout`; this only shapes what
/// the model sees.
///
/// The injected property is intentionally optional. Tools that already
/// define a `timeout_secs` field of their own (none today) keep their
/// existing definition.
fn inject_timeout_secs(mut schema: serde_json::Value) -> serde_json::Value {
    use serde_json::{Map, Value, json};

    let Some(obj) = schema.as_object_mut() else {
        return schema;
    };

    let props = obj
        .entry("properties")
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(props_obj) = props.as_object_mut() {
        props_obj.entry("timeout_secs").or_insert_with(|| {
            json!({
                "type": "integer",
                "minimum": 1,
                "description": "Optional wall-clock timeout for this call, in seconds. \
                                If omitted, the engine default (120s, configurable) applies. \
                                Pass a larger value for long-running commands (builds, large fetches); \
                                pass a smaller value to fail fast."
            })
        });
    }

    // If the schema explicitly forbids extra properties, we still want the
    // injected field to validate. Setting it to true is fine because the
    // field is now in `properties`.
    if matches!(obj.get("additionalProperties"), Some(Value::Bool(false))) {
        obj.insert("additionalProperties".into(), Value::Bool(true));
    }

    schema
}

#[cfg(test)]
mod injection_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn injects_into_empty_object_schema() {
        let injected = inject_timeout_secs(json!({"type": "object"}));
        let props = injected["properties"].as_object().expect("properties");
        assert!(props.contains_key("timeout_secs"));
        assert_eq!(props["timeout_secs"]["type"], "integer");
        assert_eq!(props["timeout_secs"]["minimum"], 1);
    }

    #[test]
    fn preserves_existing_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            },
            "required": ["path"]
        });
        let injected = inject_timeout_secs(schema);
        let props = injected["properties"].as_object().unwrap();
        assert!(props.contains_key("path"));
        assert!(props.contains_key("timeout_secs"));
        assert_eq!(injected["required"], json!(["path"]));
    }

    #[test]
    fn does_not_overwrite_existing_timeout_secs() {
        let schema = json!({
            "type": "object",
            "properties": {
                "timeout_secs": {"type": "string", "description": "custom"}
            }
        });
        let injected = inject_timeout_secs(schema);
        assert_eq!(injected["properties"]["timeout_secs"]["type"], "string");
        assert_eq!(
            injected["properties"]["timeout_secs"]["description"],
            "custom"
        );
    }

    #[test]
    fn relaxes_additional_properties_false() {
        let schema = json!({
            "type": "object",
            "properties": {"foo": {"type": "string"}},
            "additionalProperties": false
        });
        let injected = inject_timeout_secs(schema);
        assert_eq!(injected["additionalProperties"], true);
        assert!(injected["properties"]["timeout_secs"].is_object());
    }

    #[test]
    fn non_object_schema_passes_through() {
        // Some tools might return a bare type=string schema or an array.
        let schema = json!("not an object");
        let injected = inject_timeout_secs(schema.clone());
        assert_eq!(injected, schema);
    }
}

#[cfg(test)]
mod registry_tests {
    use super::*;
    use crate::error::ToolError;
    use crate::tool::ToolOutput;
    use async_trait::async_trait;
    use serde_json::{Value, json};
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
            _ctx: &dyn crate::ToolContext,
            _params: Value,
        ) -> Result<ToolOutput, ToolError> {
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

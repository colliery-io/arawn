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

// Re-export tool types from the arawn-tool crate.
// The canonical definitions now live in arawn-tool; this module provides
// backward-compatible re-exports so downstream code can still do
// `use arawn_engine::tool::{Tool, ToolOutput, ...}`.

pub use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput, ToolRegistry};

#[cfg(test)]
mod tests {
    use super::*;
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
            _ctx: &dyn arawn_tool::ToolContext,
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

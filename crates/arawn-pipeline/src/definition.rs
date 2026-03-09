//! Declarative workflow definition parser.
//!
//! Parses TOML workflow files into `WorkflowDefinition` structs, validates them,
//! and converts them to Cloacina workflows via the core builder API.
//!
//! # Example TOML
//!
//! ```toml
//! [workflow]
//! name = "session_indexing"
//! description = "Post-session entity extraction and summarization"
//!
//! [[workflow.tasks]]
//! id = "extract_entities"
//! action = { type = "tool", name = "llm_generate", params = { prompt = "Extract from: {{input.text}}" } }
//! retry_attempts = 2
//!
//! [[workflow.tasks]]
//! id = "store_results"
//! action = { type = "tool", name = "memory_store" }
//! dependencies = ["extract_entities"]
//!
//! [workflow.schedule]
//! cron = "0 9 * * *"
//! timezone = "America/New_York"
//!
//! [workflow.runtime]
//! timeout_secs = 300
//! max_retries = 3
//!
//! [workflow.triggers]
//! on_event = "session_close"
//! ```

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::error::PipelineError;
use crate::task::{DynamicTask, TaskFn};

/// Top-level wrapper matching the TOML structure `[workflow]`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowFile {
    pub workflow: WorkflowDefinition,
}

/// A complete declarative workflow definition.
///
/// # Examples
///
/// ```rust,ignore
/// use arawn_pipeline::WorkflowFile;
///
/// let toml = r#"
/// [workflow]
/// name = "indexing"
/// [[workflow.tasks]]
/// id = "extract"
/// action = { type = "tool", name = "llm_generate" }
/// "#;
/// let wf = WorkflowFile::from_toml(toml)?;
/// wf.workflow.validate()?;
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowDefinition {
    /// Unique workflow name.
    pub name: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: String,

    /// Ordered list of tasks in this workflow.
    pub tasks: Vec<TaskDefinition>,

    /// Optional cron/schedule configuration.
    #[serde(default)]
    pub schedule: Option<ScheduleConfig>,

    /// Optional runtime configuration.
    #[serde(default)]
    pub runtime: Option<RuntimeConfig>,

    /// Optional trigger configuration.
    #[serde(default)]
    pub triggers: Option<TriggerConfig>,
}

/// A single task within a workflow.
///
/// Supports two schemas:
/// - **New (runtime)**: `runtime = "http"`, `config = { url = "..." }`
/// - **Legacy (action)**: `action = { type = "tool", name = "echo" }`
///
/// When both are present, `runtime`/`config` take precedence.
///
/// # Examples
///
/// ```rust,ignore
/// use arawn_pipeline::TaskDefinition;
///
/// // runtime schema
/// let task_toml = r#"
/// id = "fetch"
/// runtime = "http"
/// config = { url = "https://example.com" }
/// "#;
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskDefinition {
    /// Unique task identifier within this workflow.
    pub id: String,

    /// Legacy action definition. Optional when `runtime` is provided.
    #[serde(default)]
    pub action: Option<ActionDefinition>,

    /// WASM runtime name (e.g., "http", "file_read", "passthrough").
    #[serde(default)]
    pub runtime: Option<String>,

    /// Runtime-specific configuration passed to the WASM module.
    #[serde(default)]
    pub config: Option<serde_json::Value>,

    /// IDs of tasks that must complete before this one runs.
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Number of retry attempts on failure.
    #[serde(default)]
    pub retry_attempts: Option<u32>,

    /// Delay between retries in milliseconds.
    #[serde(default)]
    pub retry_delay_ms: Option<u64>,

    /// WASI capability grants for script actions.
    #[serde(default)]
    pub capabilities: Option<Capabilities>,
}

impl TaskDefinition {
    /// Returns the effective runtime name.
    ///
    /// If `runtime` is set, returns it directly. Otherwise derives from `action`:
    /// - `Tool { name, .. }` → name
    /// - `Script { .. }` → "script"
    /// - `Llm { .. }` → "llm"
    pub fn effective_runtime(&self) -> Option<&str> {
        if let Some(ref rt) = self.runtime {
            return Some(rt.as_str());
        }
        match &self.action {
            Some(ActionDefinition::Tool { name, .. }) => Some(name.as_str()),
            Some(ActionDefinition::Script { .. }) => Some("script"),
            Some(ActionDefinition::Llm { .. }) => Some("llm"),
            None => None,
        }
    }

    /// Returns the effective config value.
    ///
    /// If `config` is set, returns it. Otherwise derives from `action.params`.
    pub fn effective_config(&self) -> serde_json::Value {
        if let Some(ref cfg) = self.config {
            return cfg.clone();
        }
        match &self.action {
            Some(ActionDefinition::Tool { params, .. }) => {
                serde_json::to_value(params).unwrap_or_default()
            }
            Some(ActionDefinition::Script {
                source_file,
                language,
            }) => {
                serde_json::json!({"source_file": source_file, "language": language})
            }
            Some(ActionDefinition::Llm { prompt, model }) => {
                serde_json::json!({"prompt": prompt, "model": model})
            }
            None => serde_json::Value::Object(Default::default()),
        }
    }
}

/// What a task actually does.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionDefinition {
    /// Invoke an existing Arawn tool.
    Tool {
        /// Tool name (e.g., "llm_generate", "memory_store").
        name: String,
        /// Parameters passed to the tool.
        #[serde(default)]
        params: HashMap<String, serde_json::Value>,
    },

    /// Execute a Rust script in the Wasmtime sandbox.
    Script {
        /// Source file path (relative to workflow directory).
        source_file: String,
        /// Script language (currently only "rust").
        #[serde(default = "default_script_language")]
        language: String,
    },

    /// Direct LLM call with prompt template.
    Llm {
        /// Prompt template with `{{context.field}}` expressions.
        prompt: String,
        /// Optional model override.
        #[serde(default)]
        model: Option<String>,
    },
}

fn default_script_language() -> String {
    "rust".to_string()
}

/// WASI capability grants for sandboxed script execution.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Capabilities {
    /// Filesystem paths the script can access.
    #[serde(default)]
    pub filesystem: Vec<String>,
    /// Whether the script can make network requests.
    #[serde(default)]
    pub network: bool,
}

/// Cron/schedule configuration for a workflow.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScheduleConfig {
    /// Cron expression (e.g., "0 9 * * *").
    pub cron: String,
    /// IANA timezone (e.g., "America/New_York").
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

/// Runtime configuration for a workflow.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeConfig {
    /// Maximum execution time in seconds.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// Maximum retries for the entire workflow.
    #[serde(default)]
    pub max_retries: Option<u32>,
}

/// Trigger configuration for event-driven execution.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TriggerConfig {
    /// Event name that triggers this workflow (e.g., "session_close").
    pub on_event: String,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

impl WorkflowFile {
    /// Parse a workflow definition from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, PipelineError> {
        toml::from_str(toml_str)
            .map_err(|e| PipelineError::InvalidWorkflow(format!("TOML parse error: {}", e)))
    }

    /// Load a workflow definition from a file path.
    pub fn from_file(path: &Path) -> Result<Self, PipelineError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            PipelineError::InvalidWorkflow(format!("Failed to read {}: {}", path.display(), e))
        })?;
        Self::from_toml(&content)
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

impl WorkflowDefinition {
    /// Validate the workflow definition.
    ///
    /// Checks:
    /// - At least one task
    /// - No duplicate task IDs
    /// - All dependency references point to existing tasks
    /// - No cycles in the dependency graph
    /// - Script actions have valid language
    pub fn validate(&self) -> Result<(), PipelineError> {
        if self.name.is_empty() {
            return Err(PipelineError::InvalidWorkflow(
                "Workflow name cannot be empty".into(),
            ));
        }

        if self.tasks.is_empty() {
            return Err(PipelineError::InvalidWorkflow(
                "Workflow must have at least one task".into(),
            ));
        }

        // Check for duplicate task IDs
        let mut seen_ids = HashSet::new();
        for task in &self.tasks {
            if task.id.is_empty() {
                return Err(PipelineError::InvalidWorkflow(
                    "Task ID cannot be empty".into(),
                ));
            }
            if !seen_ids.insert(&task.id) {
                return Err(PipelineError::InvalidWorkflow(format!(
                    "Duplicate task ID: {}",
                    task.id
                )));
            }
        }

        // Check all dependencies reference existing tasks
        for task in &self.tasks {
            for dep in &task.dependencies {
                if !seen_ids.contains(dep) {
                    return Err(PipelineError::InvalidWorkflow(format!(
                        "Task '{}' depends on unknown task '{}'",
                        task.id, dep
                    )));
                }
            }
        }

        // Cycle detection via topological sort (Kahn's algorithm)
        self.detect_cycles()?;

        // Each task must have either `runtime` or `action`
        for task in &self.tasks {
            if task.runtime.is_none() && task.action.is_none() {
                return Err(PipelineError::InvalidWorkflow(format!(
                    "Task '{}' must have either 'runtime' or 'action'",
                    task.id
                )));
            }
        }

        // Validate action-specific constraints
        for task in &self.tasks {
            if let Some(ref action) = task.action
                && let ActionDefinition::Script { language, .. } = action
                && language != "rust"
            {
                return Err(PipelineError::InvalidWorkflow(format!(
                    "Unsupported script language '{}' in task '{}'. Only 'rust' is supported.",
                    language, task.id
                )));
            }
        }

        Ok(())
    }

    /// Detect cycles in the task dependency graph using Kahn's algorithm.
    fn detect_cycles(&self) -> Result<(), PipelineError> {
        let task_ids: Vec<&str> = self.tasks.iter().map(|t| t.id.as_str()).collect();
        let id_to_idx: HashMap<&str, usize> = task_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();

        let n = task_ids.len();
        let mut in_degree = vec![0usize; n];
        let mut adj: Vec<Vec<usize>> = vec![vec![]; n];

        for task in &self.tasks {
            let idx = id_to_idx[task.id.as_str()];
            for dep in &task.dependencies {
                let dep_idx = id_to_idx[dep.as_str()];
                adj[dep_idx].push(idx);
                in_degree[idx] += 1;
            }
        }

        let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
        let mut visited = 0;

        while let Some(node) = queue.pop() {
            visited += 1;
            for &neighbor in &adj[node] {
                in_degree[neighbor] -= 1;
                if in_degree[neighbor] == 0 {
                    queue.push(neighbor);
                }
            }
        }

        if visited != n {
            return Err(PipelineError::InvalidWorkflow(
                "Cycle detected in task dependencies".into(),
            ));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Conversion to Cloacina
// ---------------------------------------------------------------------------

/// Type alias for a factory that produces a `TaskFn` from an `ActionDefinition`.
///
/// This allows the caller to plug in their own action execution logic
/// (tool dispatch, script sandbox, LLM calls) without the definition module
/// knowing the details.
pub type ActionExecutorFactory = Arc<dyn Fn(&str, &ActionDefinition) -> TaskFn + Send + Sync>;

impl WorkflowDefinition {
    /// Convert this declarative definition into Cloacina `DynamicTask`s.
    ///
    /// The `executor_factory` is called for each task to produce the actual
    /// execution function based on the action type. This decouples parsing
    /// from execution — the definition module doesn't know how to run tools,
    /// scripts, or LLM calls.
    pub fn to_dynamic_tasks(
        &self,
        executor_factory: &ActionExecutorFactory,
    ) -> Result<Vec<DynamicTask>, PipelineError> {
        self.validate()?;

        let mut tasks = Vec::with_capacity(self.tasks.len());

        for task_def in &self.tasks {
            // Build a synthetic ActionDefinition for runtime-only tasks
            // so the factory has something to work with.
            let synthetic_action;
            let action_ref = if let Some(ref action) = task_def.action {
                action
            } else {
                // runtime-only task: synthesize a Tool action from runtime + config
                let runtime_name = task_def.runtime.as_deref().unwrap_or("passthrough");
                let params = match task_def.config.as_ref() {
                    Some(Value::Object(map)) => {
                        map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                    }
                    _ => HashMap::new(),
                };
                synthetic_action = ActionDefinition::Tool {
                    name: runtime_name.to_string(),
                    params,
                };
                &synthetic_action
            };
            let execute_fn = executor_factory(&task_def.id, action_ref);

            let mut dynamic_task = DynamicTask::new(&task_def.id, execute_fn);

            // Add dependencies
            for dep_id in &task_def.dependencies {
                dynamic_task = dynamic_task.with_dependency_id(dep_id);
            }

            // Configure retry policy
            if let Some(attempts) = task_def.retry_attempts {
                let delay = task_def.retry_delay_ms.unwrap_or(1000);
                let policy = cloacina_workflow::retry::RetryPolicy {
                    max_attempts: attempts as i32,
                    initial_delay: std::time::Duration::from_millis(delay),
                    ..Default::default()
                };
                dynamic_task = dynamic_task.with_retry_policy(policy);
            }

            tasks.push(dynamic_task);
        }

        debug!(
            "Converted workflow '{}' to {} dynamic tasks",
            self.name,
            tasks.len()
        );

        Ok(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_WORKFLOW: &str = r#"
[workflow]
name = "test_workflow"
description = "A test workflow"

[[workflow.tasks]]
id = "step_a"
action = { type = "tool", name = "echo" }

[[workflow.tasks]]
id = "step_b"
action = { type = "tool", name = "echo" }
dependencies = ["step_a"]

[workflow.schedule]
cron = "0 9 * * *"
timezone = "America/New_York"

[workflow.runtime]
timeout_secs = 300
max_retries = 3

[workflow.triggers]
on_event = "session_close"
"#;

    #[test]
    fn test_parse_valid_workflow() {
        let wf = WorkflowFile::from_toml(VALID_WORKFLOW).unwrap();
        assert_eq!(wf.workflow.name, "test_workflow");
        assert_eq!(wf.workflow.description, "A test workflow");
        assert_eq!(wf.workflow.tasks.len(), 2);
        assert_eq!(wf.workflow.tasks[0].id, "step_a");
        assert_eq!(wf.workflow.tasks[1].dependencies, vec!["step_a"]);

        let schedule = wf.workflow.schedule.unwrap();
        assert_eq!(schedule.cron, "0 9 * * *");
        assert_eq!(schedule.timezone, "America/New_York");

        let runtime = wf.workflow.runtime.unwrap();
        assert_eq!(runtime.timeout_secs, Some(300));
        assert_eq!(runtime.max_retries, Some(3));

        let triggers = wf.workflow.triggers.unwrap();
        assert_eq!(triggers.on_event, "session_close");
    }

    #[test]
    fn test_parse_tool_action() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "tool", name = "web_fetch", params = { url = "https://example.com" } }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        match wf.workflow.tasks[0].action.as_ref().unwrap() {
            ActionDefinition::Tool { name, params } => {
                assert_eq!(name, "web_fetch");
                assert_eq!(params["url"], "https://example.com");
            }
            _ => panic!("Expected Tool action"),
        }
    }

    #[test]
    fn test_parse_script_action() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "script", source_file = "scripts/process.rs" }
capabilities = { filesystem = ["/tmp/sandbox"], network = false }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        match wf.workflow.tasks[0].action.as_ref().unwrap() {
            ActionDefinition::Script {
                source_file,
                language,
            } => {
                assert_eq!(source_file, "scripts/process.rs");
                assert_eq!(language, "rust");
            }
            _ => panic!("Expected Script action"),
        }
        let caps = wf.workflow.tasks[0].capabilities.as_ref().unwrap();
        assert_eq!(caps.filesystem, vec!["/tmp/sandbox"]);
        assert!(!caps.network);
    }

    #[test]
    fn test_parse_llm_action() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "llm", prompt = "Summarize: {{input.text}}" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        match wf.workflow.tasks[0].action.as_ref().unwrap() {
            ActionDefinition::Llm { prompt, model } => {
                assert_eq!(prompt, "Summarize: {{input.text}}");
                assert!(model.is_none());
            }
            _ => panic!("Expected Llm action"),
        }
    }

    #[test]
    fn test_validate_empty_name() {
        let toml = r#"
[workflow]
name = ""
[[workflow.tasks]]
id = "t1"
action = { type = "tool", name = "echo" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        assert!(wf.workflow.validate().is_err());
    }

    #[test]
    fn test_validate_no_tasks() {
        let toml = r#"
[workflow]
name = "empty"
tasks = []
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        assert!(wf.workflow.validate().is_err());
    }

    #[test]
    fn test_validate_duplicate_task_ids() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "dup"
action = { type = "tool", name = "a" }
[[workflow.tasks]]
id = "dup"
action = { type = "tool", name = "b" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let err = wf.workflow.validate().unwrap_err();
        assert!(err.to_string().contains("Duplicate task ID"));
    }

    #[test]
    fn test_validate_unknown_dependency() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "tool", name = "a" }
dependencies = ["nonexistent"]
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let err = wf.workflow.validate().unwrap_err();
        assert!(err.to_string().contains("unknown task"));
    }

    #[test]
    fn test_validate_cycle_detection() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "a"
action = { type = "tool", name = "x" }
dependencies = ["b"]
[[workflow.tasks]]
id = "b"
action = { type = "tool", name = "x" }
dependencies = ["a"]
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let err = wf.workflow.validate().unwrap_err();
        assert!(err.to_string().contains("Cycle detected"));
    }

    #[test]
    fn test_validate_self_cycle() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "a"
action = { type = "tool", name = "x" }
dependencies = ["a"]
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let err = wf.workflow.validate().unwrap_err();
        assert!(err.to_string().contains("Cycle detected"));
    }

    #[test]
    fn test_validate_unsupported_script_language() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "script", source_file = "test.py", language = "python" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let err = wf.workflow.validate().unwrap_err();
        assert!(err.to_string().contains("Unsupported script language"));
    }

    #[test]
    fn test_valid_workflow_validates() {
        let wf = WorkflowFile::from_toml(VALID_WORKFLOW).unwrap();
        wf.workflow.validate().unwrap();
    }

    #[test]
    fn test_to_dynamic_tasks() {
        let wf = WorkflowFile::from_toml(VALID_WORKFLOW).unwrap();

        // Stub executor factory — just returns a no-op
        let factory: ActionExecutorFactory =
            Arc::new(|_id, _action| Arc::new(|ctx| Box::pin(async move { Ok(ctx) })));

        let tasks = wf.workflow.to_dynamic_tasks(&factory).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_to_dynamic_tasks_with_retry() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "tool", name = "echo" }
retry_attempts = 5
retry_delay_ms = 2000
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();

        let factory: ActionExecutorFactory =
            Arc::new(|_id, _action| Arc::new(|ctx| Box::pin(async move { Ok(ctx) })));

        let tasks = wf.workflow.to_dynamic_tasks(&factory).unwrap();
        assert_eq!(tasks.len(), 1);
        // RetryPolicy is internal to DynamicTask, verified via Cloacina trait
    }

    #[test]
    fn test_roundtrip_serialize() {
        let wf = WorkflowFile::from_toml(VALID_WORKFLOW).unwrap();
        let serialized = toml::to_string_pretty(&wf).unwrap();
        let parsed_back = WorkflowFile::from_toml(&serialized).unwrap();
        assert_eq!(wf.workflow.name, parsed_back.workflow.name);
        assert_eq!(wf.workflow.tasks.len(), parsed_back.workflow.tasks.len());
    }

    #[test]
    fn test_minimal_workflow() {
        let toml = r#"
[workflow]
name = "minimal"
[[workflow.tasks]]
id = "only_task"
action = { type = "tool", name = "noop" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        wf.workflow.validate().unwrap();
        assert!(wf.workflow.schedule.is_none());
        assert!(wf.workflow.runtime.is_none());
        assert!(wf.workflow.triggers.is_none());
    }

    #[test]
    fn test_complex_dag() {
        let toml = r#"
[workflow]
name = "diamond"
[[workflow.tasks]]
id = "start"
action = { type = "tool", name = "a" }
[[workflow.tasks]]
id = "left"
action = { type = "tool", name = "b" }
dependencies = ["start"]
[[workflow.tasks]]
id = "right"
action = { type = "tool", name = "c" }
dependencies = ["start"]
[[workflow.tasks]]
id = "join"
action = { type = "tool", name = "d" }
dependencies = ["left", "right"]
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        wf.workflow.validate().unwrap();
    }

    #[test]
    fn test_invalid_toml_syntax() {
        let bad_toml = "this is not valid toml {{{";
        assert!(WorkflowFile::from_toml(bad_toml).is_err());
    }

    // --- New runtime schema tests ---

    #[test]
    fn test_parse_runtime_schema() {
        let toml = r#"
[workflow]
name = "runtime_test"
[[workflow.tasks]]
id = "fetch"
runtime = "http"
config = { url = "https://example.com", method = "GET" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        wf.workflow.validate().unwrap();
        let task = &wf.workflow.tasks[0];
        assert_eq!(task.runtime.as_deref(), Some("http"));
        assert!(task.action.is_none());
        assert_eq!(task.config.as_ref().unwrap()["url"], "https://example.com");
    }

    #[test]
    fn test_runtime_effective_methods() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
runtime = "http"
config = { url = "https://example.com" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let task = &wf.workflow.tasks[0];
        assert_eq!(task.effective_runtime(), Some("http"));
        assert_eq!(task.effective_config()["url"], "https://example.com");
    }

    #[test]
    fn test_legacy_effective_methods() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "tool", name = "echo", params = { msg = "hi" } }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let task = &wf.workflow.tasks[0];
        assert_eq!(task.effective_runtime(), Some("echo"));
        assert_eq!(task.effective_config()["msg"], "hi");
    }

    #[test]
    fn test_mixed_runtime_and_action_tasks() {
        let toml = r#"
[workflow]
name = "mixed"
[[workflow.tasks]]
id = "old_style"
action = { type = "tool", name = "echo" }
[[workflow.tasks]]
id = "new_style"
runtime = "http"
config = { url = "https://example.com" }
dependencies = ["old_style"]
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        wf.workflow.validate().unwrap();
        assert!(wf.workflow.tasks[0].action.is_some());
        assert_eq!(wf.workflow.tasks[1].runtime.as_deref(), Some("http"));
    }

    #[test]
    fn test_task_with_neither_runtime_nor_action() {
        let toml = r#"
[workflow]
name = "bad"
[[workflow.tasks]]
id = "empty"
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let err = wf.workflow.validate().unwrap_err();
        assert!(err.to_string().contains("must have either"));
    }

    #[test]
    fn test_runtime_to_dynamic_tasks() {
        let toml = r#"
[workflow]
name = "rt"
[[workflow.tasks]]
id = "step1"
runtime = "passthrough"
config = { data = "hello" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let factory: ActionExecutorFactory =
            Arc::new(|_id, _action| Arc::new(|ctx| Box::pin(async move { Ok(ctx) })));
        let tasks = wf.workflow.to_dynamic_tasks(&factory).unwrap();
        assert_eq!(tasks.len(), 1);
    }

    // --- Dependency resolution tests ---

    #[test]
    fn test_to_dynamic_tasks_preserves_dependencies() {
        use cloacina_workflow::task::Task;

        let toml = r#"
[workflow]
name = "deps"
[[workflow.tasks]]
id = "first"
action = { type = "tool", name = "a" }
[[workflow.tasks]]
id = "second"
action = { type = "tool", name = "b" }
dependencies = ["first"]
[[workflow.tasks]]
id = "third"
action = { type = "tool", name = "c" }
dependencies = ["first", "second"]
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let factory: ActionExecutorFactory =
            Arc::new(|_id, _action| Arc::new(|ctx| Box::pin(async move { Ok(ctx) })));
        let tasks = wf.workflow.to_dynamic_tasks(&factory).unwrap();

        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].id(), "first");
        assert!(tasks[0].dependencies().is_empty());

        assert_eq!(tasks[1].id(), "second");
        assert_eq!(tasks[1].dependencies().len(), 1);
        assert_eq!(tasks[1].dependencies()[0].task_id, "first");

        assert_eq!(tasks[2].id(), "third");
        assert_eq!(tasks[2].dependencies().len(), 2);
    }

    #[test]
    fn test_to_dynamic_tasks_invalid_workflow_fails() {
        let toml = r#"
[workflow]
name = ""
[[workflow.tasks]]
id = "t1"
action = { type = "tool", name = "a" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let factory: ActionExecutorFactory =
            Arc::new(|_id, _action| Arc::new(|ctx| Box::pin(async move { Ok(ctx) })));
        let result = wf.workflow.to_dynamic_tasks(&factory);
        assert!(result.is_err());
    }

    #[test]
    fn test_to_dynamic_tasks_single_task() {
        use cloacina_workflow::task::Task;

        let toml = r#"
[workflow]
name = "single"
[[workflow.tasks]]
id = "only"
action = { type = "tool", name = "echo" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let factory: ActionExecutorFactory =
            Arc::new(|_id, _action| Arc::new(|ctx| Box::pin(async move { Ok(ctx) })));
        let tasks = wf.workflow.to_dynamic_tasks(&factory).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id(), "only");
        assert!(tasks[0].dependencies().is_empty());
    }

    #[test]
    fn test_to_dynamic_tasks_runtime_synthesizes_tool_action() {
        // The factory should receive a Tool action with runtime name and config
        use std::sync::Mutex;

        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        let factory: ActionExecutorFactory = Arc::new(move |id, action| {
            received_clone.lock().unwrap().push((
                id.to_string(),
                match action {
                    ActionDefinition::Tool { name, params } => (name.clone(), params.clone()),
                    _ => ("other".to_string(), HashMap::new()),
                },
            ));
            Arc::new(|ctx| Box::pin(async move { Ok(ctx) }))
        });

        let toml = r#"
[workflow]
name = "rt_test"
[[workflow.tasks]]
id = "fetch"
runtime = "http"
config = { url = "https://example.com" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let _tasks = wf.workflow.to_dynamic_tasks(&factory).unwrap();

        let calls = received.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "fetch");
        assert_eq!(calls[0].1.0, "http");
        assert_eq!(
            calls[0].1.1["url"],
            serde_json::json!("https://example.com")
        );
    }

    // --- effective_runtime / effective_config edge cases ---

    #[test]
    fn test_effective_runtime_script() {
        let task = TaskDefinition {
            id: "t1".into(),
            action: Some(ActionDefinition::Script {
                source_file: "test.rs".into(),
                language: "rust".into(),
            }),
            runtime: None,
            config: None,
            dependencies: vec![],
            retry_attempts: None,
            retry_delay_ms: None,
            capabilities: None,
        };
        assert_eq!(task.effective_runtime(), Some("script"));
    }

    #[test]
    fn test_effective_runtime_llm() {
        let task = TaskDefinition {
            id: "t1".into(),
            action: Some(ActionDefinition::Llm {
                prompt: "test".into(),
                model: None,
            }),
            runtime: None,
            config: None,
            dependencies: vec![],
            retry_attempts: None,
            retry_delay_ms: None,
            capabilities: None,
        };
        assert_eq!(task.effective_runtime(), Some("llm"));
    }

    #[test]
    fn test_effective_runtime_none() {
        let task = TaskDefinition {
            id: "t1".into(),
            action: None,
            runtime: None,
            config: None,
            dependencies: vec![],
            retry_attempts: None,
            retry_delay_ms: None,
            capabilities: None,
        };
        assert_eq!(task.effective_runtime(), None);
    }

    #[test]
    fn test_effective_runtime_prefers_runtime_field() {
        let task = TaskDefinition {
            id: "t1".into(),
            action: Some(ActionDefinition::Tool {
                name: "echo".into(),
                params: HashMap::new(),
            }),
            runtime: Some("custom_rt".into()),
            config: None,
            dependencies: vec![],
            retry_attempts: None,
            retry_delay_ms: None,
            capabilities: None,
        };
        assert_eq!(task.effective_runtime(), Some("custom_rt"));
    }

    #[test]
    fn test_effective_config_script() {
        let task = TaskDefinition {
            id: "t1".into(),
            action: Some(ActionDefinition::Script {
                source_file: "main.rs".into(),
                language: "rust".into(),
            }),
            runtime: None,
            config: None,
            dependencies: vec![],
            retry_attempts: None,
            retry_delay_ms: None,
            capabilities: None,
        };
        let cfg = task.effective_config();
        assert_eq!(cfg["source_file"], "main.rs");
        assert_eq!(cfg["language"], "rust");
    }

    #[test]
    fn test_effective_config_llm() {
        let task = TaskDefinition {
            id: "t1".into(),
            action: Some(ActionDefinition::Llm {
                prompt: "Summarize".into(),
                model: Some("gpt-4".into()),
            }),
            runtime: None,
            config: None,
            dependencies: vec![],
            retry_attempts: None,
            retry_delay_ms: None,
            capabilities: None,
        };
        let cfg = task.effective_config();
        assert_eq!(cfg["prompt"], "Summarize");
        assert_eq!(cfg["model"], "gpt-4");
    }

    #[test]
    fn test_effective_config_none() {
        let task = TaskDefinition {
            id: "t1".into(),
            action: None,
            runtime: None,
            config: None,
            dependencies: vec![],
            retry_attempts: None,
            retry_delay_ms: None,
            capabilities: None,
        };
        let cfg = task.effective_config();
        assert!(cfg.is_object());
        assert!(cfg.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_effective_config_prefers_config_field() {
        let task = TaskDefinition {
            id: "t1".into(),
            action: Some(ActionDefinition::Tool {
                name: "echo".into(),
                params: [("msg".into(), serde_json::json!("from_action"))].into(),
            }),
            runtime: None,
            config: Some(serde_json::json!({"msg": "from_config"})),
            dependencies: vec![],
            retry_attempts: None,
            retry_delay_ms: None,
            capabilities: None,
        };
        let cfg = task.effective_config();
        assert_eq!(cfg["msg"], "from_config");
    }

    // --- Additional validation edge cases ---

    #[test]
    fn test_validate_empty_task_id() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = ""
action = { type = "tool", name = "echo" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let err = wf.workflow.validate().unwrap_err();
        assert!(err.to_string().contains("Task ID cannot be empty"));
    }

    #[test]
    fn test_validate_three_node_cycle() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "a"
action = { type = "tool", name = "x" }
dependencies = ["c"]
[[workflow.tasks]]
id = "b"
action = { type = "tool", name = "x" }
dependencies = ["a"]
[[workflow.tasks]]
id = "c"
action = { type = "tool", name = "x" }
dependencies = ["b"]
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let err = wf.workflow.validate().unwrap_err();
        assert!(err.to_string().contains("Cycle detected"));
    }

    #[test]
    fn test_validate_linear_chain_no_cycle() {
        let toml = r#"
[workflow]
name = "chain"
[[workflow.tasks]]
id = "a"
action = { type = "tool", name = "x" }
[[workflow.tasks]]
id = "b"
action = { type = "tool", name = "x" }
dependencies = ["a"]
[[workflow.tasks]]
id = "c"
action = { type = "tool", name = "x" }
dependencies = ["b"]
[[workflow.tasks]]
id = "d"
action = { type = "tool", name = "x" }
dependencies = ["c"]
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        wf.workflow.validate().unwrap();
    }

    // --- from_file tests ---

    #[test]
    fn test_from_file_nonexistent() {
        let result = WorkflowFile::from_file(Path::new("/nonexistent/path/workflow.toml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read"));
    }

    #[test]
    fn test_from_file_valid() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), VALID_WORKFLOW).unwrap();
        let wf = WorkflowFile::from_file(tmp.path()).unwrap();
        assert_eq!(wf.workflow.name, "test_workflow");
    }

    #[test]
    fn test_from_file_invalid_toml() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "not valid toml {{{").unwrap();
        let result = WorkflowFile::from_file(tmp.path());
        assert!(result.is_err());
    }

    // --- Parsing edge cases ---

    #[test]
    fn test_parse_llm_with_model() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "llm", prompt = "Hello", model = "claude-3" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        match wf.workflow.tasks[0].action.as_ref().unwrap() {
            ActionDefinition::Llm { prompt, model } => {
                assert_eq!(prompt, "Hello");
                assert_eq!(model.as_deref(), Some("claude-3"));
            }
            _ => panic!("Expected Llm action"),
        }
    }

    #[test]
    fn test_parse_script_with_explicit_language() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "script", source_file = "main.rs", language = "rust" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        wf.workflow.validate().unwrap();
    }

    #[test]
    fn test_schedule_default_timezone() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "tool", name = "echo" }
[workflow.schedule]
cron = "0 * * * *"
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        assert_eq!(wf.workflow.schedule.as_ref().unwrap().timezone, "UTC");
    }

    #[test]
    fn test_capabilities_defaults() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "tool", name = "echo" }
capabilities = {}
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let caps = wf.workflow.tasks[0].capabilities.as_ref().unwrap();
        assert!(caps.filesystem.is_empty());
        assert!(!caps.network);
    }

    #[test]
    fn test_retry_default_delay() {
        let toml = r#"
[workflow]
name = "test"
[[workflow.tasks]]
id = "t1"
action = { type = "tool", name = "echo" }
retry_attempts = 3
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        assert_eq!(wf.workflow.tasks[0].retry_attempts, Some(3));
        assert!(wf.workflow.tasks[0].retry_delay_ms.is_none());
    }

    // --- Struct field tests ---

    #[test]
    fn test_workflow_definition_debug() {
        let toml = r#"
[workflow]
name = "debug_test"
[[workflow.tasks]]
id = "t1"
action = { type = "tool", name = "echo" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let debug = format!("{:?}", wf.workflow);
        assert!(debug.contains("debug_test"));
    }

    #[test]
    fn test_action_definition_debug() {
        let action = ActionDefinition::Tool {
            name: "test".into(),
            params: HashMap::new(),
        };
        let debug = format!("{:?}", action);
        assert!(debug.contains("Tool"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_workflow_definition_clone() {
        let toml = r#"
[workflow]
name = "clone_test"
[[workflow.tasks]]
id = "t1"
action = { type = "tool", name = "echo" }
"#;
        let wf = WorkflowFile::from_toml(toml).unwrap();
        let cloned = wf.workflow.clone();
        assert_eq!(cloned.name, "clone_test");
        assert_eq!(cloned.tasks.len(), 1);
    }
}

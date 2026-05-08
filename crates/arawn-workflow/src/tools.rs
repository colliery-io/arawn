//! Agent-facing tools for workflow management: create, list, delete, status.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::RwLock;

use arawn_tool::{Tool, ToolError, ToolOutput};

use cloacina::WorkflowExecutor;

use crate::runner::WorkflowRunner;
use crate::scaffold::{self, TaskDef, WorkflowDef};

/// Shared handle to the workflow runner (Option because it may not be available).
pub type SharedWorkflowRunner = Arc<RwLock<Option<Arc<WorkflowRunner>>>>;

/// Tool for creating a new workflow — scaffolds, compiles, and installs.
pub struct WorkflowCreateTool {
    packages_dir: PathBuf,
}

impl WorkflowCreateTool {
    pub fn new(packages_dir: PathBuf) -> Self {
        Self { packages_dir }
    }
}

#[async_trait]
impl Tool for WorkflowCreateTool {
    fn name(&self) -> &str {
        "workflow_create"
    }

    fn description(&self) -> &str {
        "Create a new scheduled workflow. Generates a Rust workflow crate with the specified \
         tasks and dependencies, compiles it, and installs it to the workflows directory. \
         The reconciler auto-loads it for cron execution."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Workflow name (e.g., 'github-triage', 'daily-summary')"
                },
                "description": {
                    "type": "string",
                    "description": "Human-readable description of what the workflow does"
                },
                "tasks": {
                    "type": "array",
                    "description": "List of tasks in the workflow DAG",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Unique task ID" },
                            "dependencies": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "IDs of tasks this depends on"
                            },
                            "body": {
                                "type": "string",
                                "description": "Rust async function body for the task"
                            },
                            "retry_attempts": {
                                "type": "integer",
                                "description": "Max retry attempts (optional)"
                            }
                        },
                        "required": ["id", "body"]
                    }
                },
                "cron": {
                    "type": "string",
                    "description": "Cron expression for scheduling (e.g., '0 8 * * 1-5')"
                },
                "cron_timezone": {
                    "type": "string",
                    "description": "IANA timezone for cron (default: UTC)"
                }
            },
            "required": ["name", "description", "tasks"]
        })
    }

    async fn execute(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let name = params["name"].as_str().unwrap_or("").to_string();
        if name.is_empty() {
            return Ok(ToolOutput::error("name is required"));
        }

        let description = params["description"].as_str().unwrap_or("").to_string();

        let tasks: Vec<TaskDef> = params["tasks"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|t| TaskDef {
                id: t["id"].as_str().unwrap_or("task").to_string(),
                dependencies: t["dependencies"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                body: t["body"].as_str().unwrap_or("Ok(())").to_string(),
                retry_attempts: t["retry_attempts"].as_i64().map(|v| v as i32),
            })
            .collect();

        if tasks.is_empty() {
            return Ok(ToolOutput::error("at least one task is required"));
        }

        let def = WorkflowDef {
            name: name.clone(),
            description,
            tasks,
            cron: params["cron"].as_str().map(String::from),
            cron_timezone: params["cron_timezone"].as_str().map(String::from),
        };

        // Generate scaffold in a temp directory
        let tmp_dir = tempfile::tempdir()
            .map_err(|e| ToolError::ExecutionFailed(format!("create temp dir: {e}")))?;

        scaffold::generate(tmp_dir.path(), &def)
            .map_err(|e| ToolError::ExecutionFailed(format!("scaffold: {e}")))?;

        // Compile the workflow
        let output = tokio::process::Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(tmp_dir.path())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("cargo build: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(ToolOutput::error(format!("compilation failed:\n{stderr}")));
        }

        // Find the compiled cdylib
        let crate_name = name.replace('-', "_");
        let target_dir = tmp_dir.path().join("target/release");
        let lib_name = if cfg!(target_os = "macos") {
            format!("lib{crate_name}.dylib")
        } else {
            format!("lib{crate_name}.so")
        };
        let lib_path = target_dir.join(&lib_name);
        if !lib_path.exists() {
            return Ok(ToolOutput::error(format!(
                "compiled library not found at {}",
                lib_path.display()
            )));
        }

        // Install to the workflows directory
        let pkg_dir = self.packages_dir.join(&name);
        std::fs::create_dir_all(&pkg_dir)
            .map_err(|e| ToolError::ExecutionFailed(format!("create package dir: {e}")))?;

        // Copy package.toml and compiled library
        let _ = std::fs::copy(
            tmp_dir.path().join("package.toml"),
            pkg_dir.join("package.toml"),
        );
        std::fs::copy(&lib_path, pkg_dir.join(&lib_name))
            .map_err(|e| ToolError::ExecutionFailed(format!("copy library: {e}")))?;

        Ok(ToolOutput::success(json!({
            "name": name,
            "installed_at": pkg_dir.display().to_string(),
            "status": "installed"
        }).to_string()))
    }
}

/// Tool for listing installed workflows.
pub struct WorkflowListTool {
    packages_dir: PathBuf,
}

impl WorkflowListTool {
    pub fn new(packages_dir: PathBuf) -> Self {
        Self { packages_dir }
    }
}

#[async_trait]
impl Tool for WorkflowListTool {
    fn name(&self) -> &str {
        "workflow_list"
    }

    fn description(&self) -> &str {
        "List all installed workflows with their names and cron schedules."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _ctx: &dyn arawn_tool::ToolContext, _params: Value) -> Result<ToolOutput, ToolError> {
        let mut workflows = Vec::new();

        if self.packages_dir.exists() {
            for entry in std::fs::read_dir(&self.packages_dir)
                .map_err(|e| ToolError::ExecutionFailed(format!("read workflows dir: {e}")))?
            {
                if let Ok(entry) = entry
                    && entry.path().is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let pkg_toml = entry.path().join("package.toml");
                        let cron = if pkg_toml.exists() {
                            std::fs::read_to_string(&pkg_toml).ok().and_then(|s| {
                                s.lines()
                                    .find(|l| l.contains("cron"))
                                    .map(|l| {
                                        l.split('=')
                                            .nth(1)
                                            .unwrap_or("")
                                            .trim()
                                            .trim_matches('"')
                                            .to_string()
                                    })
                            })
                        } else {
                            None
                        };

                        workflows.push(json!({
                            "name": name,
                            "cron": cron,
                        }));
                    }
            }
        }

        Ok(ToolOutput::success(json!(workflows).to_string()))
    }
}

/// Tool for deleting a workflow package.
pub struct WorkflowDeleteTool {
    packages_dir: PathBuf,
}

impl WorkflowDeleteTool {
    pub fn new(packages_dir: PathBuf) -> Self {
        Self { packages_dir }
    }
}

#[async_trait]
impl Tool for WorkflowDeleteTool {
    fn name(&self) -> &str {
        "workflow_delete"
    }

    fn description(&self) -> &str {
        "Delete an installed workflow package. The reconciler will unload it automatically."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the workflow to delete"
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let name = params["name"].as_str().unwrap_or("").to_string();
        if name.is_empty() {
            return Ok(ToolOutput::error("name is required"));
        }

        let pkg_dir = self.packages_dir.join(&name);
        if !pkg_dir.exists() {
            return Ok(ToolOutput::error(format!("workflow '{name}' not found")));
        }

        std::fs::remove_dir_all(&pkg_dir)
            .map_err(|e| ToolError::ExecutionFailed(format!("delete workflow: {e}")))?;

        Ok(ToolOutput::success(json!({
            "name": name,
            "status": "deleted"
        }).to_string()))
    }
}

/// Tool for checking workflow execution status.
pub struct WorkflowStatusTool {
    runner: SharedWorkflowRunner,
}

impl WorkflowStatusTool {
    pub fn new(runner: SharedWorkflowRunner) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl Tool for WorkflowStatusTool {
    fn name(&self) -> &str {
        "workflow_status"
    }

    fn description(&self) -> &str {
        "Check recent execution status for a workflow — shows pass/fail/running with timestamps."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Workflow name to check (optional — omit for all)"
                }
            },
            "required": []
        })
    }

    async fn execute(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let runner_guard = self.runner.read().await;
        let runner = match runner_guard.as_ref() {
            Some(r) => r,
            None => return Ok(ToolOutput::error("workflow runner not available")),
        };

        let name_filter = params["name"].as_str();

        let executions = runner
            .inner()
            .list_executions()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("list executions: {e}")))?;

        let results: Vec<Value> = executions
            .iter()
            .filter(|e| name_filter.is_none() || name_filter == Some(e.workflow_name.as_str()))
            .take(20)
            .map(|e| {
                json!({
                    "execution_id": e.execution_id.to_string(),
                    "workflow": e.workflow_name,
                    "status": format!("{:?}", e.status),
                    "start_time": e.start_time.to_string(),
                    "duration_ms": e.duration.map(|d| d.as_millis()),
                    "error": e.error_message,
                })
            })
            .collect();

        Ok(ToolOutput::success(json!(results).to_string()))
    }
}

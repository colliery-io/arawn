//! Wrapper around cloacina's DefaultRunner for arawn server integration.

use std::path::{Path, PathBuf};

use cloacina::prelude::*;
use cloacina::WorkflowExecutor;
use tracing::{debug, info};

/// Configuration for the workflow runner.
pub struct WorkflowRunnerConfig {
    /// Path to the SQLite database for workflow state.
    pub database_path: PathBuf,
    /// Directory to watch for .cloacina packages.
    pub packages_dir: PathBuf,
    /// Maximum concurrent task executions.
    pub max_concurrent_tasks: usize,
}

impl WorkflowRunnerConfig {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            database_path: data_dir.join("workflows.db"),
            packages_dir: data_dir.join("workflows"),
            max_concurrent_tasks: 10,
        }
    }
}

/// Arawn's workflow engine — wraps cloacina's DefaultRunner.
///
/// Background services (scheduler, reconciler, cron) start automatically
/// on construction when enabled in config. Call `shutdown()` to drain.
pub struct WorkflowRunner {
    runner: DefaultRunner,
}

impl WorkflowRunner {
    /// Initialize the workflow runner with the given configuration.
    /// Creates the database and packages directory if they don't exist.
    /// Background services start immediately on successful construction.
    pub async fn new(config: WorkflowRunnerConfig) -> Result<Self, WorkflowError> {
        std::fs::create_dir_all(&config.packages_dir)
            .map_err(|e| WorkflowError::Init(format!("create packages dir: {e}")))?;

        let db_url = format!("sqlite://{}", config.database_path.display());
        debug!(db_url = %db_url, packages_dir = %config.packages_dir.display(), "initializing workflow runner");

        let runner_config = DefaultRunnerConfig::builder()
            .enable_registry_reconciler(true)
            .enable_cron_scheduling(true)
            .registry_storage_path(Some(config.packages_dir.clone()))
            .max_concurrent_tasks(config.max_concurrent_tasks)
            .build()
            .map_err(|e| WorkflowError::Init(format!("build runner config: {e}")))?;

        let runner = DefaultRunner::with_config(&db_url, runner_config)
            .await
            .map_err(|e| WorkflowError::Init(format!("create runner: {e}")))?;

        info!(
            db = %config.database_path.display(),
            packages = %config.packages_dir.display(),
            "workflow runner initialized"
        );

        Ok(Self { runner })
    }

    /// Execute a named workflow programmatically.
    pub async fn execute(
        &self,
        workflow_name: &str,
        context: serde_json::Value,
    ) -> Result<WorkflowExecutionResult, WorkflowError> {
        let ctx = if context.is_object() {
            let json = serde_json::to_string(&context)
                .map_err(|e| WorkflowError::Runtime(format!("serialize context: {e}")))?;
            Context::from_json(json)
                .map_err(|e| WorkflowError::Runtime(format!("create context: {e}")))?
        } else {
            Context::new()
        };
        self.runner
            .execute(workflow_name, ctx)
            .await
            .map_err(|e| WorkflowError::Runtime(format!("execute '{workflow_name}': {e}")))
    }

    /// Graceful shutdown — drains in-flight pipelines.
    pub async fn shutdown(&self) {
        info!("shutting down workflow runner");
        if let Err(e) = self.runner.shutdown().await {
            tracing::warn!(error = %e, "workflow runner shutdown error");
        }
    }

    /// Get a reference to the underlying DefaultRunner.
    pub fn inner(&self) -> &DefaultRunner {
        &self.runner
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("initialization failed: {0}")]
    Init(String),
    #[error("runtime error: {0}")]
    Runtime(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn runner_initializes_and_shuts_down() {
        let tmp = tempfile::tempdir().unwrap();
        let config = WorkflowRunnerConfig::new(tmp.path());

        // Packages dir should be created
        assert!(!config.packages_dir.exists());
        let runner = WorkflowRunner::new(config).await.unwrap();
        assert!(tmp.path().join("workflows").exists());

        // Executing a nonexistent workflow should error, not panic
        let result = runner.execute("nonexistent", serde_json::json!({})).await;
        assert!(result.is_err());

        runner.shutdown().await;
    }

    #[tokio::test]
    async fn runner_starts_with_empty_packages_dir() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("workflows")).unwrap();
        let config = WorkflowRunnerConfig::new(tmp.path());
        let runner = WorkflowRunner::new(config).await.unwrap();
        // No packages — runner should still be operational
        runner.shutdown().await;
    }
}

//! Task tracking types for long-running operations.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Task status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is queued but not started.
    Pending,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
    /// Task was cancelled.
    Cancelled,
}

/// A tracked task/operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedTask {
    /// Task ID.
    pub id: String,
    /// Task type/name.
    pub task_type: String,
    /// Current status.
    pub status: TaskStatus,
    /// Progress percentage (0-100).
    pub progress: Option<u8>,
    /// Status message.
    pub message: Option<String>,
    /// Associated session ID.
    pub session_id: Option<String>,
    /// When the task was created.
    pub created_at: DateTime<Utc>,
    /// When the task started running.
    pub started_at: Option<DateTime<Utc>>,
    /// When the task completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed.
    pub error: Option<String>,
}

impl TrackedTask {
    /// Create a new pending task.
    pub fn new(id: impl Into<String>, task_type: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            task_type: task_type.into(),
            status: TaskStatus::Pending,
            progress: None,
            message: None,
            session_id: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
        }
    }

    /// Set the session ID.
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Mark the task as running.
    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Update progress.
    pub fn update_progress(&mut self, progress: u8, message: Option<String>) {
        self.progress = Some(progress.min(100));
        self.message = message;
    }

    /// Mark the task as completed.
    pub fn complete(&mut self, message: Option<String>) {
        self.status = TaskStatus::Completed;
        self.progress = Some(100);
        self.message = message;
        self.completed_at = Some(Utc::now());
    }

    /// Mark the task as failed.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = TaskStatus::Failed;
        self.error = Some(error.into());
        self.completed_at = Some(Utc::now());
    }

    /// Mark the task as cancelled.
    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }
}

/// In-memory task store.
pub type TaskStore = Arc<RwLock<HashMap<String, TrackedTask>>>;

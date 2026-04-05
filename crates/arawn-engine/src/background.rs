//! Background task infrastructure — run shell commands and agents asynchronously.
//!
//! The `BackgroundTaskManager` tracks spawned background tasks, captures their
//! output, and produces notifications when they complete. The QueryEngine drains
//! these notifications at the top of each iteration and injects them into the
//! conversation so the LLM knows what finished.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

/// Maximum output buffer size per task (100 KB). Older output is truncated.
const MAX_OUTPUT_BYTES: usize = 100 * 1024;

/// Generates a background task ID: "bg_" + 8 hex chars.
fn generate_task_id() -> String {
    use std::fmt::Write;
    let bytes: [u8; 4] = rand_bytes();
    let mut id = String::with_capacity(11);
    id.push_str("bg_");
    for b in bytes {
        write!(id, "{b:02x}").unwrap();
    }
    id
}

fn rand_bytes() -> [u8; 4] {
    let mut buf = [0u8; 4];
    // Use timestamp + thread id as simple entropy source (no extra dep)
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let thread_id = std::thread::current().id();
    let hash = t.wrapping_mul(6364136223846793005).wrapping_add(format!("{thread_id:?}").len() as u128);
    buf.copy_from_slice(&hash.to_le_bytes()[..4]);
    buf
}

/// A notification about a completed background task, ready for injection
/// into the LLM conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNotification {
    pub task_id: String,
    pub description: String,
    pub status: String,
    pub summary: String,
}

impl TaskNotification {
    /// Format as the XML structure the LLM expects.
    pub fn to_message(&self) -> String {
        format!(
            "<task-notification>\n\
             <task-id>{}</task-id>\n\
             <status>{}</status>\n\
             <summary>{}</summary>\n\
             </task-notification>",
            self.task_id, self.status, self.summary,
        )
    }
}

/// What kind of background task this is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackgroundTaskKind {
    Shell { command: String },
    Agent { prompt: String, agent_type: Option<String> },
}

/// Current status of a background task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundTaskStatus {
    Running,
    Completed { exit_code: Option<i32> },
    Failed { error: String },
    Killed,
}

impl BackgroundTaskStatus {
    pub fn is_terminal(&self) -> bool {
        !matches!(self, BackgroundTaskStatus::Running)
    }

    pub fn label(&self) -> &str {
        match self {
            BackgroundTaskStatus::Running => "running",
            BackgroundTaskStatus::Completed { .. } => "completed",
            BackgroundTaskStatus::Failed { .. } => "failed",
            BackgroundTaskStatus::Killed => "killed",
        }
    }
}

/// A single background task being tracked.
pub struct BackgroundTask {
    pub id: String,
    pub kind: BackgroundTaskKind,
    pub description: String,
    pub status: BackgroundTaskStatus,
    pub started_at: Instant,
    pub completed_at: Option<Instant>,
    /// Captured output (stdout + stderr interleaved). Bounded to MAX_OUTPUT_BYTES.
    output: Arc<RwLock<String>>,
    /// Cancellation token for cooperative shutdown.
    pub cancel_token: CancellationToken,
    /// JoinHandle for the spawned tokio task. Kept to prevent the task from
    /// being detached — dropping the handle doesn't cancel the task in tokio,
    /// but holding it ensures we can abort if needed in the future.
    #[allow(dead_code)]
    handle: Option<JoinHandle<()>>,
    /// Whether a notification has been sent for this task.
    notified: bool,
}

impl std::fmt::Debug for BackgroundTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackgroundTask")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("status", &self.status)
            .field("notified", &self.notified)
            .finish()
    }
}

impl BackgroundTask {
    /// Read the current output buffer.
    pub fn read_output(&self) -> String {
        self.output.read().unwrap().clone()
    }

    /// Get a shared handle to the output buffer (for the writer task).
    pub fn output_handle(&self) -> Arc<RwLock<String>> {
        Arc::clone(&self.output)
    }
}

/// Append text to a bounded output buffer. If it exceeds MAX_OUTPUT_BYTES,
/// the front is truncated with a marker.
pub fn append_output(buf: &Arc<RwLock<String>>, text: &str) {
    let mut output = buf.write().unwrap();
    output.push_str(text);
    if output.len() > MAX_OUTPUT_BYTES {
        let trim_to = output.len() - MAX_OUTPUT_BYTES + 64; // leave room for marker
        // Find a char boundary to trim at
        let boundary = output.ceil_char_boundary(trim_to);
        let trimmed = output[boundary..].to_string();
        *output = format!("[...output truncated...]\n{trimmed}");
    }
}

/// Session-scoped manager for background tasks.
pub struct BackgroundTaskManager {
    tasks: RwLock<HashMap<String, BackgroundTask>>,
    /// Completed task notifications waiting to be drained by the engine.
    notifications: Mutex<Vec<TaskNotification>>,
}

impl BackgroundTaskManager {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            notifications: Mutex::new(Vec::new()),
        }
    }

    /// Register a new background task. Returns the task ID and a shared output
    /// buffer handle for the spawner to write to.
    pub fn register(
        &self,
        kind: BackgroundTaskKind,
        description: String,
        handle: JoinHandle<()>,
        cancel_token: CancellationToken,
    ) -> (String, Arc<RwLock<String>>) {
        let id = generate_task_id();
        let output = Arc::new(RwLock::new(String::new()));

        let task = BackgroundTask {
            id: id.clone(),
            kind,
            description,
            status: BackgroundTaskStatus::Running,
            started_at: Instant::now(),
            completed_at: None,
            output: Arc::clone(&output),
            cancel_token,
            handle: Some(handle),
            notified: false,
        };

        info!(task_id = %id, "background task registered");
        self.tasks.write().unwrap().insert(id.clone(), task);
        (id, output)
    }

    /// Mark a task as completed and queue a notification.
    pub fn complete(&self, task_id: &str, status: BackgroundTaskStatus) {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(task_id) {
            let summary = match &status {
                BackgroundTaskStatus::Completed { exit_code } => {
                    let code_str = exit_code.map(|c| format!(" (exit code {c})")).unwrap_or_default();
                    format!(
                        "Background {} \"{}\"{code_str}",
                        match &task.kind {
                            BackgroundTaskKind::Shell { .. } => "command",
                            BackgroundTaskKind::Agent { .. } => "agent",
                        },
                        task.description,
                    )
                }
                BackgroundTaskStatus::Failed { error } => {
                    format!("Background task \"{}\" failed: {error}", task.description)
                }
                BackgroundTaskStatus::Killed => {
                    format!("Background task \"{}\" was stopped", task.description)
                }
                BackgroundTaskStatus::Running => unreachable!(),
            };

            task.status = status;
            task.completed_at = Some(Instant::now());

            if !task.notified {
                task.notified = true;
                let notification = TaskNotification {
                    task_id: task_id.to_string(),
                    description: task.description.clone(),
                    status: task.status.label().to_string(),
                    summary,
                };
                info!(task_id, status = %notification.status, "background task completed");
                self.notifications.lock().unwrap().push(notification);
            }
        } else {
            warn!(task_id, "complete called for unknown task");
        }
    }

    /// Drain all pending notifications (called by the engine at each iteration).
    pub fn drain_notifications(&self) -> Vec<TaskNotification> {
        let mut notifs = self.notifications.lock().unwrap();
        std::mem::take(&mut *notifs)
    }

    /// Get a task's current status.
    pub fn status(&self, task_id: &str) -> Option<BackgroundTaskStatus> {
        self.tasks.read().unwrap().get(task_id).map(|t| t.status.clone())
    }

    /// Read a task's captured output.
    pub fn read_output(&self, task_id: &str) -> Option<String> {
        self.tasks.read().unwrap().get(task_id).map(|t| t.read_output())
    }

    /// Cancel a running task.
    pub fn cancel(&self, task_id: &str) -> bool {
        let tasks = self.tasks.read().unwrap();
        if let Some(task) = tasks.get(task_id) {
            if task.status == BackgroundTaskStatus::Running {
                debug!(task_id, "cancelling background task");
                task.cancel_token.cancel();
                return true;
            }
        }
        false
    }

    /// List all tasks (for inventory/status display).
    pub fn list(&self) -> Vec<TaskSummary> {
        self.tasks
            .read()
            .unwrap()
            .values()
            .map(|t| TaskSummary {
                id: t.id.clone(),
                description: t.description.clone(),
                status: t.status.label().to_string(),
                elapsed_secs: t.started_at.elapsed().as_secs(),
            })
            .collect()
    }

    /// Number of currently running tasks.
    pub fn running_count(&self) -> usize {
        self.tasks
            .read()
            .unwrap()
            .values()
            .filter(|t| t.status == BackgroundTaskStatus::Running)
            .count()
    }
}

impl Default for BackgroundTaskManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Lightweight summary for listing/display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: String,
    pub description: String,
    pub status: String,
    pub elapsed_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, sleep};

    #[test]
    fn generate_task_id_format() {
        let id = generate_task_id();
        assert!(id.starts_with("bg_"));
        assert_eq!(id.len(), 11); // "bg_" + 8 hex chars
    }

    #[test]
    fn task_status_labels() {
        assert_eq!(BackgroundTaskStatus::Running.label(), "running");
        assert_eq!(
            BackgroundTaskStatus::Completed { exit_code: Some(0) }.label(),
            "completed"
        );
        assert_eq!(
            BackgroundTaskStatus::Failed {
                error: "boom".into()
            }
            .label(),
            "failed"
        );
        assert_eq!(BackgroundTaskStatus::Killed.label(), "killed");
    }

    #[test]
    fn task_status_is_terminal() {
        assert!(!BackgroundTaskStatus::Running.is_terminal());
        assert!(BackgroundTaskStatus::Completed { exit_code: None }.is_terminal());
        assert!(BackgroundTaskStatus::Failed { error: String::new() }.is_terminal());
        assert!(BackgroundTaskStatus::Killed.is_terminal());
    }

    #[test]
    fn notification_to_message_format() {
        let notif = TaskNotification {
            task_id: "bg_abcd1234".into(),
            description: "cargo test".into(),
            status: "completed".into(),
            summary: "Background command \"cargo test\" (exit code 0)".into(),
        };
        let msg = notif.to_message();
        assert!(msg.contains("<task-id>bg_abcd1234</task-id>"));
        assert!(msg.contains("<status>completed</status>"));
    }

    #[tokio::test]
    async fn register_and_complete() {
        let mgr = BackgroundTaskManager::new();
        let token = CancellationToken::new();
        let handle = tokio::spawn(async {});

        let (id, _output) = mgr.register(
            BackgroundTaskKind::Shell {
                command: "echo hi".into(),
            },
            "echo hi".into(),
            handle,
            token,
        );

        assert_eq!(mgr.running_count(), 1);
        assert_eq!(
            mgr.status(&id),
            Some(BackgroundTaskStatus::Running)
        );

        mgr.complete(&id, BackgroundTaskStatus::Completed { exit_code: Some(0) });

        assert_eq!(mgr.running_count(), 0);
        assert!(mgr.status(&id).unwrap().is_terminal());

        let notifs = mgr.drain_notifications();
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].task_id, id);
        assert_eq!(notifs[0].status, "completed");

        // Second drain should be empty (no duplicate)
        assert!(mgr.drain_notifications().is_empty());
    }

    #[tokio::test]
    async fn cancel_running_task() {
        let mgr = BackgroundTaskManager::new();
        let token = CancellationToken::new();
        let token_clone = token.clone();
        let handle = tokio::spawn(async move {
            token_clone.cancelled().await;
        });

        let (id, _) = mgr.register(
            BackgroundTaskKind::Shell {
                command: "sleep 999".into(),
            },
            "sleep".into(),
            handle,
            token,
        );

        assert!(mgr.cancel(&id));
        // Give the task a moment to be cancelled
        sleep(Duration::from_millis(10)).await;
    }

    #[test]
    fn output_buffer_bounded() {
        let buf = Arc::new(RwLock::new(String::new()));
        // Write more than MAX_OUTPUT_BYTES
        let big_chunk = "x".repeat(MAX_OUTPUT_BYTES + 1000);
        append_output(&buf, &big_chunk);

        let output = buf.read().unwrap();
        assert!(output.len() <= MAX_OUTPUT_BYTES + 100); // some slack for marker
        assert!(output.contains("[...output truncated...]"));
    }

    #[test]
    fn output_buffer_small_writes() {
        let buf = Arc::new(RwLock::new(String::new()));
        append_output(&buf, "line 1\n");
        append_output(&buf, "line 2\n");
        let output = buf.read().unwrap();
        assert_eq!(*output, "line 1\nline 2\n");
    }

    #[tokio::test]
    async fn list_tasks() {
        let mgr = BackgroundTaskManager::new();
        let token = CancellationToken::new();
        let handle = tokio::spawn(async {});

        mgr.register(
            BackgroundTaskKind::Agent {
                prompt: "research".into(),
                agent_type: Some("explorer".into()),
            },
            "research task".into(),
            handle,
            token,
        );

        let list = mgr.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].description, "research task");
        assert_eq!(list[0].status, "running");
    }

    #[test]
    fn complete_unknown_task_is_safe() {
        let mgr = BackgroundTaskManager::new();
        mgr.complete("bg_nonexistent", BackgroundTaskStatus::Killed);
        assert!(mgr.drain_notifications().is_empty());
    }

    #[test]
    fn cancel_nonexistent_returns_false() {
        let mgr = BackgroundTaskManager::new();
        assert!(!mgr.cancel("bg_nonexistent"));
    }

    #[tokio::test]
    async fn duplicate_complete_only_notifies_once() {
        let mgr = BackgroundTaskManager::new();
        let token = CancellationToken::new();
        let handle = tokio::spawn(async {});

        let (id, _) = mgr.register(
            BackgroundTaskKind::Shell {
                command: "echo".into(),
            },
            "echo".into(),
            handle,
            token,
        );

        mgr.complete(&id, BackgroundTaskStatus::Completed { exit_code: Some(0) });
        mgr.complete(&id, BackgroundTaskStatus::Completed { exit_code: Some(0) });

        let notifs = mgr.drain_notifications();
        assert_eq!(notifs.len(), 1);
    }
}

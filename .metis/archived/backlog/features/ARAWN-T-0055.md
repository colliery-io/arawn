---
id: background-task-infrastructure
level: task
title: "Background task infrastructure (TaskStop, TaskOutput, run_in_background for agents)"
short_code: "ARAWN-T-0055"
created_at: 2026-04-03T01:01:37.271512+00:00
updated_at: 2026-04-05T12:51:39.855640+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Background task infrastructure (TaskStop, TaskOutput, run_in_background for agents)

## Objective

Build infrastructure for running agents and shell commands in the background, with tools to check status, read output, and stop them. The agent can kick off a build or sub-agent, continue working, and get notified when it finishes.

### Type: Feature | Priority: P2

- **User Value**: Non-blocking execution. Agent launches a long build, continues other work, gets notified on completion. Users see background task status in the TUI.
- **Effort Estimate**: L

## Current State

**What exists:**
- `SessionTaskStore` (`task_list.rs`) — metadata-only tracking (subject, status). No async execution, no process handles, no output capture. Just UI progress tracking.
- All tools are fully blocking — `Tool::execute()` awaits to completion before the engine collects results and proceeds to the next LLM turn.
- `QueryEngine::execute_tool()` runs read-only tools concurrently via `join_all()`, but write tools run serially. All must complete before the next LLM turn.

**The core challenge:** The QueryEngine collects all tool results before the next LLM turn. A background task needs to return a "task started" result immediately, then inject a notification into the conversation later when it completes.

## Reference: Claude Code Implementation

Claude Code's background task system:

- **Task types**: `local_bash`, `local_agent`, `remote_agent`
- **Spawning**: `run_in_background: true` on BashTool/AgentTool → spawns async, returns taskId immediately
- **State**: Tasks stored in AppState with status (`pending`/`running`/`completed`/`failed`/`killed`), output file path, exit code, `notified` flag
- **Output**: Written to disk file (`~/.claude/.task-output/{taskId}`), polled on 1s intervals for UI
- **Notifications**: XML-wrapped `<task_notification>` messages enqueued and injected into the LLM conversation on completion. `notified` flag prevents duplicates.
- **TaskOutput tool**: `block: true` polls every 100ms until task completes or timeout; `block: false` returns current status immediately
- **TaskStop tool**: Sends SIGTERM to shell process or aborts agent's AbortController. Sets status to `killed`.
- **Cleanup**: Completed tasks evicted after notification acknowledged

## Architecture

### BackgroundTaskManager

Session-scoped registry of running background tasks. Holds process handles, output buffers, and completion channels.

```rust
pub struct BackgroundTaskManager {
    tasks: Arc<RwLock<HashMap<String, BackgroundTask>>>,
    /// Channel for completed task notifications — drained by the engine
    /// at the top of each iteration loop.
    notifications: Arc<Mutex<Vec<TaskNotification>>>,
}

pub struct BackgroundTask {
    pub id: String,              // "bg_" + 8-char random
    pub kind: BackgroundTaskKind, // Shell or Agent
    pub description: String,
    pub status: BackgroundTaskStatus,
    pub started_at: Instant,
    pub completed_at: Option<Instant>,
    /// Captured stdout+stderr (buffered in memory, bounded).
    pub output: Arc<RwLock<String>>,
    /// For cancellation.
    pub cancel_token: CancellationToken,
    /// JoinHandle for the spawned tokio task.
    pub handle: Option<JoinHandle<()>>,
}

pub enum BackgroundTaskStatus {
    Running,
    Completed { exit_code: Option<i32> },
    Failed { error: String },
    Killed,
}

pub enum BackgroundTaskKind {
    Shell { command: String },
    Agent { prompt: String, agent_type: Option<String> },
}
```

### Notification Injection

The key integration point. At the top of each QueryEngine iteration (before the LLM call), drain completed task notifications and inject them as system messages:

```rust
// In QueryEngine::run(), top of loop:
if let Some(ref bg_manager) = self.background_tasks {
    let notifications = bg_manager.drain_notifications();
    for notif in notifications {
        session.add_message(Message::System {
            content: format_task_notification(&notif),
        });
    }
}
```

Notification format (matching Claude Code's XML structure):
```xml
<task_notification>
  <task_id>bg_a1b2c3d4</task_id>
  <status>completed</status>
  <summary>Background command "cargo test" completed (exit code 0)</summary>
  <output_file>/path/to/output</output_file>
</task_notification>
```

The LLM sees this at the start of its next turn and can decide whether to check the output or continue.

### Tool Changes

**ShellTool** — add `run_in_background` parameter:
```rust
// If run_in_background == true:
//   1. Spawn process via tokio::process::Command
//   2. Register in BackgroundTaskManager
//   3. Spawn tokio task that reads output + waits for exit
//   4. On exit: update status, push to notifications channel
//   5. Return immediately: "Background task bg_xxx started: {command}"
```

**AgentTool** — add `run_in_background` parameter:
```rust
// If run_in_background == true:
//   1. Build sub-agent QueryEngine (same as today)
//   2. Register in BackgroundTaskManager
//   3. Spawn tokio task that runs engine.run()
//   4. On completion: capture final text, push notification
//   5. Return immediately: "Background agent bg_xxx started: {description}"
```

**TaskOutputTool** (new):
```rust
// Parameters: task_id, block (bool, default true), timeout_ms (default 30000)
// If block && task is running: poll every 100ms until done or timeout
// Return: { task_id, status, output (tail), error? }
```

**TaskStopTool** (new):
```rust
// Parameters: task_id
// Cancel via cancel_token.cancel()
// For shell: also send SIGTERM to child process
// Update status to Killed
// Return: "Task bg_xxx stopped"
```

### Output Capture

Shell output is captured by a tokio task that reads from the child process's stdout/stderr and appends to the `output: Arc<RwLock<String>>` buffer. The buffer is bounded (e.g., last 100KB) — older output is truncated.

For agents, the final text response is captured as the output. Intermediate tool calls are not — the agent's own session tracks those.

### ToolContext Integration

`BackgroundTaskManager` needs to be accessible from tools. Options:
1. **On ToolContext** — add `background_tasks: Option<Arc<BackgroundTaskManager>>` field
2. **On the tools directly** — ShellTool/AgentTool hold an `Arc<BackgroundTaskManager>`

Option 2 is cleaner — only the tools that support backgrounding need access. The manager is created once per service and shared.

### QueryEngine Integration

```rust
pub struct QueryEngine {
    // ... existing fields
    background_tasks: Option<Arc<BackgroundTaskManager>>,
}
```

Builder: `.with_background_tasks(manager)`

The engine drains notifications at the top of each iteration. It does NOT wait for background tasks — they're fire-and-forget until the notification arrives.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `BackgroundTaskManager` — session-scoped registry with task state, output buffer, cancel token
- [ ] `run_in_background: true` parameter on ShellTool spawns command as tokio task, returns task ID immediately
- [ ] `run_in_background: true` parameter on AgentTool spawns sub-agent as tokio task, returns task ID immediately
- [ ] Shell output captured to bounded in-memory buffer from child process stdout/stderr
- [ ] Agent output captured as final response text on completion
- [ ] Completion notifications drained by QueryEngine at top of each iteration and injected as system messages
- [ ] `TaskOutput` tool reads output from a background task (with optional blocking/polling)
- [ ] `TaskStop` tool cancels a running task via CancellationToken + SIGTERM for shell
- [ ] Task status tracking: Running → Completed/Failed/Killed
- [ ] Notification deduplication — each task notifies exactly once

## Implementation Plan

**Phase 1**: `BackgroundTaskManager` struct, task state, notification channel. Unit tests for lifecycle.

**Phase 2**: ShellTool `run_in_background` — spawn process, capture output, push notification on exit. Integration test.

**Phase 3**: Notification injection in QueryEngine iteration loop.

**Phase 4**: `TaskOutput` and `TaskStop` tools.

**Phase 5**: AgentTool `run_in_background` — spawn sub-agent engine, capture result.

**Phase 6**: TUI rendering — show background task status (count, names) in status bar or sidebar.

## Implementation Notes

- `CancellationToken` from `tokio_util::sync` is the right primitive for cooperative cancellation
- Shell cancellation also needs SIGTERM to the child process group (`kill(-pid, SIGTERM)`)
- Output buffer should be bounded — Claude Code writes to disk, but in-memory with a cap is simpler for MVP. Can add disk spillover later.
- The `SessionTaskStore` (existing) is separate — it tracks user-visible progress metadata. `BackgroundTaskManager` tracks actual running processes. They're complementary, not overlapping.
- Agent background tasks inherit the parent's tool registry but get a fresh session and context (same as today's `for_sub_agent()`)
- Consider: should background task output persist across session resume? For MVP, no — in-memory only. Can add JSONL persistence later.
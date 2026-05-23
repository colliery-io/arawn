---
id: integration-toolkit-provider
level: initiative
title: "Integration Toolkit — provider-backed TaskList, Push, Schedule, and Messaging capabilities"
short_code: "ARAWN-I-0028"
created_at: 2026-04-16T17:10:23.335751+00:00
updated_at: 2026-04-16T17:10:23.335751+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: integration-toolkit-provider
---

# Integration Toolkit — provider-backed TaskList, Push, Schedule, and Messaging capabilities Initiative

## Context

Arawn is a personal assistant agent that currently only interacts through its TUI/WebSocket interface. To be genuinely useful as a day-to-day assistant it needs to reach into the user's real productivity tools: task lists, calendars, messaging, and push notifications.

Rather than building one-off integrations, we want **four abstract capability traits** with concrete provider implementations behind them — the same facade/adapter pattern used for LLM providers. This lets the user configure their preferred apps in `arawn.toml` while tools and workflows program against stable interfaces.

### Capability Map

| Capability | Trait | First Provider | Use Cases |
|-----------|-------|---------------|-----------|
| **TaskList** | `TaskListProvider` | Google Tasks | Create tasks, list/query tasks, mark complete, nag on overdue |
| **Push** | `PushProvider` | Slack | Get user attention — workflow completions, errors, reminders |
| **Schedule** | `ScheduleProvider` | Google Calendar | Read upcoming events, create/update events, find free slots |
| **Messaging** | `MessagingProvider` | Slack | Bi-directional conversation — monitor channels, send/read messages |

Future providers: Discord, Telegram, raw SMS (messaging/push), Todoist/Linear (task list), Outlook (schedule).

### Key Design Constraints

1. **Slack serves dual duty** — it's both a `MessagingProvider` and the first `PushProvider`. These are separate capabilities with shared auth/client underneath.
2. **Monitoring is workflow-driven** — Slack/channel monitoring happens via a cron workflow that Arawn authors and deploys, not via always-on polling. The workflow pushes relevant information into a session when triggered.
3. **Task tracking requires state** — Arawn needs to remember which tasks it created so it can check status and nag. This ties into the memory system.
4. **Security is critical** — OAuth tokens for Google/Slack must be stored securely and be inaccessible to the agent (see tech debt tickets T-0170 through T-0173 for sandbox hardening prerequisites).

## Goals & Non-Goals

**Goals:**
- Define four capability traits (`TaskListProvider`, `PushProvider`, `ScheduleProvider`, `MessagingProvider`) in a new `arawn-integration` crate
- Implement Google Tasks as first `TaskListProvider`
- Implement Google Calendar as first `ScheduleProvider`
- Implement Slack as first `MessagingProvider` and `PushProvider`
- Per-provider OAuth2 configuration in `arawn.toml` with secure token storage in the data dir
- Tools that expose each capability to the agent: `task_create`, `task_list`, `task_complete`, `push_notify`, `calendar_read`, `calendar_create`, `message_send`, `message_read`
- Nag capability: Arawn can query its own created tasks, detect overdue items, and push reminders
- Workflow template for Slack channel monitoring (cron-based)

**Non-Goals:**
- Always-on real-time event streaming (webhooks, Socket Mode) — cron polling is sufficient for v1
- Multi-workspace Slack (single workspace per config)
- Calendar conflict resolution or smart scheduling
- Provider auto-discovery or marketplace
- Building a full Slack bot with slash commands — this is Arawn reaching out, not Slack reaching in

## Architecture

### Crate Structure

```
arawn-integration/
├── src/
│   ├── lib.rs              — re-exports, IntegrationRegistry
│   ├── traits/
│   │   ├── task_list.rs    — TaskListProvider trait
│   │   ├── push.rs         — PushProvider trait
│   │   ├── schedule.rs     — ScheduleProvider trait
│   │   └── messaging.rs    — MessagingProvider trait
│   ├── auth/
│   │   ├── oauth2.rs       — shared OAuth2 flow (PKCE, token refresh)
│   │   └── token_store.rs  — encrypted token storage
│   ├── providers/
│   │   ├── google/
│   │   │   ├── tasks.rs    — Google Tasks impl
│   │   │   ├── calendar.rs — Google Calendar impl
│   │   │   └── auth.rs     — Google-specific OAuth scopes
│   │   └── slack/
│   │       ├── messaging.rs — Slack MessagingProvider impl
│   │       ├── push.rs      — Slack PushProvider impl (same client)
│   │       └── auth.rs      — Slack OAuth scopes
│   └── config.rs           — IntegrationConfig, provider config sections
```

### Trait Design

```rust
#[async_trait]
pub trait TaskListProvider: Send + Sync {
    async fn create_task(&self, task: NewTask) -> Result<Task, IntegrationError>;
    async fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>, IntegrationError>;
    async fn get_task(&self, id: &str) -> Result<Task, IntegrationError>;
    async fn complete_task(&self, id: &str) -> Result<(), IntegrationError>;
    async fn update_task(&self, id: &str, update: TaskUpdate) -> Result<Task, IntegrationError>;
}

#[async_trait]
pub trait PushProvider: Send + Sync {
    async fn send_notification(&self, notification: Notification) -> Result<(), IntegrationError>;
}

#[async_trait]
pub trait ScheduleProvider: Send + Sync {
    async fn list_events(&self, range: TimeRange) -> Result<Vec<Event>, IntegrationError>;
    async fn create_event(&self, event: NewEvent) -> Result<Event, IntegrationError>;
    async fn update_event(&self, id: &str, update: EventUpdate) -> Result<Event, IntegrationError>;
    async fn delete_event(&self, id: &str) -> Result<(), IntegrationError>;
    async fn free_busy(&self, range: TimeRange) -> Result<Vec<BusySlot>, IntegrationError>;
}

#[async_trait]
pub trait MessagingProvider: Send + Sync {
    async fn send_message(&self, channel: &str, message: Message) -> Result<MessageId, IntegrationError>;
    async fn read_messages(&self, channel: &str, filter: MessageFilter) -> Result<Vec<Message>, IntegrationError>;
    async fn list_channels(&self) -> Result<Vec<Channel>, IntegrationError>;
}
```

### Configuration

```toml
[integrations.google]
provider = "google"
client_id_env = "GOOGLE_CLIENT_ID"
client_secret_env = "GOOGLE_CLIENT_SECRET"
scopes = ["tasks", "calendar"]

[integrations.slack]
provider = "slack"
bot_token_env = "SLACK_BOT_TOKEN"
default_channel = "#arawn-notifications"

[capabilities]
task_list = "google"      # which provider handles task_list
schedule = "google"       # which provider handles schedule
messaging = "slack"       # which provider handles messaging
push = "slack"            # which provider handles push
```

### Auth & Token Security

OAuth2 tokens stored in `{data_dir}/tokens/` as encrypted files. The agent has **no access** to this directory:
- Token directory is excluded from `allowed_paths`
- Token directory added to sensitive path deny list (used by all file/glob/grep tools)
- Token refresh happens inside the provider implementation, not via agent tool calls
- The agent calls capability methods; the provider handles auth transparently

Initial OAuth flow requires user interaction (browser-based consent). This is a one-time setup step triggered by `arawn setup google` or similar CLI command, not by the agent.

### Tool Exposure

Each capability gets tools registered with the engine:

| Tool | Capability | Description |
|------|-----------|-------------|
| `task_create` | TaskList | Create a task with title, notes, due date |
| `task_list` | TaskList | List tasks with optional filter (overdue, today, all) |
| `task_complete` | TaskList | Mark a task done by ID |
| `push_notify` | Push | Send a push notification to the user |
| `calendar_events` | Schedule | List upcoming events in a time range |
| `calendar_create` | Schedule | Create a calendar event |
| `message_send` | Messaging | Send a message to a channel/DM |
| `message_read` | Messaging | Read recent messages from a channel |

### Nag Pattern

Arawn tracks tasks it creates by storing the external task ID in memory (via `memory_store` with entity type "task_reference"). A nag workflow runs on cron, queries the task list for overdue items that Arawn created, and pushes reminders via `push_notify`.

### Monitoring Pattern

Slack channel monitoring is a cron workflow:
1. Arawn authors a monitoring workflow via `workflow_create` (triggered by user request)
2. The workflow runs on a cron schedule (e.g., every 5 minutes)
3. Each run: `message_read` from target channel since last check, filter for relevant messages
4. If relevant: push summary into a session via the engine, or `push_notify` the user

## Alternatives Considered

**1. MCP servers for each integration**
Considered using external MCP servers (like the existing Slack/Google MCP tools in the ecosystem). Rejected because: (a) adds external process management complexity, (b) no control over auth/token security model, (c) can't enforce the provider abstraction pattern, (d) MCP tools don't integrate with the capability trait system.

**2. Direct API calls from shell tool**
The agent could `curl` Slack/Google APIs directly. Rejected because: (a) tokens would need to be in env vars (security nightmare), (b) no abstraction layer, (c) fragile prompt-driven API usage, (d) no token refresh handling.

**3. Single "integration" tool with sub-commands**
One mega-tool like `integration(action: "task.create", ...)`. Rejected in favor of separate tools per capability — cleaner for the LLM to reason about, better permission control, and follows existing tool patterns.

## Implementation Plan

1. **Auth & token storage** — OAuth2 flow, encrypted token store, CLI setup commands, token directory security
2. **Capability traits** — `arawn-integration` crate with four traits and shared types
3. **Google providers** — Google Tasks + Google Calendar implementations
4. **Slack provider** — Messaging + Push implementations
5. **Tools** — Register capability tools with the engine, wire providers via config
6. **Nag workflow** — Task tracking in memory, cron-based overdue check + push
7. **Monitoring template** — Slack channel monitoring workflow template
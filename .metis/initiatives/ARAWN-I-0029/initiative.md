---
id: integration-foundation-oauth
level: initiative
title: "Integration Foundation — OAuth, encrypted token storage, and capability traits"
short_code: "ARAWN-I-0029"
created_at: 2026-04-17T02:46:55.911321+00:00
updated_at: 2026-04-17T12:41:06.083721+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: integration-foundation-oauth
---

# Integration Foundation — OAuth, encrypted token storage, and capability traits Initiative

## Context

Arawn currently only interacts with the user through its TUI/WebSocket interface. To be useful as a day-to-day assistant it needs to reach into productivity tools (task lists, calendars, messaging, push). Rather than building one-off integrations, ARAWN-I-0028 (now segmented into I-0029/I-0030/I-0031) defined four abstract capability traits with concrete provider implementations behind them — the same facade/adapter pattern as the LLM provider system.

This initiative lands the **shared foundation** every provider needs: the trait surface, OAuth2 + token storage, configuration plumbing, and CLI setup commands. No actual providers are implemented here — that's I-0030 (Google) and I-0031 (Slack). This split lets us land the security-critical token-handling work behind a small, reviewable boundary before any external API code goes in.

Token security explicitly builds on the sandbox-hardening work from ARAWN-T-0170 through T-0173: the token directory must be inaccessible to the agent.

## Goals & Non-Goals

**Goals:**
- New `arawn-integration` crate exposing four capability traits: `TaskListProvider`, `PushProvider`, `ScheduleProvider`, `MessagingProvider`, plus shared types (Task, Notification, Event, Message, Channel, etc.).
- OAuth2 flow with PKCE + token refresh, agnostic of any specific provider, in `auth/oauth2.rs`.
- Encrypted token storage under `{data_dir}/tokens/`, with the directory excluded from `allowed_paths` and added to the sensitive-paths deny list.
- CLI command `arawn setup <provider>` that drives the one-time browser-based OAuth consent flow.
- Configuration: `[integrations.*]` sections in `arawn.toml` for per-provider OAuth settings, and a `[capabilities]` section that maps each capability to a provider name.
- An `IntegrationRegistry` that holds the wired provider trait objects, indexed by capability.

**Non-Goals:**
- Any concrete provider implementation (deferred to I-0030/I-0031).
- Always-on event streaming (webhooks, Slack Socket Mode) — providers will use cron polling.
- Multi-account-per-provider support (single account per provider in v1).
- Token rotation policies beyond what the OAuth2 refresh flow gives us.
- A general "credential vault" abstraction beyond what these integrations need.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Architecture

### Crate layout

```
arawn-integration/
├── src/
│   ├── lib.rs              — re-exports + IntegrationRegistry
│   ├── traits/
│   │   ├── task_list.rs    — TaskListProvider
│   │   ├── push.rs         — PushProvider
│   │   ├── schedule.rs     — ScheduleProvider
│   │   └── messaging.rs    — MessagingProvider
│   ├── auth/
│   │   ├── oauth2.rs       — shared PKCE flow + token refresh
│   │   ├── token_store.rs  — encrypted on-disk tokens
│   │   └── server.rs       — local HTTP listener for OAuth callback
│   ├── config.rs           — IntegrationConfig + [capabilities] mapping
│   └── error.rs            — IntegrationError
```

### Trait surface (lands in this initiative — no impls)

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
    async fn send_notification(&self, n: Notification) -> Result<(), IntegrationError>;
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
    async fn send_message(&self, channel: &str, m: Message) -> Result<MessageId, IntegrationError>;
    async fn read_messages(&self, channel: &str, f: MessageFilter) -> Result<Vec<Message>, IntegrationError>;
    async fn list_channels(&self) -> Result<Vec<Channel>, IntegrationError>;
}
```

### Token security

- Token directory: `{data_dir}/tokens/<provider>.json`, encrypted at rest using a key derived from a stable machine-local secret (`age` or platform keyring).
- `arawn-engine`'s `EngineToolContext::allowed_paths` MUST NOT include the tokens directory.
- The sensitive-paths deny list (from ARAWN-T-0171/T-0173) is extended to cover `{data_dir}/tokens/` so glob/grep/file_read/file_write/file_edit all reject access — even via symlinks.
- Token refresh happens inside the provider implementation, not via agent tool calls. The agent calls capability methods; the provider transparently handles auth.

### Configuration shape (arawn.toml)

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
task_list = "google"
schedule  = "google"
messaging = "slack"
push      = "slack"
```

### IntegrationRegistry

A thin holder that, given an `IntegrationConfig` and a set of constructed provider trait objects, exposes:
- `task_list() -> Option<Arc<dyn TaskListProvider>>`
- `push() -> Option<Arc<dyn PushProvider>>`
- `schedule() -> Option<Arc<dyn ScheduleProvider>>`
- `messaging() -> Option<Arc<dyn MessagingProvider>>`

Built once in `main.rs`; passed into `LocalService` (alongside the `LlmClientPool`).

### CLI

`arawn setup <provider>` — opens a browser, runs the local HTTP callback listener, completes PKCE, encrypts and persists the token. Uses the existing `clap` setup added in ARAWN-T-0157.

## Detailed Design

The implementation breaks into the four chunks listed in the Implementation Plan below. Internal-only design decisions worth flagging:

- **Token encryption mechanism**: prefer `age` (from `rage`/`age-encryption.org`) with a key file at `{data_dir}/tokens/.master.age-recipient`. Falls back to OS keyring (`keyring` crate) if the user opts in. Either way, the key is never readable by the agent.
- **OAuth callback port**: bind to `127.0.0.1:0` and pass the resolved port into the redirect URL. Avoids the "fixed port already in use" failure mode.
- **No async trait objects in `Send` boundaries**: stick to `async_trait` macro for now (matches existing tool/LLM trait style); revisit if perf matters.
- **Errors**: `IntegrationError` is `thiserror`-based with variants for `AuthExpired`, `RateLimited { retry_after }`, `ApiError { status, body }`, `Network`, `MissingCapability`. Providers map their native errors into this.

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered

**1. MCP servers for each integration (Slack/Google MCP).**
Rejected because: (a) external process management overhead, (b) no control over auth/token security, (c) can't enforce the trait abstraction we want, (d) MCP tools wouldn't share types with arawn-internal capability code.

**2. Direct API calls from the shell tool.**
Rejected: requires API tokens in env vars (the very thing ARAWN-T-0172 just sanitized), no token refresh, fragile prompt-driven API construction.

**3. Reuse the existing arawn-llm provider abstraction for everything.**
Rejected: LLM clients have a totally different shape (streaming responses, tool calls, model limits) than CRUD APIs. Forcing both through one trait makes both worse.

**4. Skip encryption — store raw tokens with strict file perms.**
Rejected: a single `chmod` slip or a careless `tar` of the data dir leaks every credential. age-based encryption costs us very little and the security gain is large.

## Implementation Plan

Sequential, but each step is independently mergeable:

1. **`arawn-integration` crate skeleton + capability traits + shared types.**
   New crate, four traits with no impls yet, shared types (Task, Notification, Event, Message, etc.), `IntegrationError`. Tests are compile-time only at this stage.

2. **OAuth2 + token storage core.**
   `auth/oauth2.rs` (PKCE flow, refresh logic), `auth/token_store.rs` (age-based encrypt/decrypt of token JSON), `auth/server.rs` (local callback listener). Unit tests with a mock OAuth provider; integration test that round-trips encrypt → write → read → decrypt.

3. **Sandbox/path hardening for tokens.**
   Extend `tools/sensitive_paths.rs` to deny `{data_dir}/tokens/` and any subpath. Verify `EngineToolContext::allowed_paths` excludes it. Tests: glob/grep/file_read targeting `{data_dir}/tokens/*` are denied.

4. **Config + CLI + IntegrationRegistry wiring.**
   `[integrations.*]` and `[capabilities]` parsing in arawn-bin's `config.rs`. New `arawn setup <provider>` clap subcommand that drives the OAuth flow. `IntegrationRegistry` constructed in `main.rs` and passed into `LocalService`. Stub providers (return `MissingCapability`) registered to prove the wiring works end-to-end.

Each step ends in a clean `angreal test unit` green; each is reviewable in isolation.
---
id: google-workspace-providers-tasks
level: initiative
title: "Google Workspace Providers — Tasks and Calendar"
short_code: "ARAWN-I-0030"
created_at: 2026-04-17T02:46:57.119584+00:00
updated_at: 2026-04-17T02:46:57.119584+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: google-workspace-providers-tasks
---

# Google Workspace Providers — Tasks and Calendar Initiative

## Context

First real integrations on top of the foundation laid by ARAWN-I-0029. Google Workspace is a natural starting point: a single OAuth scope set covers two capabilities (Tasks and Calendar), and the APIs are well-documented and stable.

Goals here include the **memory schema for nag tracking** — arawn must remember which tasks it created so it can later check status and remind. The actual nag *workflow* (cron-based reminder push) lives in I-0031 because it depends on Slack push being available; only the data model lands here.

## Goals & Non-Goals

**Goals:**
- `providers/google/tasks.rs` implementing `TaskListProvider` against Google Tasks API.
- `providers/google/calendar.rs` implementing `ScheduleProvider` (events list/create/update/delete + free/busy).
- `providers/google/auth.rs` defining the Google-specific OAuth scopes.
- Shared HTTP client between Tasks and Calendar (single token, single rate-limit budget).
- Engine tools registered: `task_create`, `task_list`, `task_complete`, `task_update`, `calendar_events`, `calendar_create`, `calendar_update`, `calendar_free_busy`.
- Memory entity type `task_reference` (linking arawn-side conversation context to external task IDs) so the I-0031 nag workflow can find arawn's tasks.
- End-to-end UAT scenario: agent creates a task, lists it, marks it complete; agent reads upcoming calendar events; all observable in the user's actual Google account.

**Non-Goals:**
- Slack or any other provider (deferred to I-0031).
- The nag workflow itself (deferred to I-0031 — needs Push).
- Recurring task/event support beyond what Google's API gives us natively.
- Calendar conflict detection or smart scheduling.
- Multi-account Google support.
- Google Drive / Gmail / any other Google API.

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

### Layout under arawn-integration

```
arawn-integration/src/providers/google/
├── mod.rs        — re-exports + GoogleClient (shared http+token wrapper)
├── auth.rs       — Google-specific OAuth scope strings
├── tasks.rs      — TaskListProvider impl
├── calendar.rs   — ScheduleProvider impl
└── types.rs      — wire types (Tasks/Calendar JSON shapes), private to module
```

### Shared `GoogleClient`

A small wrapper holding the encrypted token store handle, a `reqwest::Client`, and scope-set metadata. Both `tasks.rs` and `calendar.rs` borrow this — keeps token refresh logic in one place and ensures the rate-limit budget is honoured across capabilities.

### Tool registration

Each tool is a thin shim that holds an `Arc<dyn TaskListProvider>` (or `ScheduleProvider`) and translates `serde_json::Value` parameters into the trait's typed inputs. Registration happens conditionally based on whether the `IntegrationRegistry` (from I-0029) actually has that capability wired.

### Memory schema for nag tracking

A new entity type registered with `arawn-memory`:

```
task_reference
  external_id: String    — Google task ID
  provider:    String    — "google"
  created_at:  DateTime  — when arawn created it
  due_at:      Option<DateTime>
  conversation_session_id: Uuid  — the session that asked arawn to create it
  description: String    — short summary (for surfacing in reminders)
```

Stored via `memory_store` whenever `task_create` succeeds. Queried by I-0031's nag workflow via `memory_search`.

## Detailed Design

- **Wire types vs domain types**: keep Google's JSON shapes in `types.rs` and only expose the trait's domain types from the module boundary. Avoids leaking Google quirks (e.g., RFC3339 timestamps, weird nullability) into the rest of the codebase.
- **Pagination**: Google APIs paginate with `nextPageToken`. Wrap iteration internally; the trait returns `Vec<T>` already-collected.
- **Rate limiting**: Google's per-user quota is generous for personal use. Add a simple semaphore with a sane default (8 concurrent requests) but don't build adaptive throttling in v1.
- **Time zones**: Google Calendar accepts/returns RFC3339 with explicit offsets; standardise on `chrono::DateTime<Utc>` at the domain boundary.
- **Idempotency**: `task_create` accepts no idempotency token — Google's API doesn't surface one cleanly. Document the limitation; acceptable for human-paced agent use.

## Use Cases

### UC-1: agent creates a task on user request
- **Actor**: user
- **Scenario**: user types "remind me to call the dentist tomorrow at 10"; agent invokes `task_create` with title + due date; tool calls `TaskListProvider::create_task` via the Google provider.
- **Expected**: task appears in user's Google Tasks; arawn confirms with the new task ID; a `task_reference` memory is stored.

### UC-2: agent reads upcoming calendar
- **Actor**: user
- **Scenario**: user asks "what's on my calendar this week"; agent invokes `calendar_events` for the upcoming 7 days; renders the response.
- **Expected**: events match what user sees in Google Calendar.

### UC-3: agent finds a free slot
- **Actor**: user
- **Scenario**: user asks "find me a 30-minute slot tomorrow afternoon"; agent invokes `calendar_free_busy` with the time range; reasons over the busy slots and suggests a time.
- **Expected**: suggestion does not overlap any busy slot.

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

**1. Use the official `google-apis-rs` crates.**
Rejected: those crates are autogenerated, very large, and pull in heavy dependencies. Hand-rolling the few endpoints we need against `reqwest` is smaller and easier to audit.

**2. One provider per Google API (separate `google-tasks` and `google-calendar` modules with separate clients).**
Rejected: forces two separate token files for what is one user account, and breaks the single-OAuth-consent model.

**3. Land the nag workflow here.**
Rejected: nag needs Push, which lives in I-0031. Splitting the data model (here) from the workflow (there) keeps each initiative shippable on its own.

## Implementation Plan

Depends on ARAWN-I-0029 being complete.

1. **GoogleClient + auth scopes.**
   Shared `GoogleClient` wrapping reqwest + token store. `auth.rs` defines scope strings. Wire `arawn setup google` to use these scopes via the foundation's OAuth flow.

2. **TaskListProvider for Google Tasks.**
   Implement all five trait methods. Mock-server tests against captured Google API responses. End-to-end test against a live account behind a `--ignored` flag.

3. **ScheduleProvider for Google Calendar.**
   All five trait methods including `free_busy`. Same testing strategy.

4. **Engine tools + IntegrationRegistry wiring.**
   Register `task_*` and `calendar_*` tools conditionally on capability presence. Inject the providers via `LocalService`. Permission-rule defaults so calendar/task tools require explicit user approval for writes.

5. **Memory schema + task_reference storage on create.**
   Define entity type, store on every successful `task_create`. Unit test that a created task produces a queryable `task_reference` memory.

6. **UAT scenarios.**
   Add UAT cases (live Google account, gated by env var) covering create→list→complete and read calendar.
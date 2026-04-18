---
id: consolidation-cleanup-drop-one
level: initiative
title: "Consolidation cleanup — drop one-impl traits, unify retry + errors"
short_code: "ARAWN-I-0032"
created_at: 2026-04-18T14:12:12.923578+00:00
updated_at: 2026-04-18T14:14:41.564443+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: S
initiative_id: consolidation-cleanup-drop-one
---

# Consolidation cleanup — drop one-impl traits, unify retry + errors Initiative

## Context

Cleanup pass surfaced by the architectural review (commit b6637a0). Four targeted consolidations identified as "low effort, high value." Landed before feature work resumes so the Facility System implementation starts against a tidier baseline.

## Goals & Non-Goals

**Goals:**
- Drop the `LlmResolver` trait (one real impl + one test mock; no abstraction value).
- Consolidate retry logic — the engine's inline retry in `stream_response_with_retry` reimplements what `RetryClient` already does.
- Move `tool_category` from a string switch in `permissions/checker.rs` onto the `Tool` trait as a declared method.
- Unify error conversion at crate boundaries — `ServiceError::Engine(String)` / `Storage(String)` lose context; replace with explicit `From` impls.

**Non-Goals:**
- QueryEngine god-object refactor (deferred).
- `std::sync::Mutex<Store>` → async migration (deferred).
- `#[instrument]` sweep (deferred).
- Security hardening round 2 (deferred).
- `arawn-test-utils` crate (deferred).
- `EngineConfig` ↔ `QueryEngineConfig` collapse (deferred).

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

## Detailed Design

Four independent, order-agnostic cleanups. Each landed as its own commit for reviewability.

### 1. Drop `LlmResolver`
Trait currently lives in `arawn-tool/src/llm_preference.rs`. Only real impl: `LlmClientPool` in `arawn-bin`. Only other impl: a test-only `TestResolver` in `arawn-engine/src/tools/agent.rs`. `EngineToolContext` holds an `Option<Arc<dyn LlmResolver>>`.

Approach: inline `resolve_llm()` by having `EngineToolContext` hold `Option<Arc<LlmClientPool>>` directly, or (better) have `EngineToolContext` store a boxed closure `Option<Arc<dyn Fn(&LlmPreference) -> LlmResolution + Send + Sync>>`. Either way, delete the trait.

Test side: change `TestResolver` to a simple closure + `Arc::new(move |pref| ...)`.

### 2. Consolidate retry
`stream_response_with_retry` in `query_engine.rs` implements exponential-backoff-plus-retry around `LlmClient::chat_stream`. `RetryClient` in `arawn-llm/src/retry.rs` already does this for non-streaming calls. Need to either:
- Extend `RetryClient` to also wrap streaming, then let the engine call an unwrapped `chat_stream` without its own retry, OR
- Keep `RetryClient` as-is but move the streaming retry into `RetryClient::chat_stream` so it's in one place.

Verify: identical retry policy (attempts, backoff) between the two paths today, then merge.

### 3. `tool_category()` on the Tool trait
`permissions/checker.rs::tool_category(name: &str) -> ToolCategory` is a big `match name { "shell" => ..., "file_read" => ... }`. The `Tool` trait already has `fn category(&self) -> ToolCategory` with a `Core` default. Let each tool override it; delete the string switch. Permission checker receives a `&dyn Tool` or `ToolCategory` rather than a name.

### 4. Unify error conversions
Audit boundary From impls. Targets:
- `ServiceError::Engine(String)` → `ServiceError::Engine(EngineError)` with `From<EngineError>`.
- `ServiceError::Storage(String)` → typed.
- `EngineError`/`ToolError`: ensure conversions preserve source.
- WebSocket handler in arawn-bin: stop calling `.to_string()` on typed errors for the wire payload; emit structured error shapes.

Scope-bound: touch conversions only. No signature changes to public APIs unless a boundary is strictly improved.

## Alternatives Considered

**Fold into Facility System implementation.** Rejected — Facility System work is substantial and unrelated; mixing cleanup with feature work creates a messy commit history and harder reviews.

**Defer indefinitely.** Rejected — these are the specific items the architectural review flagged as "do soon." Ignoring them lets the codebase drift further. Two days of cleanup up front beats paying the same tax four times while implementing providers.

## Implementation Plan

One task per goal, sequential or parallel, order-independent:

1. Drop `LlmResolver` trait (T).
2. Consolidate retry logic (T).
3. Move `tool_category` to `Tool` trait (T).
4. Unify error conversions at crate boundaries (T).

Each task ships its own commit with targeted tests. `angreal test unit` green is the exit criterion for each.

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

## Alternatives Considered **[REQUIRED]**

{Alternative approaches and why they were rejected}

## Implementation Plan **[REQUIRED]**

{Phases and timeline for execution}
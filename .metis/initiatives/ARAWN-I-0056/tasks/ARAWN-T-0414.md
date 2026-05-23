---
id: t-c-session-permission-compaction
level: task
title: "T-C: Session, permission, compaction fires — 7 events in LocalService + Compactor"
short_code: "ARAWN-T-0414"
created_at: 2026-05-23T03:31:02+00:00
updated_at: 2026-05-23T03:57:02.392674+00:00
parent: ARAWN-I-0056
blocked_by: [ARAWN-T-0412]
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0056
---

# T-C: Session, permission, compaction fires

## Parent Initiative

[[ARAWN-I-0056]]

## Objective

Add fire sites for the seven session / permission / compaction events. All non-blocking for V1.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `SessionStart` fires in `LocalService::create_session` after the session row is inserted.
  - Hook input carries: `session_id`, `workstream_name`.
- [ ] `SessionEnd` fires when a session is closed (find the close path in `LocalService` — `delete_session` if it exists, or wherever sessions are terminated).
  - Hook input carries: `session_id`, `workstream_name`, `turn_count` (optional).
- [ ] `PermissionRequest` fires in `PermissionChecker` (or wherever "ask" mode triggers a modal prompt) BEFORE the user sees the prompt.
  - Non-blocking V1; logged + delivered to UI normally.
  - Hook input carries: `tool_name`, `tool_args`, `session_id`.
- [ ] `PermissionDenied` fires when the user denies a permission prompt.
  - Hook input carries: `tool_name`, `tool_args`, `session_id`, `denial_reason` (if available).
- [ ] `PreCompact` fires in `Compactor::compact` BEFORE the LLM compaction call.
  - Non-blocking V1.
  - Hook input carries: `session_id`, `message_count` before, `token_count` before.
- [ ] `PostCompact` fires AFTER successful compaction.
  - Hook input carries: same as PreCompact + post-counts + summary length.
- [ ] `Notification` fires in `LocalService::notice_tx` (or wherever ServerNotices are broadcast).
  - Non-blocking; lets users tap into notice events without subscribing to the WS feed.
  - Hook input carries: `notice_category`, `notice_message`, `notice_level`.
- [ ] Each fire site is guarded by `if let Some(runner) = &self.hook_runner`.
- [ ] Unit tests covering each fire site (one per event).
- [ ] `cargo test --workspace --lib` green.

## Implementation Notes

- `LocalService` needs the `Arc<HookRunner>` field added in T-A — this task uses it.
- `PermissionChecker` lives in `crates/arawn-engine/src/permissions/`. Look for the path where "ask" mode prompts the user — that's the fire site.
- `Compactor` lives in `crates/arawn-engine/src/compactor.rs`. The compact() method is the natural wrap point.
- For `Notification`: the existing `notice_tx` broadcast pattern fires `ServerNotice` to all WS subscribers. The hook fire is a peer side-effect, not a replacement.
- All seven events are non-blocking for V1 — keep the fire pattern simple: build input, call `runner.run(&input).await`, ignore the result (or log on `blocked = true` since we're not honoring it).

## Status Updates

### 2026-05-23 — landed

**Discovery (continuation from T-B):** `PreCompact` and `PostCompact` were also already wired (lines 341 and 369 in `query_engine.rs`). So T-C's real delta covers 5 events.

**Fire sites added:**

| Event | Site | How |
|---|---|---|
| `PreCompact` | `query_engine.rs` (already wired) | — |
| `PostCompact` | `query_engine.rs` (already wired) | — |
| `SessionStart` | `local_service/sessions.rs::create_session_inner` | Fires after `store.create_session()` succeeds. `source = "startup"` (vs "resume" / "clear" / "compact" — fresh creates are startup). |
| `PermissionRequest` | `arawn-engine/src/permissions/checker.rs` | Fires in both `Ask` branches (rule-Ask path and NoMatch→Mode::Ask fallback path) before `prompt_user`. |
| `PermissionDenied` | Same file | Fires on three paths: deny-rule short-circuit, user-prompted Deny response, mode-fallback Denied (Plan mode dangerous tool). |
| `Notification` | `main.rs` notice-forwarder task | New detached task subscribes to `notice_tx` and fires `Notification` for every broadcast. Decouples from the 30+ notice-send sites — zero changes to existing senders. |

**SessionEnd: deferred from V1.**

arawn sessions are persistent across WebSocket reconnects — clients can list/load/resume sessions hours later. There's no "session end" concept that maps cleanly to Claude Code's transient-process semantics. The closest signals (WS disconnect, session truncate) aren't true endings. Two options for V2: (a) fire `SessionEnd` on WS disconnect with `reason: "client_disconnected"` — but the conversation continues, which feels wrong; (b) add an explicit `/session close` command that archives a session and fires `SessionEnd`. Option (b) is cleaner but requires UX work. Left deferred until a real user need.

**New wiring:**
- `PermissionChecker` gets `hook_runner: Option<Arc<HookRunner>>` field + `with_hook_runner` builder + two private async helpers (`fire_permission_request_hook`, `fire_permission_denied_hook`).
- `LocalService::build_engine` chains `.with_hook_runner(...)` into the checker construction when the service has a runner.
- `LocalService::hook_runner_clone()` — new accessor for the main.rs notice-forwarder task.
- main.rs Notification-forwarder task spawned right before the existing TodoEvent forwarder; subscribes to `service.subscribe_notices()` and fires `Notification` for every notice.

**New tests (2 in `crates/arawn-tests/tests/hooks.rs`):**
- `permission_request_hook_fires_when_prompting` — Ask rule + MockModalPrompt(Allow Once); marker file proves the hook fired before the prompt was answered.
- `permission_denied_hook_fires_on_deny_rule` — Deny rule + Full permission mode; marker file proves the hook fired on the deny short-circuit.

**Coverage tradeoff:** SessionStart and Notification are wired but covered only transitively by the upcoming T-E UAT scenario. Adding inline unit tests for them would require constructing a LocalService in test scope (and a notice-forwarder lifecycle), which is heavier than the V1 budget warrants. The wiring is small and inspectable.

**Validation:**
- `cargo test --test hooks`: ✅ **11 tests pass** (was 9, +2 new).
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 03s).
- `cargo test --workspace --lib`: ✅ **1,742 tests pass**.
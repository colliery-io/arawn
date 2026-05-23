---
id: t-c-session-permission-compaction-fires
level: task
title: "T-C: Session, permission, compaction fires — 7 events in LocalService + Compactor"
short_code: "ARAWN-T-0414"
created_at: 2026-05-23T03:31:02.000000+00:00
updated_at: 2026-05-23T03:31:02.000000+00:00
parent: ARAWN-I-0056
blocked_by: [ARAWN-T-0412]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0056
---

# T-C: Session, permission, compaction fires

## Parent Initiative

[[ARAWN-I-0056]]

## Objective

Add fire sites for the seven session / permission / compaction events. All non-blocking for V1.

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

*To be added during implementation*

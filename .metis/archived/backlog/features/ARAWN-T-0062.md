---
id: auto-compact-circuit-breaker-retry
level: task
title: "Auto-compact circuit breaker — retry limits and failure tracking"
short_code: "ARAWN-T-0062"
created_at: 2026-04-03T02:03:49.952452+00:00
updated_at: 2026-04-05T12:58:57.376004+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Auto-compact circuit breaker — retry limits and failure tracking

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Add failure tracking and circuit-breaker logic to auto-compaction. Currently if compaction fails (LLM error, timeout), we log and continue — but on the next turn we'll try again and likely fail again, wasting tokens. Claude Code tracks consecutive failures and stops retrying after 3.

### Type: Feature | Priority: P2 | Effort: S

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Track consecutive compaction failures across turns
- [ ] After 3 consecutive failures, stop attempting compaction for the rest of the session
- [ ] Reset failure count on successful compaction
- [ ] Log circuit-breaker state changes (open/closed)
- [ ] Configurable max failures (default 3)

## Implementation Notes

- Add `consecutive_compact_failures: u32` to engine state (or Compactor)
- In `QueryEngine::run()`, check failure count before calling `compactor.compact()`
- On failure: increment counter, log warning
- On success: reset counter
- At max: log "compaction circuit breaker open, skipping" and continue without compacting
- Reference: Claude Code's `autoCompact.ts` consecutive failure tracking
- Effort: S — small change to existing compaction flow
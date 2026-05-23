---
id: operational-reliability
level: initiative
title: "Operational Reliability: Cancellation, Graceful Shutdown, and JSONL Durability"
short_code: "ARAWN-I-0023"
created_at: 2026-04-09T23:59:36.439892+00:00
updated_at: 2026-04-10T23:28:02.910996+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: operational-reliability
---

# Operational Reliability: Cancellation, Graceful Shutdown, and JSONL Durability Initiative

## Context

Architecture review found that cancellation is acknowledged but unimplemented (OPS-10), there's no signal handling for graceful shutdown (OPS-01), JSONL persistence has 4 related fragility issues (COR-008, PERF-03, OPS-06, EVO-07), and session promotion is non-atomic (COR-004). These collectively mean the system can lose data or waste resources without user awareness.

**Review findings addressed:** R-07, R-08, R-11, R-13

## Goals & Non-Goals

**Goals:**
- Implement real cancellation using CancellationToken, checked at each engine loop iteration (OPS-10)
- Harden JSONL: skip-bad-lines recovery, summary offset seeking, version header, batched writes (COR-008, PERF-03, OPS-06, EVO-07)
- Fix promotion atomicity: move file before updating SQLite (COR-004)
- Add signal handling and graceful shutdown with in-flight task cancellation (OPS-01)

**Non-Goals:**
- Replacing JSONL with a different storage format
- Distributed/cloud deployment concerns

## Detailed Design

### Cancellation (R-07)
Add `CancellationToken` per session in `LocalService` (can share DashMap from per-session lock in ARAWN-I-0021). Pass token into `QueryEngine`. Check `token.is_cancelled()` at loop top and before each tool execution. Emit `EngineEvent::Error("Cancelled by user")` and break.

### JSONL Hardening (R-08)
1. `load()`: wrap `serde_json::from_str` in match, skip+log bad lines
2. Add `last_summary_offset` column to sessions SQLite table, seek on load
3. Add `{"_version": 1}` header line to new JSONL files
4. Batch per-turn message appends into single file open/write/close

### Promotion Atomicity (R-11)
Reverse operation order: copy JSONL first, then update SQLite. If SQLite update fails, remove the copy. Add startup reconciliation check.

### Graceful Shutdown (R-13)
Use `axum::serve(...).with_graceful_shutdown(shutdown_signal())`. On shutdown: cancel all active engine tasks via CancellationTokens, give background tasks 5s timeout, then exit.

## Implementation Plan

1. Cancellation (Days) — prerequisite for graceful shutdown
2. JSONL hardening (Days) — independent
3. Promotion atomicity (Hours) — independent
4. Graceful shutdown (Days) — depends on cancellation
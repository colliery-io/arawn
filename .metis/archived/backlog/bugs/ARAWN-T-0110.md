---
id: stale-sessions-remain-in-tui
level: task
title: "Stale sessions remain in TUI sidebar after deletion from disk"
short_code: "ARAWN-T-0110"
created_at: 2026-04-05T19:08:43.482605+00:00
updated_at: 2026-04-05T21:28:19.249695+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Stale sessions remain in TUI sidebar after deletion from disk

## Objective

Session directories deleted from disk (`rm -rf ~/.arawn/workstreams/scratch/*/`) still appear in the TUI sidebar. The sidebar loads sessions from SQLite on startup and doesn't detect when the backing JSONL files are gone.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Reproduction Steps
1. Start arawn TUI, create several sessions
2. Stop arawn
3. `rm -rf ~/.arawn/workstreams/scratch/*/`
4. Start arawn TUI again
5. Sidebar still shows all old sessions

### Expected vs Actual
- **Expected**: Sidebar shows no sessions (or only valid ones)
- **Actual**: All deleted sessions still listed. Clicking them likely errors or shows empty.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] On startup, session list reconciles SQLite records with filesystem — removes entries whose JSONL files are missing
- [ ] Or: add a `/sessions clear` command to purge stale entries
- [ ] Sidebar accurately reflects sessions that actually exist on disk

## Implementation Notes

Sessions are tracked in SQLite (`sessions` table) but messages live in JSONL files. The store loads session metadata from SQLite without checking if the backing file exists. Fix could be either:
- **Startup reconciliation**: scan SQLite sessions, verify each has a corresponding JSONL, delete orphans
- **Lazy validation**: when loading a session for display, check JSONL exists, remove if not

## Status Updates

### 2026-04-05 — Complete
- Added `SessionStore::delete(session_id)` method for removing SQLite session records
- Added `Store::reconcile_sessions()` — scans all sessions (scratch + workstream-bound), checks if JSONL file exists on disk, deletes SQLite records for orphans
- Called `reconcile_sessions()` on server startup in `main.rs`
- Logs removed count at INFO level when stale sessions are cleaned up
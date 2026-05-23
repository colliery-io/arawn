---
id: unified-store-session-promotion
level: task
title: "Unified Store + session promotion (SQLite update + file move)"
short_code: "ARAWN-T-0014"
created_at: 2026-03-31T22:52:37.006618+00:00
updated_at: 2026-03-31T23:14:06.701827+00:00
parent: ARAWN-I-0002
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0002
---

# Unified Store + session promotion (SQLite update + file move)

## Parent Initiative
[[ARAWN-I-0002]]

## Objective
Compose SQLite stores and JSONL message store into a unified `Store` struct that provides a single interface for all persistence operations. Implement session promotion as an atomic operation across both backends.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `Store` struct composing `Database`, `SqliteWorkstreamStore`, `SqliteSessionStore`, `JsonlMessageStore`
- [ ] `Store::open(data_dir)` — creates directories, opens DB, runs migrations, returns ready-to-use Store
- [ ] Delegates workstream/session ops to SQLite, message ops to JSONL
- [ ] `Store::promote_session(session_id, new_ws_id)` — updates SQLite workstream_id + moves JSONL file + updates domain `Session::promote()`
- [ ] `Store::create_session_with_workstream(ws_id)` — creates session in SQLite + ensures JSONL directory exists
- [ ] `Store::create_scratch_session()` — creates session with NULL workstream_id + scratch JSONL path
- [ ] Test: full promotion flow — create scratch session, append messages, promote, verify messages loadable from new location
- [ ] Test: Store::open on fresh directory creates all subdirectories
- [ ] Promotion fails gracefully if session is already bound (not scratch)

## Implementation Notes
- `store.rs` in `crates/arawn-storage/src/`
- `Store` owns `Database` + `JsonlMessageStore` and exposes high-level methods
- Promotion is NOT truly atomic (SQLite update + fs move are separate operations). If the fs move fails after SQLite update, we have an inconsistent state. For v1 this is acceptable — single user, local filesystem. Log a warning if the move fails so it can be recovered manually.
- Tests use tempdir for both SQLite (in temp path) and JSONL
- Depends on: ARAWN-T-0010 (SQLite stores), ARAWN-T-0011 (JSONL store)

## Status Updates
- **2026-03-31**: Complete. Unified Store composing Database + JsonlMessageStore. High-level methods: create/get/list workstreams, create/get/list sessions, append/load messages, load_session (full reconstruct with messages), promote_session (SQLite update + JSONL file move). Added Session::from_parts to arawn-core for DB reconstruction. 8 new tests including full promotion flow. 34 total storage tests, clippy clean.
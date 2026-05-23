---
id: wire-storage-into-binary-auto
level: task
title: "Wire storage into binary — auto-create ~/.arawn/, persist sessions, append messages"
short_code: "ARAWN-T-0013"
created_at: 2026-03-31T22:49:36.158453+00:00
updated_at: 2026-03-31T23:32:56.933948+00:00
parent: ARAWN-I-0002
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0002
---

# Wire storage into binary — auto-create ~/.arawn/, persist sessions, append messages

## Parent Initiative
[[ARAWN-I-0002]]

## Objective
Wire the `Store` into the binary crate so that sessions and messages survive across runs. On startup: open store, create/load workstream and session. During engine loop: append messages to JSONL after each turn. On next run: resume or list previous sessions.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] Binary opens `Store::open(~/.arawn/)` on startup, creating directory + DB if needed
- [ ] Creates or reuses a scratch workstream (idempotent — check if "scratch" exists in DB first)
- [ ] Creates a new session in the store, or resumes an existing one (via `--session <id>` flag)
- [ ] Messages appended to JSONL after each user message, assistant response, and tool result
- [ ] Session metadata persisted in SQLite (visible via `--list-sessions` flag)
- [ ] Run arawn twice → second run can list the first session
- [ ] Run arawn with `--session <id>` → loads previous messages and continues conversation
- [ ] QueryEngine still works identically — persistence is transparent to the engine
- [ ] All existing tests still pass (engine tests use in-memory, unaffected)

## Implementation Notes
- Update `crates/arawn/Cargo.toml` to depend on `arawn-storage`
- `main.rs` changes: open Store early, create session via store, hook message appends into the engine loop
- The engine's `Session` object is populated from `store.load_messages()` on resume
- After `QueryEngine::run`, the new messages are already appended (done during the loop)
- Add `--list-sessions` and `--session <uuid>` CLI flags (simple `std::env::args` parsing or add `clap`)
- Data dir: `~/.arawn/` default, overridable via `ARAWN_DATA_DIR` env var
- Depends on: ARAWN-T-0014 (unified Store)

## Status Updates
- **2026-03-31**: Complete. Binary wired to Store. On startup: opens ~/.arawn/ (or ARAWN_DATA_DIR), creates DB + directory layout, creates/reuses scratch workstream idempotently. CLI flags: --list-sessions, --session <uuid> for resume. User message persisted before engine run, all new messages (assistant/tool) persisted after. Session ID printed to stderr so user can resume. 105 total workspace tests, clippy clean. Verified: ~/.arawn/ created with arawn.db, workstreams/, scratch/sessions/.
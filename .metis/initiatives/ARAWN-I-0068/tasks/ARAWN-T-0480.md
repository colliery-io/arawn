---
id: p2-4-store-promote-session-atomic
level: task
title: "P2-4: Store::promote_session + atomic lens creation + /lens promote (closes ARAWN-T-0012)"
short_code: "ARAWN-T-0480"
created_at: 2026-06-12T12:02:14.472624+00:00
updated_at: 2026-06-12T12:02:14.472624+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0068
---

# P2-4: Store::promote_session + atomic lens creation + /lens promote (closes ARAWN-T-0012)

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0068]] — implements P2-4 (HIGH, verified). **Closes the long-standing ARAWN-T-0012** (Unified Store + session promotion).

## Objective **[REQUIRED]**

Implement atomic session promotion (scratch → named lens) and atomic lens creation, and expose a `/lens promote` command. The scratch→lens promotion workflow from the vision currently cannot be performed at all.

**The defect** (`arawn-storage/src/store.rs:65-81`, `jsonl.rs:157`): `JsonlMessageStore::move_session()` exists and `SessionStore` can update rows, but no `Store::promote_session(session_id, new_lens_id)` composes the SQLite update + JSONL file move atomically. Separately, `create_lens` inserts the SQLite row then `create_dir_all` — a mkdir failure leaves a lens row with no directory and no rollback.

### Type
- [x] Feature — completes a core vision workflow (closes T-0012)

### Priority
- [x] P2 - Medium (HIGH finding; a missing core workflow)

## Acceptance Criteria **[REQUIRED]**

- [ ] `Store::promote_session(session_id, new_lens_id)` atomically updates `sessions.lens_id`/`lens_name` in SQLite AND moves the JSONL file (`JsonlMessageStore::move_session`), rolling back the SQLite change if the file move fails (and vice versa).
- [ ] `create_lens` is made atomic: mkdir-first, or roll back the SQLite row if mkdir fails — no orphaned lens rows.
- [ ] A `/lens promote <session-id>` RPC + TUI command drives it.
- [ ] Tests (in [[ARAWN-T-0483]] or inline): promotion round-trips; an injected mkdir/move failure leaves state consistent (no orphan, no half-promotion).
- [ ] `angreal check all` and `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Compose the existing pieces in `arawn-storage::Store`: begin a SQLite transaction, update the session row, move the JSONL file, commit — on any failure, roll back the txn and undo the file move (move it back). For `create_lens`, create the directory before (or transactionally with) the row insert. Add the `/lens promote` RPC to `ArawnService` + a TUI command.

### Dependencies
Failure-injection coverage lands with [[ARAWN-T-0483]]. The TUI `/lens promote` also unblocks the GUI session-promotion requirement noted in I-0069's deferred list.

### Risk Considerations
The SQLite-update + file-move spans two stores — the rollback path (undo the file move if the txn fails, or vice versa) is the crux; test it under injected failure. A crash mid-promote must not orphan the session.

## Status Updates **[REQUIRED]**

*To be added during implementation*
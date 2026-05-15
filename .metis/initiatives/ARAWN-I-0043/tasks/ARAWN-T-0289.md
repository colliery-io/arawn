---
---
id: diary-capture-ceremonies-upsert
level: task
title: "Diary capture — ceremonies.upsert_diary RPC + status transitions"
short_code: "ARAWN-T-0289"
created_at: 2026-05-15T23:45:50.098252+00:00
updated_at: 2026-05-15T23:45:50.098252+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Diary capture — upsert_diary RPC + status transitions

## Goal
The retro's third section (user diary) is captured via a dedicated RPC method. Engine maintains tablet status transitions: `open` (default) → `reviewed` (user wrote diary) → `unreviewed` (Sunday night auto-transition if no diary).

## Reference
I-0043 §Diary capture.

## Acceptance
- New RPC method `ceremonies.upsert_diary { tablet_id, body }` — writes/replaces `ceremony_diary` row; flips `ceremony_tablets.status = "reviewed"`; emits `EngineEvent::Ceremony(DiaryUpdated)`.
- `body` stored verbatim — no transformations, no parsing.
- `word_count` computed by simple whitespace split.
- A nightly cloacina job at 23:59 Sunday transitions any retro tablet with `status = "open"` and no `ceremony_diary` row to `status = "unreviewed"`.
- Tests: upsert flow, idempotent re-upsert, Sunday-night transition with fake clock.

## Out of scope
TUI editor (T-0290).

## Notes
This is the only place the user-write path (T-0282 `write_user_item`) gets used by retro. Daily/weekly will reuse the same path for user-added todos and priorities.

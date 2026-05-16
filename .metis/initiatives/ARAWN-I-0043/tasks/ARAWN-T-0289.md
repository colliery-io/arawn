---
id: diary-capture-ceremonies-upsert
level: task
title: "Diary capture — ceremonies.upsert_diary RPC + status transitions"
short_code: "ARAWN-T-0289"
created_at: 2026-05-15T23:45:50.098252+00:00
updated_at: 2026-05-16T03:10:19.168474+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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
## Status Updates

**2026-05-16 — implementation landed.**

- `CeremonyService::upsert_diary(tablet_id, body)`:
  - Verifies the tablet exists and is kind=`retro` (diary doesn't apply elsewhere); errors with `InvalidTabletState` otherwise.
  - `INSERT OR REPLACE` on `ceremony_diary` (PK by `tablet_id`) — re-upsert replaces the body, updates `written_at` to now, recomputes `word_count` (simple whitespace split).
  - Flips `ceremony_tablets.status` from `open` → `reviewed` (leaves `reviewed`, `unreviewed`, `archived` alone).
  - Emits `CeremonyEvent::DiaryUpdated { tablet_id }` if events are wired.

- New `crates/arawn-ceremonies/src/nightly.rs`:
  - `sweep_unreviewed_retros(conn)` — UPDATE that transitions retro tablets with `status='open'` AND no diary row AND `period_key < current_iso_week` to `status='unreviewed'`. Idempotent (returns 0 on the second call). Cron expression (`59 23 * * SUN` in user's local tz) lives in the binary's wiring; the function itself is timezone-agnostic — it only acts on prior-week retros.

**Tests (11 new, 71 total in the crate):**

`service::tests` (5 new):
- `upsert_diary_writes_row_and_flips_status` — body verbatim, word_count correct, tablet → `reviewed`.
- `upsert_diary_is_idempotent_and_replaces_body` — re-upsert replaces body, status stays `reviewed`.
- `upsert_diary_rejects_non_retro_tablet` — diary on a daily tablet errors with `InvalidTabletState`.
- `upsert_diary_rejects_unknown_tablet` — same error variant for missing id.
- `upsert_diary_emits_diary_updated_event` — broadcast event verified.

`nightly::tests` (6 new):
- `open_retro_from_prior_week_without_diary_transitions` — happy path.
- `open_retro_with_diary_is_left_alone`.
- `reviewed_retro_is_left_alone`.
- `current_week_open_retro_is_skipped` — guards the "give it time" window.
- `sweep_is_idempotent`.
- `sweep_only_touches_retros` — doesn't accidentally promote `daily` tablets to `unreviewed`.

Next: T-0290 (/retro TUI client).
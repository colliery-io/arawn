---
id: ceremony-sqlite-schema-migration
level: task
title: "Ceremony SQLite schema migration in arawn-storage"
short_code: "ARAWN-T-0280"
created_at: 2026-05-15T23:44:50.940827+00:00
updated_at: 2026-05-16T00:23:03.929799+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Ceremony SQLite schema migration

## Goal
One migration file `crates/arawn-storage/migrations/00NN_ceremonies.sql` that creates every ceremony table the engine + retro plugin need. I-0041 and I-0042 inherit this schema without further migrations.

## Reference
I-0043 §Data model — the prerequisite. Tables:
- `ceremony_tablets` — kind/period_key/generated_at/status/workstreams_scanned/priorities_confirmed_at
- `ceremony_sections` — tablet_id/section_key/ordinal/title
- `ceremony_items` — id/tablet_id/section_key/ordinal/kind/body/citation_id/done_at/created_at
- `ceremony_todos_rolling` — todo_id/body/origin_tablet_id/created_at/done_at/last_seen_tablet_id
- `ceremony_priorities` — id/tablet_id/body/rationale/citation_id/confirmed_at/done_at/ordinal
- `ceremony_activity_rollup` — iso_week/workstream/metric_key/value (PK composite)
- `ceremony_patterns_detected` — id/iso_week/pattern_key/magnitude/payload/surfaced_in_retro
- `ceremony_diary` — tablet_id PK/body/written_at/word_count

## Acceptance
- One `.sql` file in the migrations dir, numbered after the latest existing one.
- All 8 tables created in a single migration; foreign keys explicit.
- Refinery migration runs against a fresh DB; existing `arawn-storage` tests still pass.
- Add a small integration test in `arawn-storage` that opens a fresh DB and confirms every `ceremony_*` table exists.

## Out of scope
The Rust types that map onto these tables — those land with the engine + plugin (T-0282, T-0287).

## Notes
The `citation_id` column is `TEXT NULL` because the user-write path bypasses citation. Enforcement is Rust-side via the two-write-path API, not a NOT NULL constraint.
## Status Updates

**2026-05-16 — implementation landed.**

- New migration `crates/arawn-storage/migrations/V6__ceremonies.sql`. Numbered after V5 (extractor cursors). One file, eight tables, single migration.
- Schema:
  - `ceremony_tablets` (PK id, UNIQUE (kind, period_key)) — top-level row, `priorities_confirmed_at` null on non-weekly.
  - `ceremony_sections` (PK tablet_id+section_key) — declared section ordering.
  - `ceremony_items` (PK id) — `citation_id` nullable (user-write path), `body` JSON.
  - `ceremony_todos_rolling` (PK todo_id) — cross-day persistence.
  - `ceremony_priorities` (PK id) — weekly candidates + confirmation timestamp.
  - `ceremony_activity_rollup` (PK iso_week+workstream+metric_key) — generic per-week aggregate.
  - `ceremony_patterns_detected` (PK id) — pattern findings with cited source rows in `payload`.
  - `ceremony_diary` (PK tablet_id) — user-written diary body + word count.
- ON DELETE CASCADE wired wherever a parent row exists, so wiping a tablet drops every dependent row in one transaction.
- Indexes on common query paths: `(tablet_id)`, `(tablet_id, section_key, ordinal)`, `(kind)`, `(status)`, `(iso_week)`, `(confirmed_at)`, `(last_seen_tablet_id)`.
- Three integration tests in `arawn-storage::database::tests`:
  - `v6_ceremony_tables_present` — every table is created.
  - `v6_ceremony_tablets_accepts_a_row_and_uniques_on_kind_period` — UNIQUE enforcement.
  - `v6_ceremony_items_accepts_null_citation_for_user_path` — confirms the schema permits the user-write path.
- All 57 pre-existing `arawn-storage` tests still pass.

Next: T-0281 (cloacina workflow runner).
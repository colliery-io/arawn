---
id: plumb-since-into-gmail-drive
level: task
title: "Plumb `since=` into gmail/*, drive/recent, jira/* (cold-start backfill)"
short_code: "ARAWN-T-0233"
created_at: 2026-05-10T00:00:00+00:00
updated_at: 2026-05-10T14:43:25.331022+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Plumb `since=` into gmail/*, drive/recent, jira/* (cold-start backfill)

## Parent Initiative

[[ARAWN-I-0039]]

## Severity

P2 — slack-side `since=` already landed (T-0227). The same per-call cap that made slack cold-starts slow exists in 6 more templates. Surfaced via the audit at the end of T-0218 UAT.

## Scope narrowed during implementation

Walking through the design surfaced that **Gmail and Drive don't actually converge with the existing spawn-loop model**, even with `since=` plumbed in. Both APIs return *most-recent-first* within a query window. After call 1 writes 100, cursor advances to the newest ts; call 2's filter (`after:<cursor>`) returns 0 because nothing is newer than the newest. Loop exits with only 100 written, not the N total in the backfill window.

Fixing them needs adapter-level pagination support (follow `pageToken` / `nextPageToken` until exhausted, up to a cap), which is materially more than 5-10 LOC. Splitting:

- **This task (T-0233): Jira only.** Both Jira templates work cleanly because their JQL `ORDER BY updated ASC` returns oldest-first within the window. After writing 100 oldest, cursor advances to the max-of-batch (newest of the batch); next call returns the next 100 oldest after that. Loop converges naturally when fewer than 100 returned.
- **Spun off as T-0234: Gmail + Drive.** Needs the adapter pagination work plus a different cursor model (walk-backward via `before:` operator, or pageToken-as-cursor).

| Template | Per-call cap | What `since=` should drive | Status |
|---|---|---|---|
| `jira/project-tracker` | 100/call | JQL `updated >= "<since>"` substituted when cursor is null. | **landed in this task** |
| `jira/assignee-tracker` | 100/call | Same. | **landed in this task** |
| `gmail/inbox-archive` | 100/call | needs `pageToken` walk + walk-backward cursor | T-0234 |
| `gmail/sender-filter` | 100/call | same | T-0234 |
| `gmail/label-archive` | 100/call | same | T-0234 |
| `drive/recent` | 200/call | same | T-0234 |

## What's already wired (no work needed)

- `parse_watch_args` accepts `since=<duration|date>` and resolves to canonical RFC3339 (T-0227).
- `register_feed_dynamic` detects `params.since` and dispatches to `spawn_backfill_task` instead of immediate cron registration (T-0227).
- The backfill loop calls `dispatch::run_feed_force` repeatedly until `items_written == 0`, then flips `enabled=1` and registers cron (T-0227).
- Boot resumption picks up interrupted backfills via `last_status="backfilling"` (T-0227).
- The cursor-monotonic `if x > prior` advance pattern is already in every template under change (audit confirmed).

So the per-template work is purely: **plumb `params.since` into the time-floor on first run**. ~5–10 LOC each.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **Gmail** (3 templates) — `archive_query` in `templates/gmail/common.rs` accepts an optional first-run `since` ISO datetime. When `cursor.latest_internal_date` is null AND `since` is set, augment the query string with `after:<unix_ts_secs>`. Templates already pass through their `params`; the helper consumes `params.get_str("since")`.
- [ ] **Drive recent** — `templates/drive/recent.rs::run` already computes `since` from `days_back` when cursor is null. Add: if `params.since` is set, use it instead of `Utc::now() - Duration::days(days_back)`.
- [ ] **Jira** (2 templates) — `build_jql` in both `project_tracker.rs` and `assignee_tracker.rs` already produces `updated >= "<cursor>"`. When cursor is null AND `params.since` is set, substitute the since value into the same clause.
- [ ] **Tests** — one parser-style test per provider asserting `since=180d` propagates correctly into the query/JQL/time_min on first run, and is ignored once cursor exists.
- [ ] **`validate(params)`** for each template accepts `since` as a known optional key (no rejection).
- [ ] `angreal check workspace` and `angreal check clippy` clean.
- [ ] Smoke-test in UAT: `/watch gmail/inbox-archive me since=30d` lands a populated archive in one spawn-loop run instead of N */15 ticks.

## Out of scope (deferred)

- **`slack/my-mentions`** — uses `search.messages` with day-grained `after:` filter. The `since=` plumbing here would round to a day, which is a separate decision (probably fine, but noting it's a different shape than the others).
- **`confluence/space-archive` and `drive/folder-sync`** — already paginate fully per call; no cold-start gap to close.
- **`calendar/upcoming-archive`** — looks forward only; `since=` is meaningless.
- **Per-call pagination** (loop `next_cursor` within a single `run`). Backfill spawn loop addresses cold-start adequately; in-tick pagination would help only the steady-state case where a tick produces > cap items between firings, which is rare enough to defer.

## Status Updates

*To be added during implementation*
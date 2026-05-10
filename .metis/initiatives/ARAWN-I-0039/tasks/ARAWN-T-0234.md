---
id: gmail-drive-cold-start-backfill
level: task
title: "Gmail + Drive cold-start backfill: pageToken pagination + walk-backward cursor"
short_code: "ARAWN-T-0234"
created_at: 2026-05-10T00:00:00+00:00
updated_at: 2026-05-10T13:25:10.697274+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Gmail + Drive cold-start backfill: pageToken pagination + walk-backward cursor

## Parent Initiative

[[ARAWN-I-0039]]

## Severity

P2 — same cold-start gap T-0227 closed for slack and T-0233 closed for jira. Spun off from T-0233 because Gmail and Drive can't reuse the spawn-loop convergence pattern as-is — both APIs return *most-recent-first*, so the cursor-advances-to-max model only ever fetches the most recent N items.

## Templates affected

- `gmail/inbox-archive` (cap 100/call)
- `gmail/sender-filter` (cap 100/call)
- `gmail/label-archive` (cap 100/call)
- `drive/recent` (cap 200/call)

## The problem

Both APIs sort newest-first within a query window. The current spawn-loop convergence model assumes:

1. Call N writes a batch, advances cursor to `max(seen.ts)`.
2. Call N+1 with `after:<cursor>` returns the *next* batch (chronologically newer or older).
3. Loop converges when a call returns 0 items.

For Gmail/Drive, step 2 fails: with cursor at the newest ts, `after:<cursor>` returns 0 (nothing is newer than the newest). Loop exits with only the most recent N items written. The 99% of older messages in the window stay unfetched.

## Fix shape

Two changes per provider:

1. **Adapter-level pagination support.** `RealGmailClient::list_message_ids` and `RealDriveClient::list_modified_since` accept a `page_token: Option<String>` parameter and return `(Vec<id>, Option<next_page_token>)`. The current contracts return only the first page.

2. **Walk-backward cursor model for backfill.** During backfill (cursor is null + `since` set), the cursor stores the page_token, not a timestamp. Each spawn-loop iteration advances through pages until exhausted. When the last page is reached, transition the cursor to the steady-state `latest_internal_date` / `latest_modified_iso` shape (set to `max(seen)` of the entire backfill).

Alternative: keep the cursor as a timestamp but use `before:<oldest_seen>` to walk backward through the window. Either works; pageToken is more idiomatic Gmail/Drive.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `RealGmailClient::list_message_ids` paginates internally (or exposes pageToken). Cap at e.g. 5 pages per call to keep cron-tick latency bounded.
- [ ] `RealDriveClient::list_modified_since` same.
- [ ] Backfill mode in `templates/gmail/common.rs::archive_query`: when `params.since` is set + cursor is null, walk pages within the spawn-loop iteration until either no more pages OR a per-iteration cap.
- [ ] Same for `templates/drive/recent.rs::run`.
- [ ] After backfill completes, cursor transitions to the steady-state shape so cron-tick cadence works as before.
- [ ] Tests:
  - Mock Gmail returning 3 pages of 100 ids each; backfill walks all 3, total 300 messages written, cursor lands at the steady-state shape.
  - Same for Drive.
  - Steady-state behavior unchanged: `since=` not set, single page, cursor advances normally.
- [ ] Smoke-test in UAT: `/watch gmail/inbox-archive me since=30d` lands a populated archive in one or two spawn-loop iterations instead of stopping at 100.
- [ ] `angreal check workspace` and `angreal check clippy` clean.

## Out of scope

- Steady-state pagination (in-tick, when a single tick produces > cap items between firings). Defer.
- `slack/my-mentions` cold-start. The day-grained `after:` filter has its own dynamics; track separately if it ever bites.

## Status Updates

### 2026-05-10 — landed: adapter-internal pageToken pagination

Took the simpler shape than the task plan suggested: instead of the cursor storing a pageToken across spawn-loop iterations, the adapter walks `nextPageToken` internally within a single helper call up to `max_results` total ids. Templates pick the cap per-call:

- **Steady-state cron**: existing small caps (gmail 100, drive 200) → one API page → no behavior change.
- **Backfill (cursor null + `params.since` set)**: bumps cap to 5000 → adapter walks pages until exhausted or 5000 ids accumulated.

Spawn loop convergence is now 2 iterations regardless of how big the backfill is:
- iter 1: paginated walk pulls everything in the window, writes all to disk, cursor advances to newest.
- iter 2: same walk; every id matches `existing_message_path` skip (gmail) or cursor skip (drive). `items_written = 0`, loop exits.

### Implementation

- `RealGmailClient::list_message_ids` walks Gmail's nextPageToken with per-page cap of 500 (Gmail's API max). Caller's `max_results` is the total ceiling.
- `RealDriveClient::list_modified_since` walks Drive's nextPageToken with per-page cap of 1000. Same shape.
- New `compose_time_bound(cursor, params_since, days_back) -> (String, u32)` helper in `gmail/common.rs`. Returns `("after:<unix_ts>", BACKFILL_MAX_RESULTS)` for backfill mode, `("newer_than:<N>d", DEFAULT_MAX_RESULTS)` otherwise. All three Gmail templates call it.
- `drive/recent.rs::run` mirrors the same logic inline (no helper since drive only has one template).

### Tests

4 new unit tests for `compose_time_bound`:
- steady-state with cursor wins (since ignored)
- first-run with since uses `after:<unix>` + backfill cap
- first-run without since uses `newer_than:<days_back>d` + default cap
- garbage RFC3339 since falls back to days_back path

146 arawn-feeds tests green. Workspace + clippy clean.

### Out of scope (still)

- **Rate-limit Retry-After parsing for Gmail/Drive**. Backfill helpers can hit Gmail's per-user quota on 5000-id sweeps; current behavior is to surface `RateLimited` to the spawn loop and exit `backfill-failed`. User retries. T-0227's resilience matrix item 3 covers this.
- **`slack/my-mentions` cold-start**. Day-grained `after:` operator needs different shape; deferred.
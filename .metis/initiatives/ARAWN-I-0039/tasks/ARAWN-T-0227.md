---
id: backfill-escape-hatch-feeds
level: task
title: "Optional `since=` arg on /watch — backfill loop before cron starts"
short_code: "ARAWN-T-0227"
created_at: 2026-05-09T00:00:00+00:00
updated_at: 2026-05-09T00:00:00+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Optional `since=` arg on `/watch` — backfill loop before cron starts

## Parent Initiative

[[ARAWN-I-0039]]

## Objective

Surfaced during T-0218 UAT on `slack/channel-archive`: first run pulls only ~200 messages (Slack's `conversations.history` default page) and stops. Active channels with months of history get a tiny seed and the historical gap stays a gap forever.

The clean UX is **an optional `since=` arg on `/watch`**, but instead of letting cron firings pace the catch-up over hours/days, we run the backfill **as a single tokio loop at registration time, before the cron schedule starts**:

```
/watch slack/channel-archive design channel=#design since=180d
```

```
register_feed_dynamic:
  1. Validate template / params / cadence.
  2. Insert DB row with enabled=0  (so cron won't pick it up yet).
  3. Write initial meta.json with cursor=null, last_status="backfilling".
  4. Spawn tokio task — does NOT block the slash-command response.
  5. Return registration ack to the user (TUI shows "backfilling…").

backfill loop (in the spawned task):
  while true:
      outcome = dispatch::run_feed(feed_id, runtime)
        # On first call: template sees null cursor + params.since,
        # uses since as oldest_ts.
        # Subsequent calls: cursor takes over, walks forward 200 msgs/call.
      if outcome.summary.items_written == 0:
          break   # caught up to now
      # Optional small sleep between pages (politeness, not rate-limit
      # avoidance — backoff is the adapter's job).
  flip row enabled=1.
  register cloacina cron schedule.
  broadcast [feeds] backfill complete ServerNotice with item count + duration.

steady-state:
  cron fires on cadence, normal incremental run() each tick.
```

## Why this is better than the cron-paced version

- **Bounded only by Slack rate limits, not cron cadence.** A 10k-msg channel that would take 12 hours of `*/15` ticks at 200 msg/tick now finishes in a couple minutes (~50 pages × ~1s/page).
- **Cron schedule never sees the backfill.** No risk of cron + backfill writing to the same `meta.json` simultaneously, no need to coordinate.
- **One concept, not two.** Still just `since=` at registration. No new command, no separate backfill code path. The loop calls the existing `run_feed` repeatedly — same code that cron uses.
- **Slash-command stays responsive.** Spawn returns immediately; user gets ack instantly and a follow-up notice when done.

## Surface

### `/watch` parser change

`parse_watch_args` accepts `since=<value>` where the value is either:
- An RFC3339 date or datetime: `2026-01-01`, `2026-01-01T12:00:00Z`
- A relative duration: `7d`, `90d`, `6mo`

Resolved at parse time to an ISO string and passed through as a normal param.

### Template-side `since` handling

In `FeedTemplate::run`, when cursor is null AND `params.since` is set, use it as the `oldest_ts`. Otherwise the cursor wins. Templates that don't opt in just ignore the param.

```rust
let oldest_ts = match (cursor.get("latest_ts").and_then(|v| v.as_str()),
                       params.get_str("since")) {
    (Some(prior), _)              => Some(prior.to_string()),
    (None, Some(since_iso))       => Some(parse_since_to_slack_ts(since_iso)?),
    (None, None)                  => None,
};
```

### Templates that opt in (this task)

- `slack/channel-archive` and `slack/dm-archive` — `since` → `oldest_ts`.
- `confluence/space-archive` — `since` → CQL `lastmodified > since`.
- `jira/project-tracker` and `jira/assignee-tracker` — `since` → JQL `updated >= since`.

Templates that don't opt in:

- `slack/my-mentions` — `search.messages` is day-grained via `after:`. Skip.
- `gmail/*` — already has `days_back` covering this need.
- `calendar/upcoming-archive` — only looks forward. N/A.
- `drive/*` — `recent` already takes `days_back`; `folder-sync` is structural.

### `register_feed_dynamic` change

Extend the existing function in `arawn-feeds/src/runtime.rs`:

```rust
pub async fn register_feed_dynamic(
    &self,
    template: &str,
    feed_id: &str,
    params: TemplateParams,
    cadence_override: Option<String>,
) -> Result<FeedRecord, FeedError> {
    let has_since = params.0.get("since").is_some();
    // ...validate, insert row, write meta.json...

    if has_since && template_supports_backfill(template) {
        // 1. Row goes in with enabled=0; cron won't fire it.
        // 2. Spawn the backfill loop.
        // 3. On loop completion: flip enabled=1 + register cron.
        self.spawn_backfill(record.clone());
    } else {
        // No `since` — register cron now, first cron tick does the
        // 200-msg seed.
        register_one(&self.runner, &self.runtime_ctx, &record).await?;
    }
    Ok(record)
}
```

`spawn_backfill` lives in arawn-feeds; clones the runtime context (already cheap), spawns a tokio task, runs the loop. Surfaces progress + completion via the existing ServerNotice channel (passed via `RealClients` or threaded through; same channel `feed_register` already uses).

### Crash recovery

If the server restarts mid-backfill:

- DB row has `enabled=0`, `meta.last_status = "backfilling"`, cursor at wherever the last successful page got.
- On boot in `arawn_feeds::start`: any row with `enabled=0 AND last_status=="backfilling"` triggers a backfill-resume task. Same spawn path.
- If the user wants to give up, `/feeds rm <id>` works (it deletes the row regardless of state).

## Acceptance Criteria

- [ ] `parse_watch_args` recognizes `since=` (ISO date and `Nd`/`Nw`/`Nmo` relative forms). Rejects garbage at parse time.
- [ ] `FeedTemplate::run` for slack/{channel-archive, dm-archive} uses `params.since` as `oldest_ts` when cursor is null. Subsequent runs use the cursor.
- [ ] `FeedTemplate::run` for confluence/space-archive folds `since` into CQL.
- [ ] `FeedTemplate::run` for jira/{project-tracker, assignee-tracker} folds `since` into JQL.
- [ ] `register_feed_dynamic`:
  - Without `since`: existing behavior (insert enabled=1, register cron immediately).
  - With `since` and backfill-capable template: insert enabled=0 + meta.json `last_status="backfilling"`, spawn loop, return.
- [ ] Backfill loop:
  - Repeatedly calls `dispatch::run_feed`.
  - Stops on `items_written == 0`.
  - On completion: row flipped to `enabled=1`, cron schedule registered, ServerNotice broadcast (`[feeds] <id> backfill complete — N items in T`).
  - On error: leaves row `enabled=0` and `last_status="backfill-failed: <error>"`. User can `/feeds rm` or retry by re-watching.
- [ ] Boot resumption: `arawn_feeds::start` detects `enabled=0 AND last_status=="backfilling"` rows and re-spawns the backfill loop.
- [ ] **Tests**:
  - Unit: `parse_watch_args` accepts both date and duration forms.
  - Integration (`tests/dynamic_register.rs`): mock slack returning 3 pages of 200 messages then an empty page → backfill writes 600 messages total, cursor lands at the newest, row ends `enabled=1`, cron schedule exists.
  - Integration: backfill error mid-loop leaves row `enabled=0` + `last_status="backfill-failed: ..."`, no cron schedule. Re-run resumes from the persisted cursor.
  - Integration: registering without `since` keeps the existing fast path (insert enabled=1, cron immediately).
- [ ] `angreal check workspace` and `angreal check clippy` clean.

## Out of scope (deferred)

- **Slack `Retry-After` parsing + backoff.** The adapter currently maps 429 → `FeedError::RateLimited`; backfill loop just lets that propagate up and parks the row in `backfill-failed`. Add proper backoff in a follow-up if the manual-retry UX gets old.
- **Per-page progress streaming.** `[feeds] backfill complete` at the end is enough for v1; per-page notices add complexity without much value.
- **Other templates' `since` support.** Add when asked.

## Loop resilience requirements

The spawn loop has to survive a list of failure modes without losing data, getting stuck, or hammering the provider. Each is testable.

### 1. Mid-page crash / SIGKILL

- The cursor only advances on successful page write (existing `dispatch::run_feed` invariant — cursor write is atomic via `MetaStore::write` rename-temp).
- After a kill, on-disk JSONL has the last fully-written page; cursor in `meta.json` matches.
- **Boot resumption:** in `arawn_feeds::start`, find rows where `enabled=0 AND last_status=="backfilling"` and re-spawn the loop. The loop reads the cursor and continues forward — no re-fetched pages, no duplicate writes.

### 2. Network blip on a single page

- The loop wraps each `run_feed` call with **retry-on-transient-error**: catch `FeedError::Provider` whose message contains common transient markers (`timeout`, `connection`, `reset`, `refused`), sleep with exponential backoff (1s, 2s, 4s — capped at 3 retries), retry the same fetch.
- After 3 failed retries on the same page, mark `last_status="backfill-failed: <error>"` and exit. User can `/feeds rm` and re-watch, or land a `/feeds resume` (T-0219 slice 2 already wired) that re-spawns from the persisted cursor.

### 3. Slack rate-limit mid-loop

- Catch `FeedError::RateLimited { retry_after }`. If `retry_after` is set (a follow-up will parse it from Slack's response), sleep that long; otherwise sleep a default 60s, then retry.
- Cap rate-limit retries at 5 per page; after 5 the loop exits with `backfill-failed: rate-limited`.
- Tracked separately from transient-error retries so a flaky network doesn't burn rate-limit budget.

### 4. Template returns Schema/Provider error on one page

- A single bad page should not drop a 50-page backfill. Log a warn, **advance the cursor manually** (set `oldest_ts = current_oldest + 1µs`), continue.
- After 3 consecutive skipped pages, exit as `backfill-failed: too many bad pages` to prevent walking forever through corrupt data.

### 5. Cursor doesn't advance (pathological-loop guard)

- Track `prev_cursor` after each iteration. If `items_written > 0` but `cursor == prev_cursor`, the loop is stuck (template bug, provider returning the same page repeatedly). Exit with `backfill-failed: cursor stalled at <ts>` rather than spinning.
- Hard cap at 10,000 pages (≈2M Slack messages). Reaching it exits cleanly with `backfill-failed: page-cap exceeded`.

### 6. Concurrent backfill spawns for the same feed

- `register_feed_dynamic` for a feed_id whose row is already `backfilling` should reject before spawning a duplicate. The DB UNIQUE constraint on `feeds.id` already blocks the row insert; surface a specific error message ("backfill in progress for `<id>`; wait or `/feeds rm <id>` to abort") rather than the generic UNIQUE error.
- `/feeds resume <id>` for a `backfilling` row similarly returns "already running" without re-spawning.

### 7. Server shuts down cleanly mid-backfill

- The spawned task watches a `tokio::sync::watch` shutdown signal (already used by the workflow runner's services).
- On signal: write a final `meta.json` keeping `last_status="backfilling"` (the load-bearing state for boot resumption), exit. Boot picks up.

### 8. `/feeds` listing during backfill

- Adds a derived state to the listing: `enabled=0 AND last_status=="backfilling"` → `backfilling`; `enabled=0` + anything else → `paused`. Switch the existing boolean state in `format_feed_list` to a small enum.
- Surface ETA hint where possible: `backfilling · N pages, ~M items so far`.

### Test matrix for resilience

- `mid_page_kill_resumes_from_cursor` — interrupt the spawn task between two `run_feed` calls; the next boot resumes and writes only the missing pages.
- `transient_error_retries_then_fails_after_3` — mock returns `Provider("connection refused")` 4 times in a row; loop retries 3 then exits `backfill-failed`. Cursor unchanged.
- `rate_limit_sleeps_and_succeeds_on_retry` — mock returns `RateLimited` once with `retry_after=Some(50ms)`, then succeeds; loop sleeps 50ms and finishes.
- `bad_page_skips_then_aborts_after_3_in_a_row` — mock returns 3 consecutive `Schema` errors; loop tries to advance past each, then exits `backfill-failed: too many bad pages`. Cursor advances by 3 µs.
- `pathological_no_progress_exits_loop` — mock returns the same page forever (cursor doesn't advance but `items_written > 0`); loop exits with `cursor stalled`.
- `concurrent_register_with_existing_backfill_rejects` — second `/watch` for the same feed_id while first is still backfilling returns a clear "already in progress" error.
- `clean_shutdown_keeps_backfilling_status` — graceful shutdown signal mid-loop leaves `last_status="backfilling"` so boot resumption fires.

## Other risk / open questions

- **Slack rate limits during a long backfill.** A 10k-msg channel = ~50 pages × `conversations.history` (Tier 3, 50 req/min). Resilience case 3 covers the bad case.
- **Long-running spawned tasks with no observability.** Per-page progress notices are out of scope for v1, but the `meta.json` `run_count` increments on every successful page so `/feeds` shows progress already.

## Status Updates

*To be added during implementation*

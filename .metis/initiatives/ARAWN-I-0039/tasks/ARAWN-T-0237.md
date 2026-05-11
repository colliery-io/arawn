---
id: feed-resilience-matrix-retry-after
level: task
title: "Feed resilience matrix: Retry-After, exponential backoff, schema-skip"
short_code: "ARAWN-T-0237"
created_at: 2026-05-11T13:32:00.770563+00:00
updated_at: 2026-05-11T13:32:00.770563+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Feed resilience matrix: Retry-After, exponential backoff, schema-skip

## Parent Initiative

[[ARAWN-I-0039]]

## Severity

P2 — T-0227 closed the cold-start gap with `since=` + spawn-loop
convergence, and T-0234 made Gmail/Drive backfills walk up to 5000
ids per call. Those bigger sweeps make transient-error handling
load-bearing: a single 429 mid-sweep currently aborts the whole
backfill with `last_status=backfill-failed`, and the user has to
re-run from scratch. The cursor-stalled guard and page cap are in
place; the remaining items in T-0227's resilience matrix (items 2–7)
are not. This is the catch-up task for those.

## Scope — what's still missing from T-0227's matrix

### 1. `Retry-After` parsing on rate-limit responses

`FeedError::RateLimited(retry_after_secs)` already exists, but the
header is parsed inconsistently across providers:

- **Slack** (slack-morphism): exposes `retry_after` in the error
  variant. Wired.
- **Gmail** (google-gmail1): 429s come back with the header on the
  inner response. Not parsed today — we surface `RateLimited(0)` or
  fall through to `Provider`.
- **Drive** (google-drive3): same shape. Not parsed.
- **Atlassian**: 429 not currently distinguished from 5xx.

Land: a small helper that takes the provider error / response
headers and returns `Option<u64>` seconds; wire it into each
adapter's rate-limit path.

### 2. Exponential backoff inside the spawn loop

Once `RateLimited(secs)` is available end-to-end, the backfill loop
should `tokio::time::sleep(retry_after.max(BASE_BACKOFF))` and
continue rather than bailing. Cap the total wait per backfill at e.g.
5 min — beyond that, surface a clear `backfill-rate-limited` status
and let the next cron tick resume from the persisted cursor.

For non-429 transient errors (5xx, connection reset, dns blip),
apply `BASE_BACKOFF * 2^attempts` with small jitter and a 3-attempt
cap before bailing.

### 3. Schema-skip resilience

Today `FeedError::Schema(...)` from one bad item poisons the whole
run (templates `?` it). On a 5000-id Gmail sweep, one malformed
`internalDate` or missing field shouldn't lose the other 4999
messages.

Gmail's helper already does the right thing (`Schema(e) | Provider(e)
=> warn + continue`). Audit the rest:

- `drive/recent` — per-file `modified_to_yyyy_mm_dd` already returns
  `continue` on parse error. ✓
- `drive/folder-sync` — verify.
- `jira/project-tracker`, `jira/assignee-tracker` — verify per-issue.
- `confluence/space-archive` — verify per-page.
- `calendar/upcoming-archive` — verify per-event.

Land any missing per-item `warn + continue` paths; add a unit test
per template that injects a bad item into the middle of a good batch
and asserts the rest write.

## Acceptance Criteria

- [ ] Provider-agnostic helper that turns 429 responses into
  `FeedError::RateLimited(secs)` with the right `Retry-After` seconds
  (integer or HTTP-date form). Unit tests for both forms.
- [ ] Gmail + Drive adapters parse `Retry-After` on 429.
- [ ] Atlassian adapter distinguishes 429 from 5xx and surfaces
  `RateLimited(secs)`.
- [ ] `runtime.rs::run_backfill_loop` sleeps on `RateLimited` rather
  than bailing. Wall-clock cap of 5 min; beyond that, surface
  `backfill-rate-limited` and let the next cron tick resume from
  cursor.
- [ ] Spawn loop retries transient 5xx / connection errors with
  exponential backoff (BASE_BACKOFF=2s, max 3 attempts).
- [ ] Per-template schema-skip audit. Each of the six provider
  templates has a unit test asserting that a malformed item in the
  middle of a batch doesn't stop subsequent items from being written.
- [ ] `angreal check workspace` + `angreal check clippy` clean.
- [ ] No flaky tests — backoff sleeps use `tokio::time::pause`
  / virtual-clock in tests.

## Out of scope

- **Per-feed rate-limit budgeting.** If the user runs 20 Gmail feeds
  on a single Google account they'll share the 250-quota/sec budget.
  A central token bucket would help but is a separate design.
- **Resumable backfills across server restart.** Today a backfill
  that hits the 5-min wall-clock cap surfaces `backfill-rate-limited`
  and the next cron tick picks up from cursor. That's enough —
  full cross-restart resume state is over-engineering.

## Status Updates

*To be added during implementation*

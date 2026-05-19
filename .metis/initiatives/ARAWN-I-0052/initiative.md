---
id: ceremony-reliability-pinned-date
level: initiative
title: "Ceremony reliability — pinned date windows, back-fill, configurable retro cadence"
short_code: "ARAWN-I-0052"
created_at: 2026-05-19T18:53:27.575521+00:00
updated_at: 2026-05-19T18:55:25.937293+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#feature"
  - "#ceremonies"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: M
initiative_id: ceremony-reliability-pinned-date
---

# Ceremony reliability — pinned date windows, back-fill, configurable retro cadence

## Context

Today the ceremonies subsystem has three latent issues:

1. **Clock-locked dispatch.** `EngineDispatcher::dispatch(kind)` calls
   `Utc::now()` and derives `period_key` from it. There is no way to
   compose a tablet for a historical date.
2. **Drifting gather windows.** Daily and weekly gather paths use
   `Utc::now() - Duration::X` to define their lookback window. The
   window therefore shifts with cron-tick jitter (07:00:01 vs
   07:05:00 produce different cutoffs) and cannot be retargeted to
   a historical period.
3. **No missed-cron recovery.** When the cron tick fires while
   arawn is down (laptop closed, server stopped), the tablet for
   that day is simply absent — there is no retroactive compose
   path. The "watch, check, summarize, nudge" loop the vision
   commits to is supposed to survive laptop sleep.

Retro also has a fourth, smaller issue: its cadence is hard-coded
to weekly. A user who wants biweekly or monthly retros has no knob.

## Goals

- Gather paths read from **pinned date ranges** derived from the
  tablet's `period_key`, not from `Utc::now() - Duration::X`.
- `EngineDispatcher` can compose a tablet for an arbitrary
  historical date.
- On `arawn serve` boot, any periods missed within the last 14
  days are back-filled. Daily and weekly only — retro skips
  back-fill.
- Retro's cadence (weekly / biweekly / monthly) is
  agent-configurable.
- Back-filled tablets carry a `recovered = true` flag so the UI
  / API can mark them, and detectors can distinguish recovered
  context from live.
- The agent gets a "current local time" header injected into
  every turn — fixes a recurring "what is today?" guessing
  problem that surfaced while scoping this work.

## Non-goals

- **No back-fill beyond 14 days.** Beyond two weeks the tablets
  are ceremonial clutter, not features. Skipped silently with a
  single boot log line.
- **No retro back-fill.** Retro depends on aggregated weekly
  history; recovering a missed retro after the fact doesn't add
  value the user can act on.
- **No retroactive feed recovery.** Feeds run inside the arawn
  server process; if arawn was off, the feed history isn't in
  the DB to gather from. Back-fill operates on whatever signals
  exist, period.
- **No UI work beyond surfacing the `recovered` flag** — actual
  presentation choices land in a follow-up initiative.

## Why the 14-day cap

The cap is not a performance bound — back-fill is cheap. It is
**UX honesty + detector determinism**:

- **Weekend gap (≤ 3 days):** ~always useful. You actually want
  Friday's daily on Monday morning.
- **Week-long absence (4–10 days):** marginally useful. You'll
  skim it, not act on it.
- **Two weeks+ (>14 days):** ceremonial noise. You're not going
  to "catch up." You just open arawn and want today's stuff.

Beyond 14 days we skip with a single boot log line. Also bounds
the retro detector's recovery surface so
`priority_completion_ratio` sees a deterministic window.

## Detailed Design

### Pinned date windows

`Ceremony` trait gains:

```rust
fn period_window(&self, period_key: &str) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError>;
```

- **Daily:** `period_key` is `YYYY-MM-DD` local. Window is
  `[date 00:00 local, date+1 00:00 local)` converted to UTC.
- **Weekly:** `period_key` is `YYYY-Www`. Window is `[Monday
  00:00 local, next Monday 00:00 local)` converted to UTC.
- **Retro:** same as weekly today; biweekly / monthly cadences
  computed from an anchor date once T-δ lands.

`EngineCtx` exposes `period_window()` so gather can read it.
Gather queries replace `Utc::now() - Duration::X` with
`ctx.period_window().start`. Local timezone is the source of
truth for boundaries.

### Dispatching for a historical date

`EngineDispatcher::dispatch_for(kind, target: NaiveDate)`
replaces `Utc::now()`-based dispatch with the target. Existing
`dispatch(kind)` becomes `dispatch_for(kind, today_local())`.

- `period_key` derived from `target` via plugin.
- Tablet `generated_at` is wall-clock (truthful audit trail).
- `recovered = (target != today_local())` written into a new
  column.
- Idempotency check unchanged — re-running a date that already
  has a non-`open` tablet is a no-op.

### Back-fill loop

`arawn_ceremonies::backfill::run(conn, registry, lookback_days)`:

1. For each plugin (daily, weekly — retro skipped):
2. Walk dates from `today - lookback_days` to `today - 1 day`.
3. For each date with no existing tablet, call
   `dispatch_for(kind, date)`.
4. Log "N back-filled, M skipped (already present), K skipped
   beyond lookback".

Wired into `arawn serve` startup before the cron loop attaches.
Configurable via `[ceremonies] backfill_lookback_days` (default
14; 0 disables).

### Retro cadence

`[ceremonies.retro] cadence = "weekly" | "biweekly" | "monthly"`
(default `"weekly"`). Anchor date is the first run after
enablement. The retro plugin reads cadence from config at
registration and adjusts:

- `default_schedule()` → cron expression for the cadence.
- `period_key()` → ISO week for weekly; `YYYY-Bxx` for biweekly
  (anchor-relative); `YYYY-MM` for monthly.
- `period_window()` → corresponding window.

Agent tool `retro_set_cadence` writes the config knob.
Independent of back-fill but built on the pinned-windows
foundation.

### Current-time header

Inject a single line at the top of each turn's system context:

```
Current time: 2026-05-19 14:32 PDT (Mon)
```

Local timezone, ISO date, day-of-week. Cheap; sits in the
existing system prompt assembly. Eliminates the "agent guesses
what today is" failure mode and makes date-sensitive tool calls
reliable across turns.

## Implementation Plan

Six tasks, roughly ordered:

- **T-α (T-0364): pinned date windows.** Refactor gather to use
  `period_window()`. No behavior change.
- **T-β (T-0365): dispatch_for + recovered column.** Adds the
  historical-dispatch surface, migration, audit flag.
- **T-γ (T-0366): boot-time back-fill.** The loop itself. 14-day
  cap. Daily + weekly only.
- **T-δ (T-0367): configurable retro cadence.** Weekly /
  biweekly / monthly, agent-settable.
- **T-ε (T-0368): current-time header.** Single line, every
  turn.
- **T-ζ (T-0369): docs rewrite.** `ceremonies.md` +
  `ceremonies-tools.md`. Closes the back-fill doc-vs-code gap
  surfaced in I-0051.

T-α through T-γ supersede T-0353, which this initiative absorbs
and closes. T-δ and T-ε are scoped here because they share the
"pinned windows" foundation.

(Task short codes are placeholders until the children are
created — Metis auto-assigns.)

## Alternatives Considered

- **Stub back-fill (placeholder tablets only).** Rejected —
  only preserves the calendar; doesn't recover content. The
  pinned-windows refactor unlocks real back-fill at low
  marginal cost.
- **Retro back-fill.** Rejected — depends on aggregated weekly
  history and produces tablets the user can't act on.
- **Boot recovery for *today only*.** Rejected — the 14-day cap
  is a more honest answer.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Daily and weekly gather paths read from
  `EngineCtx::period_window()`; no `Utc::now() - Duration::X`
  remains in those plugins.
- [ ] `EngineDispatcher::dispatch_for(kind, NaiveDate)` exists
  and back-dated tablets carry `recovered = true`.
- [ ] Server startup runs the back-fill loop. The 14-day cap is
  enforced and configurable.
- [ ] Retro cadence is configurable to weekly / biweekly /
  monthly via `[ceremonies.retro] cadence` + an agent tool.
- [ ] Every agent turn's system prompt includes a current-time
  header.
- [ ] `docs/src/explanation/ceremonies.md` and
  `docs/src/reference/ceremonies-tools.md` describe all the
  above; doc-vs-code gap from I-0051 closed.
- [ ] `angreal test unit` green. `angreal check workspace` green.
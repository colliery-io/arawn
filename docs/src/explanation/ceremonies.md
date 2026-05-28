# Ceremonies

*Explanation. The morning brief / weekly priorities / retro philosophy and the "watch, check, summarize, nudge" thesis.*

A **ceremony** is a scheduled, agent-driven reflection. arawn ships three — daily, weekly, retro — and they're the most direct expression of the vision's *"watch, check, summarize, nudge"* framing.

For the agent tools and cron defaults, see [ceremonies tools reference](../reference/ceremonies-tools.md). This page is about *why* ceremonies.

## The thesis: stay organized without holding it in your head

The vision says: *"arawn watches, checks, summarizes, and nudges — so you don't have to keep everything in your head."* In practice, "watching" without "summarizing + nudging" is just a stack of unread notifications. The mirrored data sits there; you don't act on it.

Ceremonies are the bridge. They're scheduled moments when the agent looks at what's accumulated, picks out what matters, and produces something you can act on:

- **Daily** — every weekday morning. "Here's what changed since yesterday. Calendar conflicts? Stale tickets? Items from priorities you haven't moved?"
- **Weekly** — Monday morning. "What are the 3-5 priorities for this week? Carry over the unfinished ones. Surface new ones from the past week's activity."
- **Retro** — Friday afternoon. "Which of last week's priorities actually progressed? Which stalled? Anything roll over multiple weeks? What lenses went neglected?"

Without the ceremonies, the mirrored data is just storage. With them, it's an ambient awareness loop.

## Why three cadences

Each cadence answers a different question:

- **Daily** is "what's urgent today?" Cadence: weekdays only. Short window, low cost. The agent's job is to surface the items that need *now* attention.
- **Weekly** is "what matters this week?" Cadence: Monday morning, before the week starts. Higher-cost compute (priorities are a synthesis pass), but you make the decision once. The agent proposes; you confirm; the priorities become the spine of the week.
- **Retro** is "what did I learn?" Cadence: Friday afternoon, after the work-week. The agent looks back — what did last week's priorities actually become, what got rolled over, what got neglected. The diary you save here feeds next week's gather phase.

The split avoids the "everything-every-day" trap. If daily had to surface weekly priorities and retro learnings, it'd become noise. Separating them by cadence keeps each one focused.

## Why a structured tablet, not a chat message

Each ceremony produces a **tablet** — a structured artifact with items, todos, (for weekly) priorities, (for retro) detectors output. The tablet is a UI object: you open `/today`, `/week`, `/retro` and see a modal you can interact with — confirm an item, mark a priority done, edit a diary entry, dismiss a stale rollover.

Why not just a chat turn?

- **Persistence.** Tomorrow's daily wants to know what you confirmed today. A chat turn evaporates; a tablet is a row in `arawn.db`.
- **Cross-ceremony flow.** Weekly priorities feed next weekly's "carry-over" logic. Retro detectors compare to last week's priorities. Chat messages can't compose like this.
- **Structured surface for the user.** Daily-tablet items have status (done/dismissed/pending). Weekly priorities have a confirmation step. Retro items have a diary slot. Chat is too unstructured for these affordances.

## Why the agent proposes, you confirm

The weekly priority flow is the clearest case: the agent looks at last week's activity (your Jira tickets, Slack mentions, calendar density, what you accomplished, what you said matters) and **proposes** 3-5 priorities. You confirm, reject, or edit each one.

This is the same proposal-vs-apply pattern from the steward — and for the same reasons:

- **Stakes are personal.** What you choose to prioritize this week is a self-directed choice. The agent informs; you decide.
- **Cost of false-positive matters.** If the agent commits you to a priority that's wrong, you carry that into the week. Better to ask.
- **The confirmation step is the value.** Reviewing the proposed priorities is itself a useful reflection. The friction is the feature.

The daily and retro tablets are softer — items there are surfaced for awareness; you can mark them done or dismiss them, but most flow through passively. Weekly is where the explicit confirm step lives because that's the decision-shaped moment.

## Why retro has detectors

The retro's gather phase runs a small set of **detectors** — Rust code that looks back over the week's data and surfaces specific patterns:

| Detector key | What it surfaces |
|---|---|
| `priority_completion_ratio` | Which of last week's priorities actually progressed; which stalled. |
| `rollover_heat` | Items that have rolled over multiple weeks (potentially stalled or wrongly-scoped). |
| `lens_neglect` | Lenses that haven't been touched all week. |

Why detectors instead of asking the LLM "look at this week and tell me what you notice"?

- **Determinism.** A detector is reproducible code; the same inputs produce the same output. An LLM "what did you notice?" produces different outputs every run. For retro to be a stable practice, the surfaced items need to be stable.
- **Targeting.** Each detector has one job — answer one specific question. The retro ends up with three specific, named pattern outputs, not a generic "here's some stuff that happened."
- **Composability.** A new detector is a new Rust function. The user-facing tablet automatically picks it up.

The LLM still runs during retro — it composes the tablet's prose, judges which detector outputs are worth surfacing, and frames the diary prompt. But the detectors are the *structured* input.

## Pinned date windows

Daily and weekly gather queries don't read from "the last 24 hours" or "the last 7 days" relative to the wall clock — they read from a **pinned window** derived from the tablet's `period_key`:

- **Daily** for `2026-05-19` reads from `[2026-05-19 00:00 local, 2026-05-20 00:00 local)`.
- **Weekly** for `2026-W21` reads from `[Mon 2026-05-18 00:00 local, Mon 2026-05-25 00:00 local)`.

That has two consequences:

1. **Cron-jitter immunity.** Whether the 07:00 cron actually fires at 07:00:01 or 07:05:00, the window is identical.
2. **Back-filling is meaningful.** Composing a tablet for a historical date pulls signals from *that day's* window, not "the last day from now." A Monday-morning back-fill of Friday's daily gathers signals that landed Friday, not the noise that arrived over the weekend.

Source: `crates/arawn-ceremonies/src/local_window.rs` + each plugin's `period_window` impl.

## Boot-time back-fill

If arawn was offline when a daily/weekly cron tick should have fired (laptop closed, server stopped), the missed tablets get composed on the next `arawn serve` boot. The loop walks `[today - 14d, today - 1d]` and dispatches for any date whose tablet is missing.

- Configurable cap via `[backfill] ceremony_lookback_days` (default 14, `0` disables).
- **Daily + weekly only.** Retro is excluded — its detectors depend on aggregated weekly history and recovering a missed retro after the fact doesn't give the user anything actionable.
- Each back-filled tablet carries a `recovered = true` flag so the UI / API can mark them and downstream consumers can distinguish recovered context from live.
- Idempotent: dates with an existing tablet (any status) are skipped, so running back-fill twice is a no-op.

### Why 14 days?

The cap isn't a performance bound — back-fill is cheap. It's UX honesty:

- **Weekend gap (≤ 3 days):** ~always useful. You actually want Friday's daily on Monday morning.
- **Week-long absence (4–10 days):** marginally useful. You'll skim it, not act on it.
- **Two weeks+:** ceremonial noise. You're not going to "catch up." You just open arawn and want today's stuff.

Beyond 14 days we skip silently with a single boot log line.

## What the nightly maintenance loop still does

`crates/arawn-ceremonies/src/nightly.rs` runs an hourly tokio sweep — `sweep_unreviewed_retros` — that transitions stale `open` retro tablets to `unreviewed` so detectors can spot "diary skipped 3 weeks running" patterns. Distinct from back-fill, which is a one-shot boot pass.

## Retro cadence

By default retro fires Friday at 16:00 local. Users who want a longer rhythm can switch to **biweekly** (every other Friday) or **monthly** (first Friday of each month) via:

- `[ceremonies.retro] cadence = "biweekly"` in `arawn.toml`, or
- the `retro_set_cadence` agent tool, which writes the same knob to the `ceremony_config` table.

The cron schedule itself stays `"0 16 * * FRI"` in all three cases — the off-week / off-month Fridays still fire, but the existing tablet's `period_key` matches and the dispatcher returns `Skipped`. Net behaviour:

- **Weekly:** every Friday composes.
- **Biweekly:** every other Friday composes; off-week Fridays return Skipped. Anchored on the first Friday after enablement; anchor is persisted in `ceremony_config` so the cycle is stable across restarts.
- **Monthly:** first Friday of each month composes; later Fridays return Skipped. The `period_key` becomes `YYYY-MM`, so Friday-2 of May matches the existing `2026-05` tablet and skips cleanly.

Biweekly's `period_key` is `B{N}` — a flat counter from the anchor rather than a year-prefixed key, because biweekly cycles cleanly straddle year boundaries. Negative biweeks (pre-anchor dates) are well-defined via floor division for back-fill scenarios.

The agent-facing tool only persists the knob; the live plugin still has the old cadence until the next `arawn serve` restart. Hot-applying isn't supported in v1 — it'd require interior mutability on the plugin Arc that hasn't earned its complexity yet.

## Current-time header

Every agent turn's system prompt starts with a line like:

```
Current time: 2026-05-19 14:32 PDT (Tue)
```

In the system local timezone. Eliminates the recurring failure mode where date-sensitive tool calls (ceremony lookups, todo windows, scheduling) drifted across turns because the agent had no reliable now-reference. The header is cheap (a single chrono format call per turn) and lands at priority 0 in the assembled prompt so it's the first thing the model sees.

## Why ceremonies aren't optional in the engine sense

You *can* disable a ceremony in `arawn.toml`:

```toml
[ceremonies.weekly]
enabled = false
```

This stops the plugin from registering — no cron, no RPC, no tools. But the ceremony surfaces aren't pluggable in a deep sense — the engine assumes daily and retro exist when wiring the lens router, the todo system's source enum has `ceremony/daily` and `ceremony/weekly/priority` entries, and the TUI has `/today` / `/week` / `/retro` modals that try to load. Disabling a ceremony works but is a low-test path.

The opinionated stance: ceremonies are part of arawn's identity, not a feature you bolt on. The user picks personas (via `identity_profile`) and providers (via `/connect`); they don't customize the reflection cadence to anything weirder than tweaking cron expressions and skipping ceremonies they don't want.

## What ceremonies aren't

- **Notifications.** A ceremony doesn't ping you. It composes a tablet; you read it when you open the relevant modal. The push-notification path is part of the broader I-0035 work (Phase 4) but isn't a ceremony itself.
- **Action takers.** A daily-ceremony item like "send the report" is a *todo*, not a thing the agent does automatically. You see it, you act (or you defer).
- **AI standups.** A standup is a shared artifact. Ceremonies are personal. If you want shared standup behavior, that's a different system on top.

## Related

- [Ceremonies tools reference](../reference/ceremonies-tools.md) — the tool family.
- [Todos tools reference](../reference/todos-tools.md) — how ceremony items become todos.
- [Workflows explanation](./workflows.md) — workflows are the daily ceremony's gather-phase engine.
- [Lenses explanation](./lenses.md) — each ceremony runs per-lens over that lens's signal stream.
- [What is arawn?](./what-is-arawn.md) — the vision ceremonies operationalize.

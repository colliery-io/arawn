# Ceremonies

*Explanation. The morning brief / weekly priorities / retro philosophy and the "watch, check, summarize, nudge" thesis.*

A **ceremony** is a scheduled, agent-driven reflection. arawn ships three — daily, weekly, retro — and they're the most direct expression of the vision's *"watch, check, summarize, nudge"* framing.

For the agent tools and cron defaults, see [ceremonies tools reference](../reference/ceremonies-tools.md). This page is about *why* ceremonies.

## The thesis: stay organized without holding it in your head

The vision says: *"arawn watches, checks, summarizes, and nudges — so you don't have to keep everything in your head."* In practice, "watching" without "summarizing + nudging" is just a stack of unread notifications. The mirrored data sits there; you don't act on it.

Ceremonies are the bridge. They're scheduled moments when the agent looks at what's accumulated, picks out what matters, and produces something you can act on:

- **Daily** — every weekday morning. "Here's what changed since yesterday. Calendar conflicts? Stale tickets? Items from priorities you haven't moved?"
- **Weekly** — Monday morning. "What are the 3-5 priorities for this week? Carry over the unfinished ones. Surface new ones from the past week's activity."
- **Retro** — Friday afternoon. "Which of last week's priorities actually progressed? Which stalled? Anything roll over multiple weeks? What workstreams went neglected?"

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

| Detector | What it surfaces |
|---|---|
| `priority-completion` | Which of last week's priorities actually progressed; which stalled. |
| `rollover-heat` | Items that have rolled over multiple weeks (potentially stalled or wrongly-scoped). |
| `workstream-neglect` | Workstreams that haven't been touched all week. |

Why detectors instead of asking the LLM "look at this week and tell me what you notice"?

- **Determinism.** A detector is reproducible code; the same inputs produce the same output. An LLM "what did you notice?" produces different outputs every run. For retro to be a stable practice, the surfaced items need to be stable.
- **Targeting.** Each detector has one job — answer one specific question. The retro ends up with three specific, named pattern outputs, not a generic "here's some stuff that happened."
- **Composability.** A new detector is a new Rust function. The user-facing tablet automatically picks it up.

The LLM still runs during retro — it composes the tablet's prose, judges which detector outputs are worth surfacing, and frames the diary prompt. But the detectors are the *structured* input.

## Why a nightly recovery loop

`crates/arawn-ceremonies/src/nightly.rs` runs at 02:00 local and back-fills missed ceremony tablets. If your laptop was closed on Tuesday and Wednesday morning, the daily cron didn't fire — the nightly loop runs the daily ceremony for those dates retroactively the next time arawn is up.

Why bother?

- **Continuity.** Skipping days breaks the "what changed since yesterday?" loop. The retroactive daily lets you catch up cleanly.
- **Detector inputs.** retro's `priority-completion` detector needs a complete history. Gaps would skew the output.
- **No surprise.** A user shouldn't have to know "oh, I missed Tuesday's daily, so today's daily won't have Tuesday's context." The nightly loop hides that.

The trade-off: a long absence (a week-long vacation) produces a flurry of back-dated tablets on first run. They're not actionable (you weren't there), but they preserve the audit trail.

## Why ceremonies aren't optional in the engine sense

You *can* disable a ceremony in `arawn.toml`:

```toml
[ceremonies.weekly]
enabled = false
```

This stops the plugin from registering — no cron, no RPC, no tools. But the ceremony surfaces aren't pluggable in a deep sense — the engine assumes daily and retro exist when wiring the workstream router, the todo system's source enum has `ceremony/daily` and `ceremony/weekly/priority` entries, and the TUI has `/today` / `/week` / `/retro` modals that try to load. Disabling a ceremony works but is a low-test path.

The opinionated stance: ceremonies are part of arawn's identity, not a feature you bolt on. The user picks personas (via `identity_profile`) and providers (via `/connect`); they don't customize the reflection cadence to anything weirder than tweaking cron expressions and skipping ceremonies they don't want.

## What ceremonies aren't

- **Notifications.** A ceremony doesn't ping you. It composes a tablet; you read it when you open the relevant modal. The push-notification path is part of the broader I-0035 work (Phase 4) but isn't a ceremony itself.
- **Action takers.** A daily-ceremony item like "send the report" is a *todo*, not a thing the agent does automatically. You see it, you act (or you defer).
- **AI standups.** A standup is a shared artifact. Ceremonies are personal. If you want shared standup behavior, that's a different system on top.

## Related

- [Ceremonies tools reference](../reference/ceremonies-tools.md) — the tool family.
- [Todos tools reference](../reference/todos-tools.md) — how ceremony items become todos.
- [Workflows explanation](./workflows.md) — workflows are the daily ceremony's gather-phase engine.
- [Workstreams explanation](./workstreams.md) — each ceremony scopes to the active workstream.
- [What is arawn?](./what-is-arawn.md) — the vision ceremonies operationalize.

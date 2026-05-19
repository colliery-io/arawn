# Ceremonies tools

*Reference. Daily, weekly, and retro ceremony tool families plus default cron schedules.*

Source: `crates/arawn-ceremonies/`, `crates/arawn-engine/src/tools/{daily,weekly,ceremony}.rs`.

## What a ceremony is

A **ceremony** is a scheduled agent-driven reflection. arawn ships three:

| Ceremony | Default cron | Slash command | Modal in TUI |
|---|---|---|---|
| Daily | `0 7 * * MON-FRI` (07:00 weekdays) | `/today` | Daily tablet |
| Weekly | `0 7 * * MON` (07:00 Monday) | `/week` | Weekly tablet |
| Retro | Friday 16:00 local | `/retro` | Retro tablet |

Each produces a "tablet" — a structured artifact containing items the agent surfaced, todos it proposed, and (for weekly) priorities. You confirm / reject / edit items; the next ceremony tick respects those decisions.

Overrides live under `[ceremonies.<kind>]` in `arawn.toml` — see [config schema](./config-schema.md). Disable, change schedule, change timezone, change LLM with hint shortcuts.

## Daily ceremony tools

Source: `crates/arawn-engine/src/tools/daily.rs`. Plugin: `crates/arawn-ceremonies/src/plugins/daily.rs`.

| Tool | Description |
|---|---|
| `daily_run` | Run the daily ceremony now (gather → compose → persist). |
| `daily_current` | Show today's tablet. Same content the `/today` modal renders. |
| `daily_list_items` | List items on the active tablet (calendar, attention, follow-ups, agent notes). |
| `daily_patch_item` | Edit / mark done / dismiss an individual item. |
| `daily_add_todo` | Add a todo into the tablet (also visible via `/todo`). |

## Weekly ceremony tools

Source: `crates/arawn-engine/src/tools/weekly.rs`. Plugin: `crates/arawn-ceremonies/src/plugins/weekly.rs`.

| Tool | Description |
|---|---|
| `weekly_run` | Run the weekly ceremony now. |
| `weekly_current` | Show this week's tablet. Same content the `/week` modal renders. |
| `weekly_list_items` | List items on the active tablet. |
| `weekly_list_priorities` | List proposed priorities the agent surfaced. |
| `weekly_confirm_priority` | Confirm a priority. |
| `weekly_reject_priority` | Reject a priority. |
| `weekly_add_priority` | Add a priority manually. |

The weekly ceremony is the only one that distinguishes **items** (calendar, mentions, ticket activity) from **priorities** (the 3-5 things you say matter this week). Priorities feed back into next week's gather phase.

## Retro ceremony tools

Source: `crates/arawn-engine/src/tools/ceremony.rs`. Plugin: `crates/arawn-ceremonies/src/plugins/retro.rs`.

| Tool | Description |
|---|---|
| `retro_run` | Run retro now. |
| `retro_current` | Show this week's retro. Same content the `/retro` modal renders. |
| `retro_list_items` | List items the retro composed. |
| `retro_save_diary` | Save your free-form diary entry. |
| `retro_patch_item` | Edit / mark / dismiss an item. |

### Retro detectors

The retro's gather phase runs a set of detectors that look back over the week's data:

| Detector | What it surfaces |
|---|---|
| `priority-completion` | Which of last week's priorities actually progressed; which stalled. |
| `rollover-heat` | Items that have rolled over multiple weeks (potentially stalled or wrongly-scoped). |
| `workstream-neglect` | Workstreams that haven't been touched all week. |

Detector output lands as retro items; you confirm / reject / patch in the modal.

## Schedule overrides

```toml
[ceremonies.daily]
enabled = true
schedule = "0 6 * * MON-FRI"    # 06:00 instead of 07:00
timezone = "America/New_York"
model = "hint:medium"

[ceremonies.weekly]
enabled = false                  # skip weekly entirely

[ceremonies.retro]
schedule = "0 17 * * FRI"
```

When `enabled = false`, the plugin isn't registered — no cron, no RPC routes, no tools. Invalid cron expressions log a warn and fall back to the default. Defaults: enabled, `local` timezone, `hint:medium` model.

## Recovery

Source: `crates/arawn-ceremonies/src/nightly.rs`.

A nightly recovery loop runs at 02:00 local. For any workstream where the cron tick missed (laptop closed, server down, etc.) it gathers + composes a back-dated ceremony tablet. This is why opening arawn after a long gap still shows the days you missed.

## Storage

Tablets and items live in `arawn.db`:

- `ceremonies` — tablet metadata (kind, date, workstream).
- `ceremony_items` — per-item rows (title, body, status, source).
- `ceremony_priorities` — weekly priorities.
- `ceremony_events` — append-only event log for replay/debugging.

`arawn.db` is also where todos live (see [todos tools reference](./todos-tools.md)) — ceremonies and todos share infrastructure.

## Related

- [Todos tools reference](./todos-tools.md).
- [Ceremonies explanation](../explanation/ceremonies.md) — the morning brief / weekly / retro philosophy.
- [Slash commands reference](./slash-commands.md) — `/today`, `/week`, `/retro`, `/todo`.

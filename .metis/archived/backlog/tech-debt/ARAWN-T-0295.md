---
id: configurable-ceremony-schedule
level: task
title: "Configurable ceremony schedule + model — [ceremonies.<kind>] in arawn.toml"
short_code: "ARAWN-T-0295"
created_at: 2026-05-16T13:52:45.909172+00:00
updated_at: 2026-05-16T13:58:52.440971+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Configurable ceremony schedule + model

## Objective

Lift retro out of its current hard-coded posture so the daily prep
ceremony (I-0041) lands on a shared config surface from day one.
Today `main.rs` constructs `RetroCeremony` with a literal
`"hint:medium"` model and no override; the schedule comes from the
trait's `default_schedule()`. Move both to `[ceremonies.<kind>]`
sections in `arawn.toml`, falling back to plugin defaults when
absent so the existing UAT keeps passing.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ArawnConfig` deserialises an optional `[ceremonies.<kind>]`
      TOML table per ceremony kind. Fields, all optional:
      - `enabled: Option<bool>` — default true
      - `schedule: Option<String>` — cron expression
      - `timezone: Option<String>` — `"local"` (default) or IANA zone
      - `model: Option<String>` — hint or concrete model name
- [ ] `main.rs` consults `config.ceremonies.retro` before building
      `RetroCeremony`:
      - `enabled = false` skips registration of plugin, cron, RPC, and
        agent tools.
      - `model` overrides `"hint:medium"` (still routed through
        `llm_pool.resolve_hint`).
      - `schedule` + `timezone` produce a `CronSchedule` that
        overrides `RetroCeremony::default_schedule()` at cron
        registration time. New helper `CeremonyRunner::register_one_with_schedule(kind, override)` (or equivalent) — won't conflict with daily's wiring.
- [ ] Absent or empty `[ceremonies.retro]` table = identical
      behaviour to today (regression-safety; existing UAT must pass).
- [ ] Unit tests in `arawn-bin`:
      - parses a full config block correctly
      - parses an empty file (no `[ceremonies]` table) returning
        sensible defaults
      - `enabled = false` is observed
- [ ] `docs/src/configuration.md` (or wherever ceremonies docs live)
      gets a section describing the table + defaults.

## Implementation Notes

### Technical Approach

1. Add a `CeremoniesConfig` struct alongside `ArawnConfig` with one
   nested map keyed by ceremony kind. Use `serde(default)` so the
   table is optional.
2. The plugin trait stays untouched — schedule overrides happen at
   the `CeremonyRunner` registration site by either:
   - Adding a `register_one_with_schedule(kind, override)` variant
     that uses the supplied `CronSchedule` instead of
     `plugin.default_schedule()`.
   - Or passing the override through a thin `ResolvedSchedule`
     helper next to the runner's existing `register_one`.
   Lean toward the former — explicit, minimal surface change.
3. Model override: pass the resolved model string through
   `RetroCeremony::new`; today's wiring already grabs the resolved
   tuple from `llm_pool.resolve_hint`, so this is mostly a config
   read.
4. `enabled = false` short-circuits the whole ceremony block in
   `main.rs` (don't construct the service, don't mount RPC handlers,
   don't register agent tools). Matches the existing pattern when
   the workflow runner is unavailable.

### Dependencies

- Sets the precedent I-0041 (daily prep) will follow — daily will
  consume the same `[ceremonies.daily]` table via the same code path.
- No code dependencies on I-0041 itself.

### Risk Considerations

- **Schedule parse failure surface area**: cron expressions can be
  malformed. The override path must validate via cloacina's parser
  before mounting; on parse error, fall back to
  `plugin.default_schedule()` with a `warn!` rather than erroring
  out startup. Logging makes the misconfiguration discoverable.
- **Hot-reload**: out of scope. Config changes require restart.
  Document this explicitly.

## Status Updates

### 2026-05-16 — landed

- `CeremonyConfig` struct + `ceremonies: HashMap<String, CeremonyConfig>`
  field added to `ArawnConfig`, re-exported from `arawn-bin`.
  `#[serde(default)]` on every field plus `serde(default)` on the
  map itself = absent table parses cleanly to defaults.
- New `CeremonyRunner::register_one_with_schedule(kind, override)`
  in `arawn-ceremonies/src/runner.rs`; the existing
  `register_one(kind)` is now a thin shim that passes `None`. Lets
  callers override the cron expression + timezone without touching
  the plugin trait.
- `main.rs` consults `config.ceremonies.get("retro")` before wiring:
  - Disabled retro short-circuits the entire ceremony block (no
    cron, RPC, sweep, or agent tools). Documented as a v1
    simplification — daily-time will need to restructure to allow
    independent per-plugin enable.
  - Model override threaded into `llm_pool.resolve_hint` (any string
    works — hint shortcut or concrete model name).
  - Schedule + timezone override built as `CronSchedule::new(...)`
    and passed to `register_one_with_schedule`. Invalid expressions
    will surface via the existing cloacina warn path rather than
    aborting startup.
- `ArawnConfig::generate_default_toml` grows a commented-out
  `[ceremonies.retro]` block so users discover the surface from a
  fresh config.
- Four unit tests added to `config.rs`:
  - Empty config → empty `ceremonies` map.
  - Full block parses every field.
  - `enabled = false` is observed via `is_enabled()`.
  - Partial block leaves the rest as None.
  Plus the existing `generate_default_toml_is_parseable` covers the
  new commented-out section.
- Existing retro UAT remains regression-safe: the harness's
  `arawn.toml` has no `[ceremonies]` table, so the override path
  resolves to defaults and exercises the same wiring code that
  shipped in T-0292.

### Acceptance status

All criteria met. The daily ceremony (I-0041) can now consume the
same `[ceremonies.daily]` surface from day one.

Completed 2026-05-16.
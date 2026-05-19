---
id: usage-slash-command-in-tui-mirror
level: task
title: "/usage slash command — in-TUI mirror of arawn usage"
short_code: "ARAWN-T-0362"
created_at: 2026-05-19T16:00:00+00:00
updated_at: 2026-05-19T15:18:27.489445+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# /usage slash command — in-TUI mirror of arawn usage

## Backlog Item Details

### Type
- [x] Feature

### Priority
- [x] P3 — Low (CLI surface already exists)

## Objective

Mirror the existing `arawn usage` CLI command as a `/usage`
slash command so users can see token / call-count rollups
without leaving the TUI.

Filed from the I-0011 review (2026-05-19) — most of I-0011
became defunct after the I-0035 persona shift moved token
chrome off the status bar; `/usage` keeps that surface
accessible on demand without re-cluttering the chrome.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `/usage` slash command registered.
- [ ] Handler queries the usage rollup (same data source as
      `arawn usage` — call into `arawn_llm::usage` directly
      or via an existing service method).
- [ ] Renders the result as a system message in the chat,
      formatted as a compact table or matching the CLI's
      `render_usage_human` output.
- [ ] Optional arg: `/usage week` to scope to the last 7 days,
      `/usage month` for 30 days. Default = all-time. Mirror
      the CLI's flags if present.
- [ ] `angreal test unit` green.

## Implementation Notes

- The CLI version lives in `crates/arawn/src/main.rs`
  (`render_usage_human`). Extract or reuse the rendering
  helper so the slash command and CLI stay in sync.
- The slash command dispatch lives in `arawn-tui::command`
  and `arawn-tui::event_loop`. Mirror the
  `CeremonyShowToday` pattern: parse → CommandResult enum →
  event-loop dispatch.

## Status Updates

### 2026-05-19 — /usage shipped

- **Shared renderer.** Moved `render_usage_human` from
  `crates/arawn/src/main.rs` into
  `arawn_llm::usage::render_usage_human` so the `arawn usage`
  CLI and the TUI `/usage` slash command share one
  formatter. CLI updated to use the new path.
- **New WS-RPC method `usage.summary`** in `ws_server.rs`.
  Params: `period` (day|week|month|all, default `day`),
  `by_site` (bool, default false), `model` (optional).
  Server reads the process-wide tracker via
  `arawn_llm::usage::global()` and returns a serialised
  `UsageSummary`. Unknown period → `invalid_params`; no
  tracker installed → `tracker_not_installed`.
- **Slash command** `/usage [day|week|month|all]` registered
  with default `day`. Parsing lowercases the arg.
- **Event-loop dispatch** in `arawn-tui::event_loop` via new
  `render_usage(client, period)` helper — calls the RPC,
  deserialises, hands to
  `arawn_llm::usage::render_usage_human`, wraps the body in
  a fenced code block so the rollup keeps its column
  alignment under the markdown renderer.
- **Cargo:** added `arawn-llm = { workspace = true }` to
  `arawn-tui/Cargo.toml` so the TUI shares the renderer +
  `UsageSummary` type.
- **Tests (3 new in `command::tests`):**
  - `execute_usage_default_period_is_day`
  - `execute_usage_with_week_arg`
  - `execute_usage_lowercases_args`
- `cargo test -p arawn-tui --lib` 220/0 (217 prior + 3 new).
  `angreal check workspace` green.
---
id: phase-6-watch-slash-command-feeds
level: task
title: "Phase 6 — /watch slash command + /feeds management UX"
short_code: "ARAWN-T-0219"
created_at: 2026-05-07T00:42:53.527120+00:00
updated_at: 2026-05-09T00:20:07.795585+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Phase 6 — /watch slash command + /feeds management UX

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0039]]

## Objective **[REQUIRED]**

User-facing surface for managing feeds at runtime, without restarting the server or editing config files. Adds:

- `/watch <provider> <template> [params]` — register a new feed. Picker UI when params are omitted; explicit name accepted when provided.
- `/feeds` — list configured feeds with last-run / status / data-dir size. Action affordances per row: pause, resume, decommission.
- `/feeds rm <id>` — explicit decommission flow (deletes DB row + data dir; the only way to wipe history).

Depends on: T-0214 (runtime, especially the `register_feed_runtime` stub that this task fills in).

## Type / Priority

- Feature.
- P1 — without this, users can only configure feeds by directly inserting DB rows, which is bad UX. Task closes the I-0039 user-facing loop.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] **`/watch` command** parsed in `arawn-tui/src/command.rs`. Three forms:
  - `/watch <provider>` — list templates the provider exposes; user picks one in a modal.
  - `/watch <provider> <template>` — open a parameter-collection picker (if the template has discoverable params like a Slack channel list, fetch + render; otherwise free-form text).
  - `/watch <provider> <template> <param=value> ...` — register directly without a picker.
- [ ] **Cross-cutting registration flow**:
  - Insert row into `feeds` table.
  - Validate params via `template.validate()`.
  - Compute initial cursor (template-defined).
  - Write initial `meta.json`.
  - Register cloacina cron task via the runtime's `register_feed_runtime(&Feed)` (stub from T-0214 gets a real impl here).
  - Emit a `[integration] feed <id> registered` system message.
- [ ] **Discovery pickers** for watched-space templates:
  - `slack/channel-archive` → picker fetches `slack_channels_list` and renders a selectable list. Tab to filter.
  - `jira/project-tracker` → picker fetches accessible projects.
  - `confluence/space-archive` → picker fetches accessible spaces.
  - `drive/folder-sync` → picker prompts for a path or id (no folder enumerator yet).
- [ ] **`/feeds` command** opens a modal listing configured feeds:
  - Columns: `template`, `id`, `cadence`, `last_run`, `status`, `data_size`.
  - Per-row keys: `p` pause, `r` resume, `d` decommission, `Enter` open feed_dir in OS file browser.
- [ ] **`/feeds rm <id>` decommission flow**:
  - Confirmation modal — clearly states "this deletes <N> files (<size>) and the feed record. Cannot be undone."
  - On confirm: deregister cloacina cron task, delete DB row, recursively delete `feed_dir`. All transactional — if any step fails, the row stays as `enabled=0` and the failure is surfaced.
- [ ] **Pause / resume**:
  - Pause: set `enabled=0`, deregister cloacina task. Data dir untouched.
  - Resume: set `enabled=1`, re-register cloacina task with the persisted cadence + cursor.
- [ ] **TUI rendering** uses the existing modal pattern from T-0060 / T-0210 — inherits the visual style (Catppuccin palette, gutters, etc.).
- [ ] **Error UX**:
  - Cadence floor violation (cron < 15min) → modal error before the feed is created.
  - Template name not found → modal error with "did you mean: ..." suggestions if there's a similar template.
  - Param validation failure → inline error in the picker.
- [ ] **Tests**:
  - `command::tests::watch_parses_provider_template_params`
  - `feeds_modal::tests::pause_toggles_enabled_and_deregisters_cron`
  - `feeds_modal::tests::decommission_requires_confirm_and_deletes_dir`
  - `register_feed_runtime_round_trips_through_cloacina` (integration)
- [ ] `angreal check workspace` and `angreal check clippy` clean. Existing TUI snapshot tests still pass.

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

### 2026-05-08 — slice 1: non-interactive `/watch` + read-only `/feeds`

End-to-end registration plumbing landed. Filling in T-0214's `register_feed_runtime` stub with the real flow.

**arawn-feeds**:
- `FeedRuntime::register_feed_dynamic(template, feed_id, params, cadence_override)` — full flow: validate template + params + cadence, persist row, write initial `meta.json`, register cloacina cron. Failure rolls back the DB row so `/watch` re-tries aren't blocked by half-baked state.
- `FeedRuntime::list_summaries()` — every feed (enabled + paused) with last-run health from `meta.json` + recursive disk-size walk.
- `FeedSummary` Serializable type re-exported from the crate root.

**arawn-service**:
- Trait gains `feed_register(spec) -> FeedSummaryDto` + `feed_list() -> Vec<FeedSummaryDto>`.
- New `FeedRegisterSpec` and `FeedSummaryDto` types in `service::types`.

**arawn (server)**:
- `LocalService::set_feed_runtime(Arc<FeedRuntime>)` — main.rs hands the live runtime to the service after `arawn_feeds::start`.
- Both trait methods implemented; `feed_register` emits a `[feeds] feed <id> registered` ServerNotice.
- `ws_server` registers the two new RPC methods.

**arawn-tui**:
- `/watch` and `/feeds` registered as built-in commands.
- `parse_watch_args` lexes `<template> <feed_id> [k=v]... [@cadence="..."]` with quote-aware tokenization. Param values are typed via `serde_json::from_str` — `count=5` arrives as a number, `enabled=true` as bool, anything else as string. Cron with spaces must be quoted (`@cadence="*/30 * * * *"`).
- `CommandResult::FeedRegister(WatchSpec)` and `FeedList` variants; event_loop dispatches via `WsClient::feed_register` / `feed_list`.
- Slice-1 rendering is a chat-pane system message — modal upgrade lands in slice 2.
- Helper `format_feed_list` renders a markdown-ish list with cadence, state, last-run, status, human-readable size.

**Tests**:
- `command::tests::watch_parses_template_id_and_string_param` — minimal form.
- `command::tests::watch_parses_typed_and_quoted_params_and_cadence_override` — typed values + quoted cron.
- `command::tests::watch_rejects_missing_args_and_bad_template` — gates.
- `command::tests::watch_command_dispatch_returns_feed_register` — end-to-end through registry.
- `command::tests::feeds_command_dispatch_returns_feed_list`.
- `tests/dynamic_register.rs::dynamic_register_full_flow` — real `DefaultRunner` + sqlite, exercises validate→insert→meta→cron through to `list_summaries`.
- `tests/dynamic_register.rs::dynamic_register_rolls_back_on_unknown_template` — failure path leaves the DB clean.

129 arawn-feeds tests + 144 arawn-tui tests green. Workspace + clippy clean.

### 2026-05-08 — slice 2: pause / resume / decommission row actions

Subcommand path landed end-to-end. Modal upgrade deferred to slice 2b — chat-line confirm is fine for now and proves the backend.

**arawn-feeds** (`runtime.rs`):
- `pause_feed(id)` — drops the cloacina cron schedule (load-bearing step first; if cron deletion fails the row stays alone), then flips DB `enabled=0`. Idempotent — pausing an already-paused feed is a no-op from the caller's perspective.
- `resume_feed(id)` — re-registers cron via `register_one` first; only flips DB `enabled=1` once the schedule is back so we never have a row that says "active" with nothing firing.
- `remove_feed(id) -> RemoveOutcome` — order is cron→fs→row: if cron deletion fails we haven't lost any data, if fs deletion fails the row stays so the user can retry. Returns `bytes_wiped` so the confirm message can show what was removed.
- New `delete_schedule_for(workflow_name)` helper looks up by workflow name + delete; idempotent if the schedule isn't there.
- `RemoveOutcome { record, bytes_wiped }` re-exported.

**Why delete cron rather than just disable**: cloacina's `register_cron_workflow` always inserts a new schedule (no upsert). After pause + resume across a server restart, a `set_enabled(false)` approach would leave the disabled schedule in cloacina's DB and the boot loop would create a new one alongside it — double firing. Deleting the schedule on pause keeps arawn-feeds DB authoritative.

**arawn-service**: trait gains `feed_pause`/`feed_resume`/`feed_remove`. New `FeedRemoveDto` carries `id/template/bytes_wiped`.

**arawn (server)**: trait impls dispatch to the runtime; each emits a category="feeds" `ServerNotice`. `ws_server` registers three new RPC methods.

**arawn-tui**:
- `parse_feeds_args` now parses `pause|resume|rm <id>` subcommands. Empty args → list (slice 1 behavior).
- `/feeds rm <id>` → confirm preview (template, dir, on-disk size). User re-runs `/feeds rm <id> --yes` to commit. The `--yes` (or `-y`) flag is wired so scripts can skip the confirm step.
- New `CommandResult` variants: `FeedPause(String)`, `FeedResume(String)`, `FeedRemove { feed_id, confirmed }`.
- `event_loop` dispatches via `WsClient::feed_pause` / `feed_resume` / `feed_remove`.
- For the unconfirmed `/feeds rm <id>` form, the event loop fetches the current summary first to show "deletes <N> bytes from `<dir>`" — same info the AC wanted in the modal.

**Tests**:
- `command::tests::feeds_pause_and_resume_dispatch`
- `command::tests::feeds_rm_requires_confirm_flag`
- `command::tests::feeds_pause_without_id_is_a_usage_message`
- `command::tests::feeds_unknown_subcommand_lists_usage`
- `tests/dynamic_register::pause_resume_round_trip_through_cloacina` — real `DefaultRunner`; verifies cron schedule disappears on pause and re-appears on resume.
- `tests/dynamic_register::remove_wipes_cron_row_and_data_dir` — drops a marker file in the feed dir, confirms it's wiped + bytes_wiped is non-zero, plus row + cron schedule both disappear.
- `tests/dynamic_register::pause_unknown_feed_returns_invalid_params` — error path.

132 arawn-feeds tests + 148 arawn-tui tests green. Workspace + clippy clean.

**Remaining slices**:
- Slice 2b (later, optional): full ratatui modal with `p`/`r`/`d` keybindings on top of the same backend. Deferred — chat-line UX is functional and the modal is pure sugar.
- Slice 3: discovery pickers (Slack channels, Jira projects, Confluence spaces, Drive folders).
- Slice 4: auto-create on `/connect` — absorbs deferred bits from T-0220 / T-0216 / T-0221 / T-0223.
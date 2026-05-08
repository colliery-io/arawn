---
id: phase-6-watch-slash-command-feeds
level: task
title: "Phase 6 — /watch slash command + /feeds management UX"
short_code: "ARAWN-T-0219"
created_at: 2026-05-07T00:42:53.527120+00:00
updated_at: 2026-05-07T00:42:53.527120+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

*To be added during implementation*
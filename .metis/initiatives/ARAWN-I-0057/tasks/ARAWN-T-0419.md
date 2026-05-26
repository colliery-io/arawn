---
id: t-c-wire-filesystemfeedtemplate
level: task
title: "T-C: Wire FilesystemFeedTemplate into FeedRuntime + projection"
short_code: "ARAWN-T-0419"
created_at: 2026-05-25T15:47:56.656632+00:00
updated_at: 2026-05-26T02:15:50.465031+00:00
parent: ARAWN-I-0057
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0057
---

# T-C: Wire FilesystemFeedTemplate into FeedRuntime + projection

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0057]]

## Objective

Register `FilesystemFeedTemplate` with the feed runtime so the user can actually use it, and wire signals through the standard projection path so `feed_search` returns matches. After this task: a filesystem feed registered in `arawn.toml` actually fires on the cron and lands rows in the search index.

## Files

- **Modify:** `crates/arawn-feeds/src/runtime.rs` — register `FilesystemFeedTemplate` in `register_default_templates`
- **Modify (possibly):** `crates/arawn-feeds/src/dispatch.rs` — verify the filesystem signals route through projection like other feeds; if a new projection table is required, add it
- **Modify (possibly):** projection schema files under `crates/arawn-feeds/src/` if the existing `feed_signal` table doesn't fit the shape

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

## Acceptance Criteria

## Acceptance Criteria

- [ ] `FilesystemFeedTemplate` appears in the registry after startup (visible via whatever listing mechanism exists today — e.g. `feed_search` returning matches scoped to `source=filesystem`, or a runtime introspection point)
- [ ] A feed registered in `arawn.toml` with `template = "filesystem/folder"` + `params = { root = "..." }` is scheduled by cloacina at the documented 30s cadence
- [ ] On first run after registration, every matching file in `root` produces a row queryable via `feed_search "<filename>"`
- [ ] On subsequent runs, only changed files produce new rows (verified by checking row count delta)
- [ ] No regressions: `cargo test -p arawn-feeds` green; `angreal check workspace` green
- [ ] KNOWN_FEED_TYPES (or equivalent allowlist that gated GitHub feeds in T-0345) includes filesystem signals so they aren't silently dropped by feed_search

## Notes

- T-0345 (completed) added GitHub projection tables to `KNOWN_FEED_TYPES` — same pattern likely applies here
- Look at how `gmail/inbox` or `slack/channel-archive` flow from `run()` output to `feed_search` hits — that's the path to replicate
- If a new projection table is needed, mirror the existing ones (schema + migration + insert path) rather than inventing a new pattern

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

**2026-05-25 — Done.** Wired the template end-to-end. (Note: the task's file list named `runtime.rs::register_default_templates`, which doesn't exist — registration actually lives in `templates/mod.rs::default_registry()`, and the projection reader belongs in the `arawn-projections` crate, not `arawn-feeds`. Did the right thing per the actual architecture.)

Changes:
- **Registered** `FilesystemFeedTemplate` in `arawn-feeds/src/templates/mod.rs::default_registry()`.
- **New projection module** `arawn-projections/src/filesystem.rs` (`FilesystemSignalProjection`, `FEED_TYPE = "filesystem_signals"`, `walk_feed_dir`). Reads `signals.jsonl`, one row per event. `source_id` = stable hash over (path, event, ts, mtime, size) → distinct events get distinct rows, re-reading the append-only log is idempotent (dedup via `missing_source_ids`). Title = filename, body_text = `rel_path\nevent` so `feed_search "<filename>"` hits. 6 unit tests.
- **Dispatch arm** for `"filesystem"` provider in `arawn-projections/src/dispatch.rs`; module exported from `lib.rs`. Projection tables are created lazily on first write (no startup schema list needed).
- **Extractor fan-out**: added `"filesystem" => ["filesystem_signals"]` to `arawn-feeds/src/dispatch.rs::projection_feed_types_for`.
- **feed_search allowlist**: added `"filesystem_signals"` to `KNOWN_FEED_TYPES`.

Verification:
- New end-to-end test `run_feed_projects_filesystem_signals` (arawn-feeds dispatch): registers a `filesystem/folder` feed, runs through `run_feed` with a real in-memory `ProjectionStore`. 2 files → 2 rows; no-change rerun → 0 new; +1 file → +1 row; `fts_search("gamma")` finds it. Green.
- `cargo test -p arawn-projections --lib filesystem` 6/6, `-p arawn-feeds --lib dispatch` 5/5, feed_search drift tests 2/2.
- `angreal check workspace` green.

Cadence acceptance criterion says "30s cadence" — superseded by the 15-min floor flagged in T-A; the feed schedules at `*/15 * * * *`.
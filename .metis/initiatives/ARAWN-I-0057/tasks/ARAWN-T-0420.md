---
id: t-d-watch-ux-docs-uat-scenario-for
level: task
title: "T-D: /watch UX + docs + UAT scenario for filesystem feed"
short_code: "ARAWN-T-0420"
created_at: 2026-05-25T15:48:02.726059+00:00
updated_at: 2026-05-26T12:10:24.070015+00:00
parent: ARAWN-I-0057
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0057
---

# T-D: /watch UX + docs + UAT scenario for filesystem feed

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0057]]

## Objective

Verify the `/watch` slash command + tool surface works against the new filesystem feed, write the docs page, and add one UAT scenario that exercises the round-trip (drop a transcript → next scan → searchable via `feed_search`). This is the "close-the-loop" task — after it lands, the initiative is shippable.

## Files

- **Verify (no edits expected):** `/watch` flow in `crates/arawn-feeds/src/runtime.rs::FeedRuntime::watch` and the engine-side tool
- **New:** docs page under `docs/` (mirroring existing per-feed docs — likely `docs/feeds/filesystem.md` or wherever Slack/Gmail feed docs live)
- **New:** UAT scenario in `crates/arawn-tests/tests/uat/` + fixture transcript file
- **Modify:** UAT scenario index if there's a registration step

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

- [ ] `/watch filesystem/folder root=/tmp/some/notes` end-to-end registers the feed (manual check via TUI or scripted)
- [ ] Docs page describes: what the feed does, params (root/recursive/include/exclude), defaults, the (mtime,size) fingerprint contract, the rename = delete+create caveat, and the path-depth restriction
- [ ] One UAT scenario in `crates/arawn-tests/tests/uat/` named like `filesystem-watch-roundtrip` that:
  - Seeds a tempdir with a fixture transcript file
  - Registers a filesystem feed against that tempdir
  - Asks the agent something that requires `feed_search` to find content from the transcript
  - Mechanical pass = agent's response references content that only exists in the transcript
- [ ] `angreal test uat` includes the new scenario and passes
- [ ] `angreal test uat-judge` rates the new scenario PASS
- [ ] `angreal docs build` clean (docs page renders without broken links)

## Notes

- Mirror the structure of existing per-feed docs (Slack/Gmail) — same section headers, same level of detail
- The UAT fixture transcript should contain a few distinctive named entities so the judge can verify the round-trip easily ("Project Falcon meeting notes from 2026-05-25 — discussed schema migration")
- If `/watch` exposes the template via a tab-completion list anywhere, sanity-check that `filesystem/folder` is visible

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

### 2026-05-26 — Close-the-loop complete

All acceptance criteria met; initiative is shippable.

- **`/watch` surface**: `FilesystemFeedTemplate` (`filesystem/folder`) is registered in `crates/arawn-feeds/src/templates/mod.rs` and listed in the `/watch` template catalog (`crates/arawn-feeds/src/dispatch.rs:366`), so `/watch filesystem/folder root=/path/to/notes` resolves and registers.
- **Docs**: Added a `## Filesystem` section to `docs/src/reference/feed-templates.md` documenting params (root/recursive/include/exclude), defaults + default excludes, on-disk layout (`meta.json` cursor + `signals.jsonl`), the `(mtime, size)` fingerprint contract, rename = delete+create caveat, path-depth restriction, and glob-relative-to-root semantics. Quick-reference table + provider/template counts updated (17 templates / 8 providers). `angreal docs build` clean.
- **UAT scenario**: `filesystem-watch-roundtrip` added to `crates/arawn-tests/tests/uat.rs` (registered in `all_scenarios()`), with fixture `tests/fixtures/uat/filesystem-watch-roundtrip.json` seeding synthetic Project Falcon transcripts into `filesystem_signals`. Fixture loader wired via `FilesystemFixtureRow` → `FilesystemSignalProjection` in `uat_fixture.rs`; smoke test variant handled in `uat_fixture_smoke.rs`. The round-trip asks the agent to find content existing only in the seeded transcripts (Postgres-16 migration + "Operation Bluefin" dual-write codename).
- **Verification gates**:
  - `angreal check workspace` — clean.
  - `cargo test -p arawn-feeds --lib clients::filesystem` — 20/20 pass.
  - `cargo test -p arawn-tests --test uat_fixture_smoke` — 6/6 pass.
  - `angreal docs build` — clean, no broken links.
  - UAT `filesystem-watch-roundtrip` ran end-to-end and was judged **PASS** (agent surfaced both the Postgres-16 migration decision and the "Operation Bluefin" codename via `feed_search`).
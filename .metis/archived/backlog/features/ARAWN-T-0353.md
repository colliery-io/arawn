---
id: nightly-recovery-back-fills-missed
level: task
title: "Nightly recovery back-fills missed daily/weekly ceremony tablets"
short_code: "ARAWN-T-0353"
created_at: 2026-05-19T12:07:51.424200+00:00
updated_at: 2026-05-19T12:07:51.424200+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Nightly recovery back-fills missed daily/weekly ceremony tablets

## Objective

Implement back-fill for missed daily/weekly ceremony tablets. Today, when a cron tick fires while arawn is down (laptop closed, server stopped), the ceremony for that day is simply absent — there's no retroactive compose path. The "watch, check, summarize, nudge" loop is supposed to survive laptop sleep.

## Impact

- **Severity:** P2 — vision-aligned feature gap. Users opening arawn on Monday after a weekend laptop close don't see Friday's daily tablet, breaking the continuity loop.
- **Affected users:** anyone who uses ceremonies and doesn't keep arawn running 24/7 (i.e. most users).
- **Cascade:** the retro's `priority_completion_ratio` detector needs a complete priority history. Gaps skew the output.

## Current behavior

- `crates/arawn-ceremonies/src/nightly.rs` runs an hourly `sweep_unreviewed_retros` task that only transitions stale `open` retro tablets to `unreviewed`.
- There is no path to compose a back-dated daily/weekly tablet when the cron tick was missed.

## Proposed design

A nightly back-fill loop (or a startup back-fill on `arawn serve` boot — pick one):

1. For each enabled ceremony (`daily`, `weekly`, `retro`):
   - Walk dates from `last_tablet_at` forward to `now`, applying the configured cron schedule + timezone.
   - For each missed date, compose a back-dated tablet (gather + agent compose) with the canonical `ceremony.date` field set to that historical date, not today.
2. Cap how far back to look (e.g. 14 days) to avoid runaway back-fill after a long absence.
3. Recovery tablets are marked so the UI can distinguish them (small badge or note).

## Implementation notes

- Compose pipeline: `arawn-ceremonies/src/{plugins/daily,plugins/weekly,plugins/retro}.rs` — the per-ceremony plugin's `compose` function takes a date and produces a tablet. Should already be parameterized over date for retro replay; verify for daily/weekly.
- Storage: tablets live in `arawn.db` `ceremonies` table; back-fill writes rows with the historical date.
- Configuration: respect `[ceremonies.<kind>].enabled = false` — skip back-fill for disabled ceremonies.
- Trade-off: a long absence (week-long vacation) produces a flurry of back-dated tablets. They're not actionable (you weren't there) but they preserve the audit trail. The 14-day cap bounds the damage.

## Acceptance criteria

- [ ] Missed daily / weekly / retro cron ticks are back-filled when arawn boots or on the nightly tick.
- [ ] Configurable look-back cap (default 14 days) to prevent runaway after long absences.
- [ ] Back-dated tablets carry the historical date as their canonical `ceremony.date`, not the back-fill run date.
- [ ] Retro detectors (`priority_completion_ratio`, etc.) operate correctly on back-filled history.
- [ ] `docs/src/reference/ceremonies-tools.md` and `docs/src/explanation/ceremonies.md` rewritten to describe the back-fill (currently they explicitly say back-fill is not implemented).

Surfaced during ARAWN-I-0051 doc triple-check. The doc previously claimed back-fill existed (`nightly.rs` plugin-header comment) but the code did not; the doc was rewritten to reflect reality.

**Superseded by [[ARAWN-I-0052]]** (2026-05-19). The discussion
upgraded this from a "stub back-fill" task to a full initiative
covering pinned date windows, historical dispatch, the 14-day
back-fill cap, configurable retro cadence, and a current-time
prompt header. See I-0052 tasks T-0364–T-0369.

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

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

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
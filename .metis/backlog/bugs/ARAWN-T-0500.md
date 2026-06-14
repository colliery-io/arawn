---
id: feed-search-fts-recall-verbose
level: task
title: "feed_search FTS recall — verbose queries AND to zero; add OR-fallback"
short_code: "ARAWN-T-0500"
created_at: 2026-06-14T18:12:50.545368+00:00
updated_at: 2026-06-14T18:53:49.529176+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# feed_search FTS recall — verbose queries AND to zero; add OR-fallback

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Fix `feed_search` (and all projection FTS reads) returning **zero results for realistic multi-word queries**. Surfaced by the `filesystem-watch-roundtrip` UAT scenario (2026-06-14, gemma4:31b-cloud): the agent searched "Project Falcon meeting notes" over seeded `filesystem_signals` rows that clearly contained the answer, got 0 hits, and correctly reported nothing — a judge FAIL (completion 1/5). The other 13 UAT scenarios passed.

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: Anyone using `feed_search` (the agent's primary feed/notes retrieval tool) with a natural-language query — i.e. the common case.
- **Reproduction Steps**:
  1. Seed a projection row whose `body_text` contains "Postgres 16" and "Operation Bluefin" (no token "meeting").
  2. `feed_search("Project Falcon meeting notes")`.
  3. Get 0 results (should surface the row).
- **Expected vs Actual**: Expected the row to surface (it contains "Falcon"/"Project"/"migration"); actual 0, because `escape_fts5` quotes each token and **joins with a space → FTS5 AND**, requiring every token (incl. the absent "meeting"). Punctuation tokens like `dual-write` previously also crashed MATCH (already quoted by `escape_fts5`, so neutralized — the remaining defect is the AND join).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] A multi-word query where not all tokens are present still surfaces matching rows (recall), ranked by relevance.
- [ ] A precise multi-term query whose tokens all match still works (no precision regression) — AND stays the primary query; OR is a fallback only when AND yields 0.
- [ ] Punctuation-bearing queries (`dual-write`, `RFC-0042`, `foo:bar`) never error and match the intended rows.
- [ ] Unit tests in `arawn-projections` for: AND-hit (all present), OR-fallback (some absent), punctuation, empty/whitespace.
- [ ] The `filesystem-watch-roundtrip` UAT scenario passes the judge (re-run to confirm).
- [ ] `angreal check all` + `angreal test unit` green.

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

**2026-06-14 — Fixed & verified.**
- `escape_fts5` kept (AND, precise) + new `escape_fts5_or` (OR-join of quoted tokens), both via a shared `escape_fts5_joined`. `ProjectionStore::fts_search` now runs the AND query first and, **only if it returns 0 and the query had >1 token, retries with OR** (ranked by FTS rank). Precision preserved; recall added. Punctuation was already neutralised by per-token quoting (T-0370), so no separate escaping change needed.
- Tests (arawn-projections, 62 pass): `verbose_query_falls_back_to_or_for_recall` (absent token no longer zeroes the result), `or_fallback_does_not_fire_when_and_matches` (no over-broadening when AND hits), `escape_or_joins_tokens_with_or`. Existing `multi_token_is_implicit_and` still green (precision unchanged).
- **UAT re-verified:** re-ran `filesystem-watch-roundtrip` against the rebuilt binary → judge **pass=true, completion 5, artifact 4** (was pass=false 1/1). Agent made a single `feed_search` call (vs 13 flailing before), cited both required facts (Postgres 16 + Operation Bluefin), no hallucination. Full UAT suite now **14/14**.
- Gate clippy + fmt clean.
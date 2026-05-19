---
id: permission-rule-content-pattern
level: task
title: "Permission rule content-pattern matches JSON, not extracted argument"
short_code: "ARAWN-T-0344"
created_at: 2026-05-19T12:07:40.182802+00:00
updated_at: 2026-05-19T14:54:01.248842+00:00
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

# Permission rule content-pattern matches JSON, not extracted argument

## Objective

Fix permission rule content-pattern matching so `shell(git *)` matches against the `command` field (not the JSON-wrapped arguments). Today, every content-pattern rule in user `arawn.toml` files silently no-ops in production.

## Impact

- **Severity:** P1 — silent security failure. Deny rules that users believe protect them never fire.
- **Affected users:** anyone who writes content patterns in `[permissions]` (essentially every user who locks down permissions).
- **Reproduction:**
  1. Add `deny = ["shell(rm -rf *)"]` to `arawn.toml`.
  2. Have the agent run `shell { command: "rm -rf /tmp/foo" }`.
  3. Watch the call succeed — the deny pattern was matched against the literal text `{"command":"rm -rf /tmp/foo"}` (starts with `{`), not against `rm -rf /tmp/foo`.
- **Expected vs actual:** the deny should fire and the call should be rejected.

## Implementation notes

- `crates/arawn-engine/src/permissions/rules.rs::glob_match` is called with `tool_input = arguments.to_string()` from `crates/arawn-engine/src/query_engine.rs:847-856`.
- `crates/arawn-engine/src/approval/allowlist.rs::ArgShape::for_tool` (lines 44-58) already extracts per-tool fields. Reuse this in the rules path — pass `tool_name` + extracted-field string to `glob_match`.
- Update unit tests in `rules.rs:248-251` to feed JSON-shaped arguments (matching production), not naked command strings.

## Acceptance criteria

- [ ] `shell(*)` content patterns match against the `command` field.
- [ ] `file_read(*)` / `file_write(*)` / `file_edit(*)` / `glob(*)` / `grep(*)` match against the relevant `path` field.
- [ ] `web_fetch(*)` matches against the `url` field.
- [ ] Other tools: match against a sensible primary argument, or fall through to "no content match" with a debug log.
- [ ] Unit tests in `rules.rs` updated to feed JSON-shaped arguments.
- [ ] Sharp-edge callouts in `docs/src/reference/permissions.md` and `docs/src/how-to/lock-down-permissions.md` removed.

Surfaced during ARAWN-I-0051 doc triple-check (commit 7037763).

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

## Status Updates

### 2026-05-19 — Content-pattern extraction shipped

- New `extract_content_for_match(tool_name, raw_input)` in
  `crates/arawn-engine/src/permissions/rules.rs` pulls the
  tool's primary argument out of the JSON input:
  - `shell` / `Bash` → `command`
  - `file_read` / `file_write` / `file_edit` (+ camelCase) →
    `path`, falling back to `file_path`
  - `glob`, `grep` → `pattern`
  - `web_fetch` / `WebFetch` → `url`
  - `safe_env` → `name`
  - Other tools fall back to the raw JSON with a debug log.
- `PermissionRule::matches` now calls the extractor before
  globbing the content pattern. Tool-name match path unchanged.
- Backward compatibility: unknown tools fall through to raw
  JSON matching — pre-existing pattern widening (e.g.
  `shell(*git*)`) still works; the new clean form
  (`shell(git *)`) also works.
- **Unit tests:** added 4 cases covering shell command
  extraction, file path extraction, web_fetch URL extraction,
  and the unknown-tool raw-input fallback.
- **Doc fixes:** stripped the "sharp edge" callout in both
  `docs/src/how-to/lock-down-permissions.md` and
  `docs/src/reference/permissions.md`. Replaced with a
  per-tool field-mapping table pointing at the new helper.
- `cargo test -p arawn-engine --lib permissions` 62/0.
  `angreal check workspace` green.
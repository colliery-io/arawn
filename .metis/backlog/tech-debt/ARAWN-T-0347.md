---
id: accept-on-ux-add-permissions
level: task
title: "/accept on UX + add [permissions].permission_mode TOML key"
short_code: "ARAWN-T-0347"
created_at: 2026-05-19T12:07:44.653236+00:00
updated_at: 2026-05-19T12:07:44.653236+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# /accept on UX + add [permissions].permission_mode TOML key

## Objective

Two related changes to make permission-mode UX less surprising:

1. **Rename `/accept on`** (currently maps to `bypass` mode — full autonomy) to something less ambiguous, e.g. `/accept all` or `/accept full`. The current naming reads as "turn on permission asking" but actually means the opposite.
2. **Add a `[permissions].permission_mode` TOML key** so users can pin a starting mode in `arawn.toml`. Today the mode is purely runtime state — every server restart drops to `default`.

## Impact

- **Severity:** P2 — UX confusion + missing config knob.
- **(1) /accept on:** a user who types `/accept on` expecting "turn on permission prompts" actually disables all prompts. The dispatcher comment at `command.rs:667` says `"on Full autonomy (bypass all permissions)"`, so the intent is documented, but the slash word is misleading.
- **(2) permission_mode TOML key:** users running arawn unattended (CI, scheduled workflows) want `permission_mode = "bypass"` declared in TOML, not typed every session.

## Implementation notes

### (1) `/accept` UX

- Dispatch in `crates/arawn-tui/src/command.rs:660-669` maps `on → bypass`, `edits → accept_edits`, `off → default`.
- Proposed: keep `off` as-is, rename `on → full`, keep `edits`. Print a deprecation message for the old `on` for one release.
- Update help text in `command.rs:667` and `docs/src/reference/slash-commands.md` § `/accept`, `docs/src/how-to/lock-down-permissions.md` § "Switching modes at runtime", `docs/src/reference/permissions.md`.

### (2) `permission_mode` TOML key

- Add `permission_mode: Option<PermissionMode>` to `crates/arawn-engine/src/permissions/config.rs::PermissionConfig` (or top-level if cleaner).
- Read it at startup in `LocalService::new` (or wherever `PermissionMode::Default` is currently chosen).
- Update `docs/src/reference/config-schema.md` to add the key back (it was removed during the triple-check pass since it didn't exist).

## Acceptance criteria

- [ ] `/accept full` works as today's `/accept on`; `/accept on` either removed or aliased with a deprecation warning.
- [ ] `[permissions].permission_mode = "bypass"` in `arawn.toml` sets the starting mode at server boot.
- [ ] Invalid values log a warn and fall back to `default`.
- [ ] Docs updated.

Surfaced during ARAWN-I-0051 doc triple-check.

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
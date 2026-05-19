---
id: foreground-shell-falls-back
level: task
title: "Foreground shell falls back unsandboxed on missing bwrap / Windows"
short_code: "ARAWN-T-0346"
created_at: 2026-05-19T12:07:43.157093+00:00
updated_at: 2026-05-19T15:01:35.754299+00:00
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

# Foreground shell falls back unsandboxed on missing bwrap / Windows

## Objective

Make the foreground `shell` tool **fail closed** on unsupported sandbox platforms (missing `bwrap` on Linux, Windows) instead of falling back to unsandboxed execution with a warning. Today this is a security gap that contradicts the documented sandbox guarantees.

## Impact

- **Severity:** P1 — security regression. On a Linux host without `bwrap` installed, or on Windows, the agent's `shell` calls execute commands with NO sandbox protection (no sensitive-path deny list, no network gating, no env scrubbing) — only a warning prefix `[WARNING: Command ran without sandbox protection ...]` precedes the output.
- **Affected users:** Linux users who haven't installed `bubblewrap` (very common on macOS-only devs trying arawn on a Linux test box; also default for distros that don't pre-install it). Windows users period.
- **Asymmetry:** the *background-task* shell path (`init_sandbox_for_background` in `tools/shell.rs:221-225`) already fails closed. Foreground doesn't.

## Implementation notes

- `crates/arawn-engine/src/tools/shell.rs:404-417` catches `SandboxExecError::Unavailable` and dispatches to `execute_unsandboxed`. Change this to return an error like the background path does.
- The error message should be actionable: "Sandbox unavailable on this platform. Install bwrap (`apt install bubblewrap` / `pacman -S bubblewrap`) or run on macOS." Don't silently execute.
- Update `docs/src/reference/shell-sandbox.md` once the fix lands — the current text accurately describes the buggy behavior; reword to describe the fail-closed behavior.

## Acceptance criteria

- [ ] Foreground `shell` returns `SandboxExecError::Unavailable` (or equivalent) when `SandboxExec::available()` is false, instead of dispatching `execute_unsandboxed`.
- [ ] Error message guides the user toward installing the sandbox backend.
- [ ] Regression test (Linux host without `bwrap` available, or a mock) confirming fail-closed.
- [ ] `docs/src/reference/shell-sandbox.md` reworded.

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

### 2026-05-19 — Foreground shell fail-closed

- `ShellTool::execute` in
  `crates/arawn-engine/src/tools/shell.rs` no longer dispatches
  to `execute_unsandboxed` on `SandboxExecError::Unavailable`.
  Returns `ToolOutput::error(sandbox_unavailable_message(detail))`
  — foreground now mirrors the background path's fail-closed
  behavior.
- Error message is actionable: explains the sandbox is
  unavailable, lists install commands per platform
  (`apt install bubblewrap` / `pacman -S bubblewrap` on Linux,
  `sandbox-exec` built-in on macOS, unsupported on Windows),
  and points at the sandbox doc.
- `execute_unsandboxed` removed — no longer reachable.
  `safe_env` is still used by the sandboxed-execution env scrub,
  so it stays.
- **Regression test** `sandbox_unavailable_message_is_fail_closed`
  asserts the message format, mentions install commands, and
  explicitly rejects the pre-fix wording ("ran unsandboxed",
  "without sandbox protection") so a future accidental revert
  is caught.
- **Doc fix** in `docs/src/reference/shell-sandbox.md`:
  stripped the per-platform fallback notes from the support
  table, removed the "sharp edge" security callout, replaced
  both with a "fail-closed" guarantee callout.
- `cargo test -p arawn-engine --lib shell::tests` 8/0 (plus
  the new sandbox_unavailable test). `angreal check workspace`
  green.
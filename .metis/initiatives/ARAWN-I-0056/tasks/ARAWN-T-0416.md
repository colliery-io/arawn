---
id: t-e-docs-uat-close-gate
level: task
title: "T-E: Docs + UAT close gate — configure-hooks.md, end-to-end scenario, full UAT+judge"
short_code: "ARAWN-T-0416"
created_at: 2026-05-23T03:31:04.000000+00:00
updated_at: 2026-05-23T03:31:04.000000+00:00
parent: ARAWN-I-0056
blocked_by: [ARAWN-T-0413, ARAWN-T-0414, ARAWN-T-0415]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0056
---

# T-E: Docs + UAT close gate

## Parent Initiative

[[ARAWN-I-0056]]

## Objective

Land the user-facing docs for hooks configuration, add a UAT scenario that proves end-to-end integration, and run the full UAT + judge as the initiative's close gate.

## Acceptance Criteria

### Docs

- [ ] New `docs/src/how-to/configure-hooks.md` with:
  - One-paragraph what-and-why intro.
  - Table of the 17 V1 events with one-sentence descriptions and trigger sites.
  - Note that 8 additional events (Worktree, CwdChanged, FileChanged, Setup, TeammateIdle, Elicitation, PromptInjectionVerdict) are defined but not fired in V1.
  - JSON schema of `~/.arawn/settings.json` (the `hooks` block) — by event type, matcher fields, command shape, allow/block return semantics.
  - Three runnable examples:
    1. **Auto-format on save**: `PostToolUse` hook matching `tool_name: "file_write"` runs `prettier --write {{file_path}}`.
    2. **Audit log**: `SessionEnd` hook appends `{session_id, turn_count, timestamp}` to `~/.arawn/audit.jsonl`.
    3. **Block dangerous shell**: `PreToolUse` hook matching `tool_name: "shell"` rejects commands matching `rm -rf /` with a block reason the agent reads.
  - Cwd note: hooks run with `cwd = workstream root`.
- [ ] Add link to `docs/src/SUMMARY.md` under "How-to guides".
- [ ] `angreal docs build` clean.

### UAT

- [ ] New UAT scenario `hooks-fire-postpost` (or similar) in `crates/arawn-tests/tests/uat/scenarios/`:
  - Setup: write a `~/.arawn/settings.json` (in the scenario's data dir) with a `PostToolUse` hook on `file_write` that runs `echo "fired"` and writes to `${HOOK_DATA_DIR}/sentinel.txt`.
  - Run: scenario asks the agent to write a file via `file_write`.
  - Mechanical: agent completes the turn, `sentinel.txt` exists and contains "fired".
  - Judge: assesses the agent's final response is normal (not influenced by the hook, since PostToolUse is non-blocking).
- [ ] UAT scenario added to the registered scenarios list in `crates/arawn-tests/tests/uat/mod.rs` (or wherever the scenario registry lives).
- [ ] `angreal test uat` + `angreal test uat-judge` green at close.

### Validation

- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --lib` green.
- [ ] All 17 V1 events have at least one unit test (delivered in T-B/C/D) and the integration is end-to-end verified by the UAT scenario.

## Implementation Notes

- The docs should be Claude-Code-shaped enough that someone with Claude Code experience can pick it up quickly, but use arawn's own paths and naming.
- For the UAT sentinel approach: the scenario fixture needs to know where to put the hook config. Probably under the scenario's per-run data dir so it doesn't bleed into the user's real `~/.arawn`.
- The judge prompt for this scenario should focus on "did the agent behave normally" rather than "did the hook fire" — the mechanical check verifies the hook; the judge verifies the agent wasn't disrupted.

## Status Updates

*To be added during implementation*

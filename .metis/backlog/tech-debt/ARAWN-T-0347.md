---
id: accept-on-ux-add-permissions
level: task
title: "/accept on UX + add [permissions].permission_mode TOML key"
short_code: "ARAWN-T-0347"
created_at: 2026-05-19T12:07:44.653236+00:00
updated_at: 2026-05-19T16:51:07.909732+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# /autonomy command + [permissions] autonomy TOML key

## Objective

Replace the confusing `/accept on|off|edits` slash command with
`/autonomy ask|edits|full|plan`, rename the underlying
`PermissionMode` enum to match, and add a `[permissions] autonomy`
TOML key so users can pin a starting mode in `arawn.toml`.

Locked design decisions (from the T-0347 UX discussion, 2026-05-19):

- **One verb everywhere.** Slash, TOML, enum, audit log, and
  serde-JSON all use the same four words: `ask | edits | full
  | plan`.
- **No backward compatibility.** Breaking change: drop `/accept`
  entirely, drop the `/plan` top-level alias. Drop the
  `Default | AcceptEdits | Bypass` enum variant names.
- **Internal enum renames too.** `PermissionMode::Ask | Edits |
  Full | Plan`. Audit log strings, serde keys, and every match
  arm update with it.

## Surface mapping

| Concept | Slash | TOML | Enum |
|---|---|---|---|
| Ask before mutating | `/autonomy ask` | `autonomy = "ask"` | `PermissionMode::Ask` |
| Auto-accept edits | `/autonomy edits` | `autonomy = "edits"` | `PermissionMode::Edits` |
| Full autonomy | `/autonomy full` | `autonomy = "full"` | `PermissionMode::Full` |
| Plan mode | `/autonomy plan` | `autonomy = "plan"` | `PermissionMode::Plan` |

## Impact

- **Severity:** P2 — UX confusion + missing config knob.
- **Breaking change.** Anyone with `/accept` in muscle memory or
  `permission_mode = "default"` in their `arawn.toml` will need
  to migrate. Acceptable per the user's "full send" direction.

## Acceptance criteria

- [ ] `PermissionMode` enum renamed:
  `Default` → `Ask`, `AcceptEdits` → `Edits`, `Bypass` → `Full`,
  `Plan` unchanged. All match arms, Display, FromStr, serde
  attributes, and audit-log string literals updated.
- [ ] Slash command `/accept` removed. New `/autonomy` slash
  command registered with usage
  `/autonomy <ask|edits|full|plan>`.
- [ ] Slash command `/plan` removed. Plan mode is reachable via
  `/autonomy plan`.
- [ ] `[permissions] autonomy = "..."` TOML key added. Read at
  server boot (`LocalService::new` or wherever the starting
  `PermissionMode` is chosen). Invalid values log a warn and
  fall back to `Ask`. Absent → `Ask` (today's behavior).
- [ ] Help text + descriptions updated in `command.rs`.
- [ ] Docs updated:
  - `docs/src/reference/slash-commands.md` § `/accept` →
    `/autonomy`; remove `/plan`.
  - `docs/src/how-to/lock-down-permissions.md` § "Switching
    modes at runtime" + recipe examples.
  - `docs/src/reference/permissions.md` mode glossary.
  - `docs/src/reference/config-schema.md` `[permissions]` adds
    the `autonomy` row.
- [ ] Unit tests:
  - Slash command parsing for each of the 4 args.
  - Invalid arg returns a usage message.
  - TOML parse round-trip for each enum variant.
  - Invalid TOML value falls back to `Ask` with a warn.
- [ ] `angreal test unit` green. `angreal check workspace` green.

Surfaced during ARAWN-I-0051 doc triple-check; redesigned in the
T-0347 UX discussion 2026-05-19.

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

### 2026-05-19 — /autonomy shipped (breaking change)

- **Enum rename.** `PermissionMode::Default | AcceptEdits |
  BypassPermissions | Plan` →
  `PermissionMode::Ask | Edits | Full | Plan`. Dropped the two
  explicit `#[serde(rename = ...)]` attributes — the
  `rename_all = "lowercase"` derive now produces
  `"ask" | "edits" | "full" | "plan"`. 38 call sites
  bulk-renamed via sed.
- **Slash command.** `/accept` removed; `/plan` removed. New
  `/autonomy <ask|edits|full|plan>` registered, with a usage
  message that documents each posture.
- **TOML key.** New `[permissions] autonomy = "ask"` row in
  `PermissionConfig` (`Option<PermissionMode>`). Read at
  server boot in `arawn-bin::main` and threaded into
  `LocalService::with_permission_mode`. Absent → `Ask`.
  Invalid → whole `[permissions]` section warn-and-falls-back
  via the existing loader contract.
- **TUI surfaces** updated: status-bar label table and
  event-loop "permission mode set to ..." text use the new
  vocabulary (`FULL`, `EDITS`, `PLAN`, default `ASK`).
- **Service error message** updated to list
  `ask, edits, full, plan`.
- **Tests:**
  - `permission_mode_serde` round-trip on all four new
    strings.
  - `permission_mode_legacy_strings_fail` — `"default"`,
    `"accept_edits"`, `"bypass"` no longer deserialize.
  - `load_autonomy_from_toml` round-trip for all four values.
  - `load_autonomy_absent_is_none`.
  - `load_autonomy_invalid_value_falls_back_to_default`.
  - `execute_autonomy_each_valid_mode` slash parser.
  - `execute_autonomy_invalid_value_returns_usage_message`.
  - `execute_autonomy_no_arg_returns_usage_message`.
  - `legacy_slash_commands_no_longer_resolve`.
  - `registry_has_builtins` / `registry_matching_prefix`
    updated for the new registry shape.
- **Docs updated** in 7 files: `slash-commands.md` (replace
  /accept entry, drop /plan entry, refresh command index),
  `permissions.md` (mode table + glossary), `permission-model.md`
  (four-modes table + bullet explanations + plan-mode tip),
  `lock-down-permissions.md` (recipes rewritten), `config-schema.md`
  (new `autonomy` row, drop "no TOML key" note),
  `troubleshooting.md` (denied-shell row), `sub-agents.md`
  (plan-mode reference), `agent-tools.md` (category table).
- `cargo test --workspace --lib` 1706/0.
  `angreal check workspace` green.
---
id: phase-c-5-reference-cross-cutting
level: task
title: "Phase C-5: Reference — cross-cutting catalogs (CLI, slash-commands, config, env-vars, data-dir, troubleshooting)"
short_code: "ARAWN-T-0337"
created_at: 2026-05-19T01:39:40.292598+00:00
updated_at: 2026-05-19T02:33:09.675043+00:00
parent: ARAWN-I-0051
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0051
---

# Phase C-5: Reference — cross-cutting catalogs (CLI, slash-commands, config, env-vars, data-dir, troubleshooting)

## Parent Initiative

[[ARAWN-I-0051]]

## Objective

Write six top-level reference catalogs: every CLI flag/subcommand, every slash command (25+), every `arawn.toml` section/key, every env var (18), the full `~/.arawn/` directory map, and the consolidated troubleshooting tables. These are the highest-lookup-volume pages in the docs — pure tables, code blocks, no narrative.

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

- [x] `reference/cli.md` — all 5 subcommands (`serve`, `tui`, `plugin {…}`, `doctor`, `usage`) + global flags + `arawn plugin` subtree. ~110 lines.
- [x] `reference/slash-commands.md` — 26 commands alphabetized; subcommand syntax called out where relevant. ~130 lines.
- [x] `reference/config-schema.md` — every `[section]` + key: `[llm.<name>]`, `[engine]`, `[compactor]`, `[extraction]`, `[server]`, `[storage]`, `[prompts]`, `[sandbox]`, `[integrations.*]`, `[routing.*]`, `[ceremonies.<kind>]`, `[permissions]`, `[[mcp.servers]]`. ~140 lines.
- [x] `reference/env-vars.md` — runtime + LLM API keys + 11 integration OAuth env vars + GitHub App env vars + resolution order. ~70 lines.
- [x] `reference/data-directory.md` — full `~/.arawn/` tree as ASCII layout + "safe to delete" table + workstream isolation + encrypted-blob locations. ~95 lines.
- [x] `reference/troubleshooting.md` — server startup / TUI / OAuth / feeds / workflows / permissions tables + escape hatches. ~85 lines.
- [x] `SUMMARY.md` updated with the six new pages.
- [x] `angreal docs build` clean.

## Status Updates

### 2026-05-18 — Completed (uncommitted)

The biggest reference batch in the initiative. Source data pulled from:

- `crates/arawn/src/main.rs` (CLI clap defs at lines 43-111).
- `crates/arawn/src/plugin_cmd.rs` (plugin subtree).
- `crates/arawn-tui/src/command.rs::register_builtins` (lines 72-214; 26 commands).
- `crates/arawn/src/config.rs` (lines 1-440; full TOML schema with default values).
- `crates/arawn-mcp/src/config.rs` ([[mcp.servers]] schema).
- `crates/arawn-storage/src/layout.rs::DataLayout::v1` (eagerly-created dirs).

Drift fixes incorporated: feeds rm not /unwatch in slash-commands.md; /remember and /memory and /forget listed without WIP labels; full 25+ command list (vs the 6-8 documented before); 17-template note for feed-templates (will be reflected in feed-templates.md content in C-8); permissions modes documented from code.

These are the highest lookup-volume pages in the new doc set. They're austere by design — tables and code blocks, no narrative. Concept work happens in C-10 explanation pages.

Forward references to pages not yet written (`./permissions.md`, `./agent-tools.md`, `./feeds-overview.md`, `./ceremonies-tools.md`, `./todos-tools.md`, `./skills.md`, etc.) are intentional — they're inline markdown links, not `SUMMARY.md` entries. They resolve as C-6, C-7, C-8, C-9 land.
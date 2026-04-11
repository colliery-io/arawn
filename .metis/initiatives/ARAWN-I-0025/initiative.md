---
id: api-polish-protocol-versioning-cli
level: initiative
title: "API Polish: Protocol Versioning, CLI, and String-Typed Dispatch Elimination"
short_code: "ARAWN-I-0025"
created_at: 2026-04-09T23:59:38.545659+00:00
updated_at: 2026-04-10T23:28:04.045615+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: api-polish-protocol-versioning-cli
---

# API Polish: Protocol Versioning, CLI, and String-Typed Dispatch Elimination Initiative

## Context

Architecture review found inconsistent RPC method naming across 4 patterns, no protocol versioning, hand-rolled CLI parsing without `--help`, and string-typed dispatch as the root cause of two silent correctness bugs (LEG-009, LEG-010). This initiative addresses the API polish and developer experience improvements that reduce future bug risk.

**Review findings addressed:** R-19, R-20, R-22

**Dependencies:** R-22 (string-typed dispatch) depends on ARAWN-I-0024's arawn-tool extraction for `ToolCategory` enum home. R-19 (protocol versioning) is easier after ARAWN-I-0024's service trait completion.

## Goals & Non-Goals

**Goals:**
- Add RPC protocol versioning with `hello` handshake method (API-004, API-005)
- Standardize RPC method naming to `verb_noun` pattern (API-001)
- Replace hand-rolled CLI parsing with clap for `--help`, `--version`, validation (API-009)
- Replace string-typed tool dispatch with `Tool::category() -> ToolCategory` enum (LEG-009, LEG-010, EVO-06)

**Non-Goals:**
- Breaking API changes without migration period
- Full RPC framework replacement (just polish)
- GraphQL or REST alternatives

## Detailed Design

### Protocol Versioning (R-19)
Add `hello` RPC returning `{ version, methods }`. Rename inconsistent methods to `verb_noun`: `remember_fact` → `store_memory`, `forget_entity` → `delete_memory`, `memory_summary` → `get_memory_summary`, `list_available_commands` → `list_commands`. Wrap streamed events in Response envelope or document the state machine.

### CLI via Clap (R-20)
Define `#[derive(Parser)]` struct with subcommands for `serve`, `tui`, `plugin`. Current arg surface is small — straightforward migration.

### String-Typed Dispatch Elimination (R-22)
Add `ToolCategory` enum to `arawn-tool` crate. Add `fn category(&self) -> ToolCategory` to `Tool` trait. Replace `CORE_TOOLS`, `TASK_TOOLS`, `AGENT_TOOLS` filter constants with `registry.tools_by_category()`. Replace `tool_category(name)` string matching in permissions with `tool.category()`. Compile-time verification eliminates the casing bug class entirely.

## Implementation Plan

1. CLI via clap (Hours) — independent, quick win
2. Protocol versioning (Days) — depends on ARAWN-I-0024 service trait completion
3. String-typed dispatch elimination (Weeks) — depends on ARAWN-I-0024 arawn-tool extraction
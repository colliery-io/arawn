---
id: tokenjuice-style-rule-driven-tool
level: task
title: "TokenJuice-style rule-driven tool-output compaction layer"
short_code: "ARAWN-T-0274"
created_at: 2026-05-15T14:12:50.950323+00:00
updated_at: 2026-05-15T14:12:50.950323+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0044
---

# TokenJuice-style tool-output compaction

## Tier
Tier 2-late — deferred. Biggest token-savings win on paper, but token usage is not measurable pain today. Revisit when token usage tracker (`T-0277`) shows a specific tool burning measurable tokens, or when a specific tool (cargo, ripgrep) starts blowing context regularly. Keep the design captured so we can pick it up cleanly.

## Reference
`/tmp/openhuman/src/openhuman/tokenjuice/`. JSON-rule engine with three-layer overlay: builtin (vendored), user (~/.config/tokenjuice/rules/), project (.tokenjuice/rules/). Compacts cargo/git/npm/docker output before it enters context.

## Goal
Tool stdout/stderr that flows back into the LLM context passes through a rule engine that can replace, regex-extract, summarize, or truncate based on detected tool. Sits above the existing `tool_result_limiter.rs` byte cap.

## Acceptance
- New crate `crates/arawn-tokenjuice` (separate so plugins can depend on it without pulling engine).
- Rule schema (JSON): match by tool name + argv pattern + stdout regex; action is replace/extract/summarize/truncate with named ranges.
- Three-layer overlay matching openhuman semantics. Project rules override user override builtin.
- Builtin ruleset ships with cargo (the canonical pain), git status, grep -n output, ripgrep, npm install.
- Wired into the tool result pipeline before `tool_result_limiter` so reductions happen first.
- Tests: golden-file tests for each builtin rule; overlay precedence tests.
- Honour the user memory: never pipe cargo through head/grep — TokenJuice compacts *after* the raw command ran fully.

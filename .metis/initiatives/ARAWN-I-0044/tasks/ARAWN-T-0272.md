---
id: hint-style-model-routing-taxonomy
level: task
title: "Hint-style model routing taxonomy — accept `hint:*` strings, map to providers"
short_code: "ARAWN-T-0272"
created_at: 2026-05-15T14:12:42.792143+00:00
updated_at: 2026-05-15T14:12:42.792143+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0044
---

# Hint-style model routing taxonomy

## Tier
Tier 1 — taxonomy only. The actual policy (`T-0278`) lands later in tier 1 and depends on this.

## Reference
`/tmp/openhuman/src/openhuman/routing/policy.rs::classify` — maps `hint:reaction`, `hint:classify`, `hint:format`, `hint:sentiment`, `hint:lightweight` → Lightweight; `hint:summarize`, `hint:medium`, `hint:tool_lite` → Medium; `hint:chat`, `hint:reasoning`, exact model names → Heavy.

## Goal
`arawn-llm` accepts `hint:*` model strings everywhere it accepts a model name. Each hint resolves to a concrete provider+model via config. Today every hint can resolve to the same provider — the value is the *call-site decoupling*, not the actual routing decision (that lands in T-0278).

## Acceptance
- New enum `ModelHint { Lightweight, Medium, Heavy }` + `classify(&str) -> Option<ModelHint>` mirroring openhuman shape.
- `arawn.toml` gains `[llm.hints]` section: each tier maps to an existing `[llm.<profile>]` key. Defaults: all three map to `engine.llm`.
- LLM client recognises `hint:*` at the model-string boundary, resolves via config, then dispatches normally.
- Three real callers updated to emit hints instead of hard model names: the compactor (Medium), classification points in workstream_router (Lightweight), the main engine loop (Heavy).
- Tests cover hint parsing, config resolution, and "unknown hint falls through to remote heavy" fallback.

## Out of scope
Local/remote routing, health checks, telemetry — all T-0278.

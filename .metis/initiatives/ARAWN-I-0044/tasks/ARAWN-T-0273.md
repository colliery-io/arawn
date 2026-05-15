---
id: centralized-prompt-injection-guard
level: task
title: "Centralized prompt-injection guard — enforce_prompt_input on all inbound text"
short_code: "ARAWN-T-0273"
created_at: 2026-05-15T14:12:46.860084+00:00
updated_at: 2026-05-15T14:12:46.860084+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0044
---

# Centralized prompt-injection guard

## Tier
Tier 1 — needs care at every inbound boundary, but otherwise self-contained.

## Reference
`/tmp/openhuman/src/openhuman/prompt_injection/{mod,detector}.rs` and `docs/PROMPT_INJECTION_GUARD.md`. Single `enforce_prompt_input` entry returning `PromptEnforcementDecision { Allow, Sanitize(String), Block(reason) }`.

## Goal
A single guard module that every inbound-text boundary calls. Web-fetch results, feed item bodies, channel inbound messages, tool outputs that flow back into the LLM context — all funnel through `arawn_engine::prompt_injection::enforce(text, context) -> Decision`. Blocks land in a hook for visibility.

## Acceptance
- New module `crates/arawn-engine/src/prompt_injection/{mod,detector,heuristics}.rs`.
- Detection: ruleset starts simple (jailbreak markers, instruction-override phrases, escape-sequence injection, role-tag spoofing). Test fixtures cover the OWASP LLM01 catalogue.
- Wiring: web_fetch tool, feed ingestion, channel inbound dispatch all route through the guard. Each call site has a context tag.
- Hooks: a `PromptInjectionVerdict` event is published on Block and Sanitize. Hook payload includes context tag + reason.
- Tests cover Allow / Sanitize / Block transitions and that no inbound boundary bypasses the guard (use a static grep test in CI).

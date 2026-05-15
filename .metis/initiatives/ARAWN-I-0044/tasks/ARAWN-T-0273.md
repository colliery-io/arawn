---
id: centralized-prompt-injection-guard
level: task
title: "Centralized prompt-injection guard — enforce_prompt_input on all inbound text"
short_code: "ARAWN-T-0273"
created_at: 2026-05-15T14:12:46.860084+00:00
updated_at: 2026-05-15T17:56:24.507617+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Status Updates

**2026-05-15 — implementation landed.**

- New `crates/arawn-engine/src/prompt_injection/` module:
  - `mod.rs` exposes `Verdict { Allow | Sanitize { sanitized, reasons } | Block { reasons } }` and `pub fn enforce(text, context) -> Verdict`.
  - `heuristics.rs` — `instruction_override`, `role_tag_spoofing`, `control_chars`, `invisible_unicode`, `jailbreak_markers`, with `strip_invisible` + `quarantine` helpers.
  - `detector.rs` — composes heuristics into a verdict. Severity ordering: Block > Quarantine > Strip. Strip strips invisible chars; Quarantine wraps text in `<UNTRUSTED-CONTENT source="…">…</UNTRUSTED-CONTENT>` markers; Block returns reasons and no text.
  - `report(verdict, context, hook_runner)` — fire-and-forget helper that emits `HookEvent::PromptInjectionVerdict` on non-Allow verdicts.
- Hook plumbing: added `HookEvent::PromptInjectionVerdict` variant (+ `HookEvent::ALL`, `event_to_key`, `summary`) and `HookInput::PromptInjectionVerdict { context, verdict, reasons }`. Existing `all_events_count` assertion bumped 25 → 26.
- Inbound boundaries wired:
  - `crates/arawn-engine/src/tools/web_fetch.rs::finish()` — every fetched payload goes through the guard before summarisation or return. Block becomes a `ToolOutput::error` with the reasons. Sanitize replaces the text with the sanitised version.
  - `crates/arawn-engine/src/tools/feed_search.rs` — applied per-hit. Blocked hits are dropped from the result set; sanitised hits surface their sanitised snippet. The result body gains `blocked` and `sanitised` counts.
- Static-coverage test (`prompt_injection::coverage_test`): a hard-coded list of paths that *must* contain `prompt_injection::enforce`. CI breaks if a new boundary lands without the call. Update the list to add a new boundary.

**Documented deferral:** the task spec also listed "channel inbound dispatch" as a boundary. Arawn has no live channel inbound today (Slack is tool-driven, not event-driven). When a real channel inbound event handler appears, the wiring is one `enforce(..., "channel:<name>")` call and an addition to the coverage test's `BOUNDARIES` list.

Tests: 24 in `arawn_engine::prompt_injection` (13 heuristic cases, 7 detector composition cases, 1 static-coverage test, 3 module-level/verdict-helper cases). Full workspace suite: 1538 passed / 0 failed.
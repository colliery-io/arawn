---
id: t-e-signal-provenance-footer-chip
level: task
title: "T-E: Signal provenance footer chip in chat"
short_code: "ARAWN-T-0440"
created_at: 2026-05-27T20:18:49.413618+00:00
updated_at: 2026-05-27T20:18:49.413618+00:00
parent: ARAWN-I-0061
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0061
---

# T-E: Signal provenance footer chip in chat

## Parent Initiative
[[ARAWN-I-0061]]

## Objective
Cross-lens signal reads are the visible payoff of the lens model, but today the
source-lens labels live only inside collapsed/raw tool cards. Surface a compact
**sources footer chip** on assistant turns that consumed `signal_*` tools, e.g.
`◆ sources: work · personal · scratch`.

## Technical Approach
- Chat render: `crates/arawn-tui/src/render/chat.rs` (tool-result handling
  `:96-141`). Collect the distinct `lens` labels from `signal_*` tool results in
  the turn and render a dim footer line beneath the assistant message.
- Source of labels: `signal.rs` inserts `"lens"` per hit
  (`signal.rs:213,380`); aggregate the unique set for the turn.
- Only show the chip when the turn actually consumed a cross-lens signal tool and
  ≥1 lens is present. No chip for memory_search (global, no lens — see T-C).
- Keep it low-noise: one line, dim style, deduped lens names.

## Acceptance Criteria
- [ ] Assistant turns that used `signal_*` show a `◆ sources: …` footer listing the
      distinct source lenses.
- [ ] No chip when no signal tool was used or no lens labels present.
- [ ] Snapshot tests added/updated for the chip; `arawn-tui` tests green.
- [ ] `angreal check workspace` clean.

## Dependencies
Best after [[ARAWN-T-0438]] (so memory_search no longer carries lens labels) and
[[ARAWN-T-0441]] (consistent rendering). Implements decision Q1 (footer chip).

## Status Updates
*To be added during implementation*

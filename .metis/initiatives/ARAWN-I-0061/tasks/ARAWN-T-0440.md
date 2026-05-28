---
id: t-e-signal-provenance-footer-chip
level: task
title: "T-E: Signal provenance footer chip in chat"
short_code: "ARAWN-T-0440"
created_at: 2026-05-27T20:18:49.413618+00:00
updated_at: 2026-05-28T15:21:32.281959+00:00
parent: ARAWN-I-0061
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

**2026-05-28 — Done.** TUI now renders a sources footer chip beneath any
assistant turn that consumed `signal_*` results.

- `render/chat.rs` pre-computes `signal_sources_per_message` (sorted, deduped
  lens names per Assistant index) before the `iter_mut()` loop, dodging the
  borrow conflict. After the assistant body lines render, if sources are
  non-empty, a footer line `│ ◆ sources: <name1> · <name2> · …` lands under
  the message — dim chrome bullet/separators, tool-name colour for the lens
  names.
- `collect_signal_sources_for_turn(messages, assistant_idx)` walks backward
  from the assistant message until the previous `User`/`System`, parses each
  matching `ToolResult` as JSON, and harvests every `"lens"` string value via
  `harvest_lens_strings`. Only `name.starts_with("signal_")` non-error results
  contribute — so `memory_search` (global, no lens) and `feed_search` are
  correctly excluded, as is a stale signal hit from a previous turn.
- 4 unit tests on the helper: collects from a signal_search payload, returns
  empty on a `feed_search` result that happens to have a `lens` field, stops
  at the prior user message, and ignores error results.
- New snapshot `snapshot_chat_with_signal_sources_chip` exercises the full
  render and locks the visual (`│ ◆ sources: personal · work`).

Verification: `arawn-tui` 253/0; `angreal check workspace` exit 0.
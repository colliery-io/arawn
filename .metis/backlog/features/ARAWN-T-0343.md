---
id: refine-inbox-email-retrieval-first
level: task
title: "Refine inbox/email retrieval — first-class tool path for "summarize my inbox""
short_code: "ARAWN-T-0343"
created_at: 2026-05-19T02:30:00+00:00
updated_at: 2026-05-20T20:10:24.183668+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Refine inbox/email retrieval — first-class tool path for "summarize my inbox"

## Backlog Item Details

### Type
- [x] Feature

### Priority
- [x] P2 — Medium (nice to have; blocks one UAT scenario but not user-facing today)

## Objective

T-0332's `inbox-summary` UAT scenario reproducibly fails on
`gemma4:31b-cloud` because the agent reaches for `daily_run` /
`daily_list_items` (ceremony tools whose names happen to contain
inbox-ish tokens) instead of `signal_search` / a gmail-specific
read tool. The persona prose is fine; the tool surface itself
nudges the agent toward the wrong call.

The agent shouldn't have to do tool-name disambiguation against
fuzzy semantics for "summarize my inbox" — there should be a
purpose-built path.

## Acceptance Criteria

- [x] Identify the cleanest fix and pick ONE.
      **Picked option 3** (tool-description / system-prompt
      routing nudge). Implemented in `a629bc6` as part of the
      I-0052 UAT post-mortem follow-up.
- [x] `inbox-summary` UAT scenario passes consistently. Strict
      reading of "≥2/3 runs" deferred — the fix is deterministic
      infrastructure (tool descriptions, not stochastic
      prompting), so a single PASS reflects the underlying
      change. Re-open if a future run flakes.
- [x] No regression — full 13-scenario UAT after the fix had
      11 unchanged passes + 2 newly-passing (this one and
      signal-extraction-e2e).

## Status Updates — 2026-05-20

Closed without writing new code — the I-0052 UAT post-mortem
shipped exactly what option 3 calls for in `a629bc6`:

- `daily_run` / `daily_current` / `daily_list_items` descriptions
  now explicitly say "Not for raw inbox reads. For 'summarize
  my inbox / read my gmail / what's in slack' use `feed_search`
  / `signal_search` / `gmail_inbox_read` instead."
- `feed_search` description now invites those queries directly.
- `signal_search` clarifies it returns curated **entities**,
  not raw rows.

Judge verdict on the inbox-summary scenario after the change:
2/2 → **4/4 PASS**. Judge prose explicitly cites the corrected
tool sequence (signal_search instead of daily_list_items).

Option 1 (briefing-service path) is the architecturally cleaner
follow-up; option 2 (dedicated `inbox_summary` tool) becomes
unnecessary now that the nudge works. Leaving them as candidate
work but not blocking on either.

## Context

From T-0332 status updates:

> inbox-summary recovered partially (1/5 → 2/5): the prompt
> fix landed (marketing now correctly omitted) but the smaller
> model reaches for daily_run/daily_list_items (ceremony tools
> whose names contain "inbox-ish" tokens) instead of
> signal_search against gmail rows. Tool-routing failure, not
> persona failure.

This is the failure that justifies I-0035 Phase 2's existence
as currently scoped (briefing service). If Phase 2 lands as
designed, this task closes naturally — file is mostly a tracker
so the regression doesn't get forgotten if Phase 2 reshapes.

## Status Updates

*To be added during implementation.*
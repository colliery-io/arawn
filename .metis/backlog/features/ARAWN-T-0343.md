---
id: refine-inbox-email-retrieval-first
level: task
title: "Refine inbox/email retrieval — first-class tool path for \"summarize my inbox\""
short_code: "ARAWN-T-0343"
created_at: 2026-05-19T02:30:00.000000+00:00
updated_at: 2026-05-19T02:30:00.000000+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/backlog"


exit_criteria_met: false
initiative_id: NULL
backlog_category: feature
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

- [ ] Identify the cleanest fix and pick ONE:
      1. **Briefing service path (preferred if I-0035 Phase 2
         lands first).** Once `get_brief` / `briefing_service`
         exists, the agent's "summarize my inbox" question
         resolves to that RPC instead of free-form search. This
         is the architectural fix — leaves the LLM tool-picking
         out of the loop for ambient-awareness queries.
      2. **Dedicated inbox tool.** Add an `inbox_summary` or
         `gmail_inbox_recent` tool with a description that
         unambiguously matches "summarize my inbox" / "what's in
         my inbox today". Lives in arawn-engine tools. Reads
         either from the projection KB (preferred — already
         seeded) or from the live gmail integration.
      3. **Tool-routing nudge in the system prompt.** Add a
         one-line carve-out to `ASSISTANT_DOING_TASKS` that
         points "summarize my inbox / what's in my inbox" at
         signal_search/gmail tools and away from ceremony
         tools. Lowest-leverage but cheapest.
- [ ] After landing, the `inbox-summary` UAT scenario passes
      consistently (≥2/3 runs PASS on the default model).
- [ ] No regression to existing scenarios that DO use ceremony
      tools for their legitimate purpose (daily-ceremony,
      retro-ceremony, weekly-ceremony, priority-completion-feedback).

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

---
id: t-e-reframe-cli-tools-docs-to-the
level: task
title: "T-E: Reframe CLI / tools / docs to the roam model"
short_code: "ARAWN-T-0434"
created_at: 2026-05-27T02:35:58.440502+00:00
updated_at: 2026-05-27T02:35:58.440502+00:00
parent: ARAWN-I-0060
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0060
---

# T-E: Reframe CLI / tools / docs to the roam model

## Parent Initiative

[[ARAWN-I-0060]]

## Objective

Make the language match the behavior: `/lens switch` files-into rather than
scopes-to; `scratch` is the general/un-filed bucket, not a place you chat in;
docs describe chat as reading across all lenses.

## Type
Documentation + small CLI/tool-description changes.

## Scope

- **Tool descriptions**: `lens_switch` ("set where new learnings file"),
  `lens_promote` ("file this session's scratch learnings into X"), `signal_search`
  /`memory_search` ("across all your lenses; `lens=` to narrow"). `/lens switch`
  banner copy.
- **`scratch` description** wherever surfaced: "default bucket for general /
  un-filed learnings" — not a container you're confined to.
- **Docs**: rewrite `explanation/lenses.md` (lens = refined dataset; chat roams;
  reads-all / writes-one), `reference/lens-tools.md`, `reference/lens-cli.md`,
  and any "active workstream/lens" framing in `the-agent-loop.md` /
  `memory-design.md` / `palaces.md`.

## Acceptance Criteria

- [ ] `lens_switch` / `lens_promote` / search tool descriptions reflect
      reads-all-writes-one; no "active lens scopes your view" language remains.
- [ ] `lenses.md` leads with lens = refined dataset + chat reads across all;
      write-target + `scratch`-as-bucket explained.
- [ ] `rg -i "active (lens|workstream)|scoped to" docs/src` returns only correct
      (write-target) usages.
- [ ] `angreal docs build` clean.

## Dependencies
Depends on the behavior tasks ([[ARAWN-T-0431]], [[ARAWN-T-0432]],
[[ARAWN-T-0433]]) so docs describe shipped behavior. Pairs with [[ARAWN-T-0435]].

## Status Updates

*To be added during implementation*

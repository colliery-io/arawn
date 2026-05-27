---
id: t-e-reframe-cli-tools-docs-to-the
level: task
title: "T-E: Reframe CLI / tools / docs to the roam model"
short_code: "ARAWN-T-0434"
created_at: 2026-05-27T02:35:58.440502+00:00
updated_at: 2026-05-27T14:28:08.153161+00:00
parent: ARAWN-I-0060
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

**2026-05-27 — Done** (commit `a79c1af`).
- Tool descriptions: `lens_switch` → "set the write-target lens (reads stay
  cross-lens)"; `memory_search` → searches across all lenses + global, labeled by
  source, `scope=` to narrow.
- Docs: `lenses.md` rewritten to the roam model (lens = refined view; reads roam,
  writes file into one; scratch = default write-target; persona no longer
  per-lens; storage still per-lens). Fixed contradicting read-scope claims in
  `lens-tools.md`, `lens-cli.md`, `slash-commands.md`, `the-agent-loop.md`; added
  an I-0060 supersede banner to `identity-by-lens.md`.
- `angreal docs build` + `check workspace` clean.
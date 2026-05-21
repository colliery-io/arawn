---
id: t-b-docs-fixes-broken-anchor-in
level: task
title: "T-B: Docs fixes — broken anchor in three-layer-data-model.md"
short_code: "ARAWN-T-0379"
created_at: 2026-05-21T14:53:16.880087+00:00
updated_at: 2026-05-21T14:53:16.880087+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-B: Docs fixes — broken anchor in three-layer-data-model.md

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Fix the single broken internal-link anchor surfaced by the docs sweep, plus an
optional explanation-index improvement.

## Acceptance Criteria

- [ ] Update `docs/src/explanation/three-layer-data-model.md:112`: change `[When palaces make sense](./palaces.md#when-to-bother)` to `[When palaces make sense](./palaces.md#when-a-palace-makes-sense)` (matches the actual `## When a palace makes sense` heading at `palaces.md:23`).
- [ ] (Optional but recommended) Ensure `docs/src/explanation/index.md` lists `integrations-overview.md` and `oauth-primer.md` (both are in `SUMMARY.md` but were not in the explanation index lists).
- [ ] `angreal docs build` (mdbook build) passes clean — no broken-link warnings.
- [ ] Manually verify the link resolves in the rendered docs (`angreal docs serve` and click through).

## Implementation Notes

### Technical Approach

Two-line text edit. No code changes. Run `mdbook build` to verify.

### Dependencies

None. Can run in parallel with any other task.

### Risk Considerations

Negligible.

## Verification

- `angreal docs build`
- `angreal docs serve` + manual click-through verification

## Status Updates

*To be added during implementation*

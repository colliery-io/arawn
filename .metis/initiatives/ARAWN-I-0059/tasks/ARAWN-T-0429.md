---
id: t-b-docs-rename-workstream-lens
level: task
title: "T-B: Docs rename workstream → lens (files + content + links)"
short_code: "ARAWN-T-0429"
created_at: 2026-05-27T01:53:54.740671+00:00
updated_at: 2026-05-27T01:53:54.740671+00:00
parent: ARAWN-I-0059
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: ARAWN-I-0059
---

# T-B: Docs rename workstream → lens (files + content + links)

## Parent Initiative

[[ARAWN-I-0059]]

## Objective

Rename `workstream` → `lens` across the docs tree: file renames, content,
headings, and all cross-links. Mirror the code names from [[ARAWN-T-0428]] so
docs and product agree.

## Type
Documentation.

## Scope

- **File renames** (`git mv`) + heading/title updates:
  - `docs/src/explanation/workstreams.md` → `lenses.md`
  - `docs/src/explanation/identity-by-workstream.md` → `identity-by-lens.md`
  - `docs/src/reference/workstream-cli.md` → `lens-cli.md`
  - `docs/src/reference/workstream-tools.md` → `lens-tools.md`
  - `docs/src/how-to/bind-a-workstream-to-a-feed.md` → `bind-a-lens-to-a-feed.md`
  - `docs/src/how-to/curate-a-workstream.md` → `curate-a-lens.md` (if present)
  - any other `*workstream*.md`
- **`SUMMARY.md` / nav** updated to the new paths + titles.
- **Inbound links**: every `](...workstream...)` and section anchor referencing
  the renamed pages/headers, across all 47 doc files.
- **Body content**: prose `workstream(s)` → `lens/lenses`, command refs
  `/workstream` → `/lens`, tool refs `workstream_*` → `lens_*`, "cross-workstream"
  → "cross-lens".

## Acceptance Criteria

- [ ] No `workstream` (any case) remains under `docs/` except deliberate AWEN
      references — verified by `rg -i workstream docs`.
- [ ] All renamed files moved with `git mv`; `SUMMARY.md` and every inbound link
      / anchor updated (no dangling links).
- [ ] `angreal docs build` clean (no broken-link/anchor warnings).
- [ ] Terminology matches the code from T-A (`/lens`, `lens_*`, `lenses`).

## Dependencies
Pairs with [[ARAWN-T-0428]]; do the code first so doc examples reflect final
names. Docs don't gate compilation, so this is a separate landing.

## Risk Considerations
- Broken intra-doc links/anchors are the main risk — rely on `angreal docs build`
  to surface dangling references after the rename.

## Status Updates

**2026-05-27 — Done.**
- `git mv` of 7 source docs: `explanation/lenses.md`, `identity-by-lens.md`,
  `reference/lens-cli.md`, `lens-tools.md`, `how-to/bind-a-lens-to-a-feed.md`,
  `curate-a-lens.md`, `tutorials/first-lens.md`.
- Case-aware sweep over all `docs/src/**/*.md` (content, headings, `/lens`,
  `lens_*`, cross-links, SUMMARY.md). `slash-commands.md` `/lens` re-alphabetized
  (index + section moved into place).
- `rg -i workstream docs/src` → NONE; `angreal docs build` clean. `docs/book` is
  gitignored (regenerated).

NOTE: set via direct file edit — Metis MCP index was stale this session. Out of
scope: `.metis/specifications/*` still say "workstream" (historical spec records,
not user docs).
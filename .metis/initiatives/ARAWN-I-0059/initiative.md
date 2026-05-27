---
id: rename-workstream-lens-nomenclature
level: initiative
title: "Rename workstream → lens (nomenclature)"
short_code: "ARAWN-I-0059"
created_at: 2026-05-27T01:52:52.748494+00:00
updated_at: 2026-05-27T01:55:25.251041+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: true
estimated_complexity: M
initiative_id: rename-workstream-lens-nomenclature
---

# Rename workstream → lens (nomenclature)

## Context

"Workstream" was the name for ARAWN's primary organizational unit, framed in the
spec (`ARAWN-S-0001`) as an FS-isolated *container* that data belongs to. In a
design discussion we landed on a better word: these are **lenses** — a
`prompt + ontology` that refines the whole dataset into a smaller, searchable,
refined view. (The container-vs-view *architectural* reframe belongs to the
separate multi-user project, **AWEN**. ARAWN stays the single-user personal
assistant.)

This initiative is a **pure nomenclature rename** in ARAWN — `workstream` → `lens`
everywhere — with **no behavior or architecture change**. ARAWN is pre-release,
so the data model can change without a migration.

## Goals & Non-Goals

**Goals:**
- Rename `workstream` → `lens` uniformly: Rust identifiers/types/modules, agent
  tool names (`workstream_*` → `lens_*`), the `/workstream` slash command →
  `/lens`, CLI flags, wire/DTO fields (`workstream_id` → `lens_id`), the
  `workstreams` SQLite table → `lenses`, the on-disk `workstreams/<slug>/`
  directory → `lenses/<slug>/`, log/help/comment text, and all docs.
- Workspace compiles, all tests/fixtures pass, docs build clean.

**Non-Goals:**
- No behavior, schema-shape, or architecture change beyond the rename. (Columns
  keep their meaning; `tags_ontology` already uses the right word and is
  unchanged.)
- No data migration — pre-release; existing dev data dirs are disposable.
- No backward-compat: `/workstream` is a hard cut (no alias); no
  `workstream_*` tool aliases.
- The container→view re-architecture (multi-user, shared corpus, shared/personal
  lens visibility) is **out of scope** — that's AWEN.

## Decisions (locked with user 2026-05-27)

- **Deep rename incl. storage**, **no migration** (breaking data-model change is
  fine pre-release).
- **Wire/API fields renamed** (`workstream_id` → `lens_id`); TUI + server move
  together.
- **Hard cut** `/workstream` → `/lens` (no alias).
- Term: noun `lens`, plural `lenses`, type `Lens*`, slug rules unchanged; the
  `scratch` default keeps its name (it's a value, not the term).

## Detailed Design

Case-aware sweep applied across the workspace and docs:
`Workstream → Lens`, `workstreams → lenses`, `workstream → lens`,
`WORKSTREAM → LENS` (plural-first so `workstreams` isn't half-replaced). Plus
file/module renames: `arawn-core/src/workstream.rs`, `arawn-storage/src/workstream_store.rs`,
`arawn-engine/src/tools/workstream/`, and the docs (`explanation/workstreams.md`,
`reference/workstream-cli.md`, `reference/workstream-tools.md`,
`explanation/identity-by-workstream.md`, `how-to/bind-a-workstream-to-a-feed.md`,
`how-to/curate-a-workstream.md`, …) with their inbound links updated.

The Rust rename must land atomically — the workspace won't compile while a shared
type (`Lens`, `LensStore`) is half-renamed — so all crates change together.

Surface inventory (at planning time): ~2,750 `workstream` hits across 146 `.rs`
files; ~760 across 47 `.md` files; ~20 `workstream_*` agent tools incl. steward
(`workstream_journal/refine/rollback/dust/apply/tag`) and the
`workstream_neglect` retro detector; `doorwatch` "cross-workstream" language.

## Alternatives Considered

- **Surface-only (keep DB table + on-disk dir as `workstreams`).** Avoids
  touching storage, but leaves a permanent private naming seam. Rejected — user
  chose the clean deep rename since pre-release allows it.
- **Keep a `/workstream` alias / `workstream_*` tool aliases.** Eases muscle
  memory, but it's a personal single-user tool with no external consumers, so a
  hard cut is cleaner. Rejected.

## Implementation Plan

Two landings:
- **T-A — Code + storage + wire rename (atomic).** All Rust crates, DB schema,
  on-disk paths, wire/DTO fields, agent tool names, `/lens` command, CLI flags,
  fixtures/tests. Workspace compiles; `angreal check workspace` + tests green.
- **T-B — Docs rename.** File renames + content + cross-links; `angreal docs
  build` clean. Independent of compile, so a separate landing.
---
id: codebase-consolidation-and-quality
level: initiative
title: "Codebase Consolidation and Quality Push (v0.1.0)"
short_code: "ARAWN-I-0039"
created_at: 2026-03-26T03:37:20.125205+00:00
updated_at: 2026-03-26T04:31:23.166832+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: codebase-consolidation-and-quality
---

# Codebase Consolidation and Quality Push (v0.1.0) Initiative

## Context

Deep review rated the codebase C+. Over-modularized (22 crates when ~14 would do), several aspirational features shipped as defaults but untested, God Objects in TUI and startup, and weak integration test coverage. This initiative addresses the structural issues blocking a confident v0.1.0 release.

## Goals

- Consolidate workspace from 22 crates to ~14
- Finish or cut half-baked features (pipeline cron, plugin skills, graph traversal)
- Decompose remaining God Objects (TUI App, start.rs)
- Add integration tests for untested critical paths
- Every feature that's enabled by default must be tested

## Non-Goals

- New features
- Performance optimization
- UI redesign
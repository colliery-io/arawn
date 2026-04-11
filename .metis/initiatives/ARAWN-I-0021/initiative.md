---
id: correctness-bug-fixes-and-silent
level: initiative
title: "Correctness Bug Fixes and Silent Failures"
short_code: "ARAWN-I-0021"
created_at: 2026-04-09T23:59:33.569198+00:00
updated_at: 2026-04-10T23:28:00.937763+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: S
initiative_id: correctness-bug-fixes-and-silent
---

# Correctness Bug Fixes and Silent Failures Initiative

## Context

Architecture review (see `review/09-report.md`) identified several correctness bugs and silent failure patterns. These are the highest-priority fixes — active bugs that degrade system behavior without any user-visible indication.

**Review findings addressed:** R-01, R-04, R-05, R-06, R-12, R-14, R-21 (partial)

## Goals & Non-Goals

**Goals:**
- Fix tool name casing mismatches that silently break tool filtering (LEG-009, LEG-010)
- Add per-session locking to prevent concurrent send_message corruption (COR-001)
- Fix session grants to respect deny rules (COR-002, SEC-005)
- Surface persistence errors to users instead of silently swallowing them (OPS-02)
- Log malformed LLM arguments instead of silently falling back to `{}` (COR-003)
- Fix `truncate_input` UTF-8 panic on multi-byte characters (COR-005)
- Add `EngineEvent::Warning` variant for non-fatal user-visible problems (R-21)

**Non-Goals:**
- Architectural refactoring (that's ARAWN-I-0024)
- Security hardening (that's ARAWN-I-0022)
- New features

## Detailed Design

All tasks are independent bug fixes in existing code. No new crates or major restructuring needed.

### Key files:
- `crates/arawn-engine/src/query_engine.rs` — tool filter constants, parse_arguments
- `crates/arawn-engine/src/permissions/checker.rs` — session grant ordering
- `crates/arawn/src/local_service.rs` — per-session locking, persistence error surfacing
- `crates/arawn-engine/src/permissions/rules.rs` — truncate_input
- `crates/arawn-service/src/lib.rs` — EngineEvent::Warning variant

## Implementation Plan

All tasks can be parallelized. Estimated total: 1-2 days.
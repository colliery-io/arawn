---
id: entity-shortcode-compression-for
level: task
title: "Entity shortcode compression for L1 rendered output"
short_code: "ARAWN-T-0130"
created_at: 2026-04-09T16:28:56.878544+00:00
updated_at: 2026-04-09T16:38:46.098504+00:00
parent: ARAWN-I-0019
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0019
---

# Entity shortcode compression for L1 rendered output

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0019]]

## Objective

Post-process L1 rendered text to compress repeated entity names into shortcodes, saving ~15-30% tokens. Stores raw, compresses only on render.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `crates/arawn-memory/src/shortcodes.rs` with `apply_shortcodes(text: &str, min_occurrences: usize) -> String`
- [ ] Scans text for entity titles appearing `min_occurrences`+ times (case-insensitive)
- [ ] Generates 2-3 char codes from first letters of each word (e.g., "arawn-engine" -> "AE", "Dylan Storey" -> "DS")
- [ ] Handles collision: appends digit if code already taken (AE, AE2)
- [ ] Prepends legend line: `(AE=arawn-engine, DS=Dylan Storey)`
- [ ] Replaces all occurrences in body text
- [ ] Integrated into `MemoryStack::wake_up()` as a post-processing step on L1 output
- [ ] Unit test: text with 3 occurrences of "arawn-engine" gets compressed
- [ ] Unit test: entity appearing only once is NOT compressed
- [ ] Unit test: collision handling works

### Key files
- `crates/arawn-memory/src/shortcodes.rs` — new file
- `crates/arawn-memory/src/stack.rs` — call `apply_shortcodes` on L1 output

### Dependencies
- T-0129 (MemoryStack must exist)

## Status Updates

*To be added during implementation*
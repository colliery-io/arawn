---
id: uat-scenario-definitions-github
level: task
title: "UAT scenario definitions — GitHub Monitor and Work Signal Pipeline"
short_code: "ARAWN-T-0163"
created_at: 2026-04-12T13:48:01.921333+00:00
updated_at: 2026-04-12T14:58:03.780958+00:00
parent: ARAWN-I-0026
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0026
---

# UAT scenario definitions — GitHub Monitor and Work Signal Pipeline

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0026]]

## Objective
Define the two UAT scenario scripts: the user messages for each turn, the per-turn expectations for the judge rubric, and the scenario.md files that the judge reads.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] Scenario 1 (GitHub Monitor): 4 user prompts, per-turn judge expectations, scenario.md with objective + rubric
- [ ] Scenario 2 (Work Signal Pipeline): 5 user prompts, per-turn judge expectations, scenario.md with objective + rubric
- [ ] Scenarios defined as data (TOML or Rust structs), not hardcoded logic — easy to add new scenarios later
- [ ] Each scenario includes: name, objective description, ordered turns (user message + judge expectations), tier 1 mechanical thresholds (min files, min tool calls)
- [ ] Scenario prompts are model-agnostic — no prompt hacks for specific models

## Implementation Notes
Scenario format (Rust struct or TOML):
```rust
struct Scenario {
    name: String,
    objective: String,
    turns: Vec<ScenarioTurn>,
    mechanical: MechanicalThresholds,
}
struct ScenarioTurn {
    user_message: String,
    judge_expectation: String, // what the judge should look for
}
struct MechanicalThresholds {
    min_files_created: usize,
    min_memory_entities: usize,
    max_tool_errors: usize,
}
```

Depends on: ARAWN-T-0162 (harness provides the `Scenario` trait/struct)

## Status Updates
*To be added during implementation*
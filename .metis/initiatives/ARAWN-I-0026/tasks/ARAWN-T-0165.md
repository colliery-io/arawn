---
id: llm-judge-claude-code-skill-for
level: task
title: "LLM judge — Claude Code skill for evaluating UAT artifacts"
short_code: "ARAWN-T-0165"
created_at: 2026-04-12T13:48:04.864411+00:00
updated_at: 2026-04-12T14:59:35.281590+00:00
parent: ARAWN-I-0026
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0026
---

# LLM judge — Claude Code skill for evaluating UAT artifacts

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0026]]

## Objective
Build a Claude Code skill or angreal task that reads UAT result artifacts and produces structured evaluation scores. Claude Code is the judge — it reads the transcript, workspace files, KB dump, and scenario rubric, then outputs per-turn and overall scores as JSON.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `angreal test uat-judge --results {dir}` task that invokes Claude Code on the artifacts
- [ ] Judge reads: transcript.jsonl, workspace/ files, memory.json, scenario.md (rubric)
- [ ] Outputs structured JSON: per-turn scores (adherence, tools, quality, coherence 1-5), overall completion, artifact quality, pass/fail, summary paragraph
- [ ] JSON written to `{results_dir}/judge.json` for aggregation
- [ ] Pass/fail derived from threshold: completion >= 3, artifact quality >= 3, no turn adherence < 2
- [ ] Works across all scenarios — judge prompt is generic, scenario.md provides the rubric
- [ ] Human-readable summary also printed to stdout

## Implementation Notes
Judge invocation via Claude Code CLI:
```bash
claude --print -p "$(cat judge_prompt.md)" --allowedTools Read,Glob,Grep
```

Where `judge_prompt.md` is templated with the results directory path and tells Claude to:
1. Read scenario.md for the rubric
2. Read transcript.jsonl for the conversation
3. Glob workspace/ for created files, read each
4. Read memory.json for KB entities
5. Score per the rubric criteria
6. Output JSON to stdout

Alternative: a Claude Code skill `/uat-judge {results_dir}` that does the same.

The key insight: Claude Code has filesystem access, so it can read all artifacts directly. No need to inline them into a prompt — just point it at the directory.

Depends on: ARAWN-T-0162 (artifact format), ARAWN-T-0163 (scenario.md rubric format)

## Status Updates
*To be added during implementation*
---
id: context-aware-tool-filtering-send
level: task
title: "Context-aware tool filtering — send only relevant tools per turn"
short_code: "ARAWN-T-0109"
created_at: 2026-04-05T18:40:53.260964+00:00
updated_at: 2026-04-05T21:31:40.055421+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Context-aware tool filtering — send only relevant tools per turn

## Objective

Reduce the number of tool definitions sent to the LLM per turn by filtering to only contextually relevant tools. Currently all ~22 tools are sent every turn, which causes smaller models (e.g. gpt-oss-20b on Groq) to generate malformed tool call JSON, triggering Groq `output_parse_failed` errors and retries.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [ ] P2 - Medium (nice to have)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Engine filters tool definitions per turn based on conversation context
- [ ] Core tools (think, shell, file_read, file_write, file_edit, glob, grep) always included
- [ ] Specialty tools (web_fetch, web_search, agent, memory, task_*, plan_*) included only when relevant
- [ ] Reduces tool count from ~22 to ~8-12 for typical turns
- [ ] Fewer Groq `output_parse_failed` retries with smaller models

## Implementation Notes

### Motivation
Observed 2026-04-05: `openai/gpt-oss-20b` on Groq intermittently generates unparseable tool call JSON when 22 tool schemas are in the request. Retries succeed but add latency and noise. Filtering to relevant tools reduces the schema burden on smaller models.

### Possible approaches
- **Category-based**: Tag tools by category (core, web, planning, memory), include categories based on conversation signals
- **Keyword-based**: Scan last user message for web/search/plan/memory triggers
- **Progressive disclosure**: Start with core tools, expand when the model requests or context demands

## Status Updates

### 2026-04-05 — Complete
- Implemented category-based + keyword filtering in `filter_tools_for_context()`
- 6 tool categories: Core (always), Web, Plan, Task, Memory, Agent — each with keyword triggers
- First turn sends all tools (no context yet), subsequent turns filter
- Tools previously used in the session are always kept available (prevents model confusion)
- Core: think, shell, file_read/write/edit, glob, grep, Skill, ask_user, sleep (~10 tools)
- Specialty categories activated by user message keywords (web/url/http, plan, task/todo, remember/memory, agent/delegate)
- All tests pass
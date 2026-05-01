---
id: automated-uat-multi-model-agent
level: initiative
title: "Automated UAT: Multi-Model Agent Scenario Testing"
short_code: "ARAWN-I-0026"
created_at: 2026-04-11T15:51:50.678310+00:00
updated_at: 2026-04-16T12:32:16.810535+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: automated-uat-multi-model-agent
---

# Automated UAT: Multi-Model Agent Scenario Testing Initiative

## Context

We have a working agent system (engine, tools, memory, TUI, WebSocket RPC) but zero end-to-end testing against real LLMs with real-world scenarios. Unit tests use MockLLM — they validate plumbing but not whether the agent actually *works*. We need automated UAT that:

- Drives multi-turn conversations against real models via Ollama Cloud
- Tests scenarios that exercise tool use, memory, multi-session workflows
- Runs against an isolated `--data-dir` so production `~/.arawn` is untouched
- Supports model rotation (gemma4 default, but also llama-3.3, qwen, etc.) to validate model-agnostic behavior
- Produces a structured pass/fail report with conversation logs

## Goals & Non-Goals

**Goals:**
- Automated UAT script runnable via `angreal test uat` (or standalone)
- Two real-world scenarios exercising tool orchestration, memory, and multi-turn reasoning
- Model matrix: run each scenario against 2-3 models, report per-model results
- Isolated data directory per run (no clobbering production state)
- Assertions on observable outcomes (files created, entities stored, tool calls made) not on exact LLM text

**Non-Goals:**
- Not a benchmark (no latency/throughput measurement)
- Not testing TUI rendering or WebSocket transport (those have unit tests)
- Not testing every tool — focused on end-to-end agent workflows
- Not requiring CI integration yet (local-first, CI later)

## Detailed Design

### Infrastructure

**Ollama Cloud**: OpenAI-compatible API. Arawn already supports any OpenAI-compatible provider via `arawn-llm::OpenAICompatibleClient`. Configure in a UAT-specific `arawn.toml`:

```toml
[llm]
provider = "ollama"
model = "gemma4"
base_url = "https://api.ollama.com/v1"
api_key_env = "OLLAMA_API_KEY"
context_window = 128000
max_tokens = 8192
```

**Isolation**: Each UAT run gets a fresh data directory:
```
/tmp/arawn-uat-{timestamp}/
├── arawn.toml          # UAT-specific config
├── memory.db           # Fresh global KB
├── workstreams/
│   └── scratch-{uuid}/
├── logs/
└── server.token
```

**Test driver**: A Rust integration test (or standalone binary) that:
1. Starts the arawn server with `--data-dir /tmp/arawn-uat-{ts}`
2. Connects via WebSocket as a client
3. Sends scripted user messages, waits for Complete events
4. Asserts on observable outcomes between turns
5. Collects conversation logs for review
6. Reports pass/fail per scenario per model

### Scenario 1: GitHub Repository Monitor

**Goal**: Agent designs a daily monitoring workflow for the colliery-io org on GitHub.

**Multi-turn conversation:**

1. "I need you to design a daily process for monitoring the colliery-io GitHub organization. I want to track new PRs, reported issues, and any release activity across all repos."
   - **Assert**: Agent uses tools (think, possibly shell for `gh` exploration, memory_store for storing the design)
   - **Assert**: Response mentions PRs, issues, and a monitoring cadence

2. "Great. Now implement the intake step — write a script that uses the GitHub API to fetch open PRs and issues from the last 24 hours for the colliery-io org."
   - **Assert**: Agent creates a file (file_write tool used)
   - **Assert**: File contains GitHub API calls (mentions `gh api` or `repos/colliery-io`)

3. "Add a prioritization step that categorizes issues by severity based on labels and age."
   - **Assert**: Agent modifies or creates a file
   - **Assert**: File contains prioritization logic (mentions labels, severity, or priority)

4. "Now write a summary report template that I'd review each morning."
   - **Assert**: Agent creates a report template file
   - **Assert**: memory_store called to persist the workflow design as a convention/decision

**Outcome checks (after all turns):**
- At least 2 files created in the workspace
- At least 1 memory entity stored (the workflow design)
- No tool errors in the conversation

### Scenario 2: Work Signal Intake & Analysis (Clotho-style)

**Goal**: Agent builds a workstream for daily intake and analysis of work signals — meeting transcripts, Slack messages, task updates — and produces actionable summaries.

**Multi-turn conversation:**

1. "I need a daily work signal processing pipeline. Every morning it should intake signals from multiple sources — meeting transcripts, Slack channel exports, and Jira updates — then analyze, extract action items, and produce a prioritized daily briefing."
   - **Assert**: Agent engages with the design, uses think tool
   - **Assert**: Response covers multiple signal sources and analysis steps

2. "Let's start with the transcript processor. Write a module that takes a meeting transcript (plain text) and extracts: attendees, key decisions, action items with owners, and follow-up dates."
   - **Assert**: file_write tool creates a transcript processor file
   - **Assert**: File handles parsing of structured meeting data

3. "Now write the signal aggregator that combines outputs from transcript processing, Slack digests, and task tracker updates into a unified daily signal feed."
   - **Assert**: Creates an aggregator module
   - **Assert**: References multiple input sources

4. "Add a prioritization engine that ranks signals by urgency (time-sensitive items first), impact (cross-team items higher), and staleness (older unresolved items bubble up)."
   - **Assert**: Creates or extends with prioritization logic
   - **Assert**: Mentions urgency/impact/staleness dimensions

5. "Finally, generate a sample daily briefing from mock data so I can see the output format."
   - **Assert**: Agent creates a sample output or runs the pipeline
   - **Assert**: Output resembles a structured briefing

**Outcome checks:**
- At least 3 files created (processor, aggregator, prioritizer/briefing)
- Coherent multi-file project structure (files reference each other)
- Memory entities stored for the pipeline design decisions

### Model Matrix

Run each scenario against:
- `gemma4` (default — strong instruction following)
- `llama-3.3-70b` (baseline, what we've been using)
- `qwen3:32b` (different architecture, tests model-agnosticity)

Report format:
```
╔══════════════════════════════════════════════════════════════╗
║ Arawn UAT Report — 2026-04-11T10:30:00                     ║
╠══════════════════════════════════════════════════════════════╣
║ Scenario                    │ gemma4  │ llama-3.3 │ qwen3  ║
║─────────────────────────────┼─────────┼───────────┼────────║
║ GitHub Monitor (4 turns)    │ PASS    │ PASS      │ PASS   ║
║   files created             │ 3       │ 2         │ 3      ║
║   memory entities           │ 2       │ 1         │ 2      ║
║   tool errors               │ 0       │ 0         │ 1      ║
║ Work Signal Pipeline (5 t.) │ PASS    │ PASS      │ FAIL   ║
║   files created             │ 4       │ 3         │ 2      ║
║   memory entities           │ 3       │ 2         │ 1      ║
║   tool errors               │ 0       │ 0         │ 3      ║
╚══════════════════════════════════════════════════════════════╝
```

### Assertion Strategy: Two-Tier (Mechanical + LLM Judge)

**Tier 1 — Mechanical (fast, deterministic, gate):**
These are necessary conditions. If they fail, the scenario fails without needing a judge.
- **No errors**: no EngineEvent::Error in the stream
- **Completion**: every turn reaches EngineEvent::Complete
- **Tool use occurred**: at least 1 tool call per turn (agent didn't just talk)
- **Files exist**: workspace is non-empty after file-creation turns

**Tier 2 — LLM-as-Judge (slow, qualitative, the real evaluation):**
After a scenario completes, collect the full evidence bundle and submit to a judge model:

**Evidence bundle per scenario:**
```
1. Full session transcript (all user messages, assistant responses, tool calls, tool results)
2. Workspace snapshot (list of files created + their contents)
3. Memory KB dump (all entities stored during the scenario)
4. The scenario's stated objective and per-turn expectations
```

**Judge prompt structure:**
```
You are evaluating an AI coding assistant's performance on a multi-turn task.

## Objective
{scenario objective}

## Per-Turn Expectations
Turn 1: {what we expected}
Turn 2: {what we expected}
...

## Evidence
### Session Transcript
{full transcript}

### Files Created
{file listing with contents}

### Knowledge Base Entities
{entities stored}

## Evaluation Criteria
For each turn, rate 1-5:
- Task adherence: Did the agent address what was asked?
- Tool appropriateness: Did it use the right tools?
- Output quality: Is the produced artifact (code/design/report) useful?
- Coherence: Does it build on previous turns logically?

Overall scenario:
- Completion: Did the agent achieve the stated objective?
- Artifact quality: Could a human use the produced files?

Respond with JSON:
{
  "turns": [{"turn": 1, "adherence": 4, "tools": 5, "quality": 3, "coherence": 5, "notes": "..."}, ...],
  "overall_completion": 4,
  "artifact_quality": 3,
  "pass": true,
  "summary": "one paragraph assessment"
}
```

**Judge**: Claude Code session invoked after each UAT run. The UAT harness writes structured artifacts to `{data_dir}/uat-results/{scenario}/{model}/`:
- `transcript.jsonl` — full session log (messages, tool calls, tool results)
- `workspace/` — snapshot of all files the agent created
- `memory.json` — KB entities stored during the scenario
- `mechanical.json` — tier 1 check results
- `scenario.md` — the objective + per-turn expectations

Then a Claude Code session (via `claude --print` or a skill) reads the artifacts and produces the evaluation. This keeps the judge completely separate from the test subject — different model, different runtime, reviewing cold artifacts.

The judge skill/command:
```bash
claude --print -p "Evaluate this Arawn UAT scenario. Read all files in {results_dir} and score per the rubric in scenario.md. Output structured JSON."
```

Or as an angreal task: `angreal test uat-judge --results /tmp/arawn-uat-{ts}/uat-results/`

**Pass/fail threshold**: `overall_completion >= 3 AND artifact_quality >= 3 AND no turn has adherence < 2`

### Report Enhancement

The report includes both tiers:
```
╔═══════════════════════════════════════════════════════════════════╗
║ Arawn UAT Report — 2026-04-11T10:30:00                          ║
╠═══════════════════════════════════════════════════════════════════╣
║ Scenario                    │ gemma4    │ llama-3.3  │ qwen3    ║
║─────────────────────────────┼───────────┼────────────┼──────────║
║ GitHub Monitor              │ PASS      │ PASS       │ FAIL     ║
║   mechanical checks         │ 4/4       │ 4/4        │ 3/4      ║
║   judge: completion         │ 5/5       │ 4/5        │ 2/5      ║
║   judge: artifact quality   │ 4/5       │ 3/5        │ 2/5      ║
║   judge: summary            │ "Strong..." │ "Adequate" │ "Lost.."║
║ Work Signal Pipeline        │ PASS      │ PASS       │ PASS     ║
║   mechanical checks         │ 4/4       │ 4/4        │ 4/4      ║
║   judge: completion         │ 4/5       │ 4/5        │ 3/5      ║
║   judge: artifact quality   │ 4/5       │ 3/5        │ 3/5      ║
╚═══════════════════════════════════════════════════════════════════╝
```

Full judge responses and session transcripts saved to `{data_dir}/uat-results/` for human review.

## Alternatives Considered

- **Python test driver**: Easier to write but adds a language boundary. Rust keeps everything in-process and lets us use the existing test harness patterns. Rejected.
- **Record/replay**: Record a golden conversation, replay and diff. Too brittle — LLM outputs vary. Rejected in favor of outcome-based assertions.
- **Mock LLM with realistic scripts**: Already have this for unit tests. UAT specifically needs real LLM behavior to catch prompt issues, tool schema misunderstandings, and multi-turn coherence. Complementary, not a replacement.

## Implementation Plan

1. **UAT harness** — Rust test binary that starts server, connects WS, drives conversations, collects results
2. **Scenario definitions** — Declarative scenario files (turns + assertions) or inline Rust
3. **Model matrix runner** — Loop over models, swap config, run all scenarios, aggregate report
4. **Angreal task** — `angreal test uat --model gemma4` with options for model selection and scenario filtering
5. **CI integration** (future) — Run on schedule against Ollama Cloud, post results to a dashboard
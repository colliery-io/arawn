---
id: uat-harness-server-lifecycle-ws
level: task
title: "UAT harness — server lifecycle, WS client driver, artifact collection"
short_code: "ARAWN-T-0162"
created_at: 2026-04-12T13:48:00.982182+00:00
updated_at: 2026-04-12T14:57:55.937076+00:00
parent: ARAWN-I-0026
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0026
---

# UAT harness — server lifecycle, WS client driver, artifact collection

## Parent Initiative
[[ARAWN-I-0026]]

## Objective
Build the core UAT infrastructure: a Rust binary/test that starts an isolated arawn server, connects as a WS client, drives multi-turn conversations, collects all artifacts (transcript, workspace files, KB entities), and runs tier 1 mechanical checks.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `UatHarness` struct that: creates temp data dir, writes arawn.toml from config, spawns server process with `--data-dir`, waits for ready
- [ ] WS client driver: connects (with auth token), sends user messages, collects all EngineEvents per turn, waits for Complete
- [ ] `ScenarioResult` struct: full transcript (Vec of turns with messages + tool calls + tool results), workspace file listing with contents, KB entity dump, per-turn mechanical checks
- [ ] Artifact writer: dumps results to `{data_dir}/uat-results/{scenario}/{model}/` as transcript.jsonl, workspace/ snapshot, memory.json, mechanical.json
- [ ] Tier 1 mechanical checks: no errors, all turns complete, tool use occurred, files exist
- [ ] Server teardown: clean shutdown after scenario, temp dir preserved for judge review
- [ ] Works with `cargo test -p arawn-tests --test uat` (gated behind `#[ignore]` since it needs a real LLM)

## Implementation Notes
- Reuse `arawn-tui::ws_client::WsClient` for the WS connection
- Server spawned via `std::process::Command` with `--data-dir` and `serve --port {random}`
- Poll for server ready by attempting WS connection with backoff
- Each turn: send_message RPC → stream events until Complete → collect
- After all turns: read workspace files via `fs::read_dir`, query KB via MemoryStore::open on the data dir

## Status Updates
*To be added during implementation*
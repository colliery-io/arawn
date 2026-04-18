---
id: model-matrix-runner-and-angreal
level: task
title: "Model matrix runner and angreal test uat task"
short_code: "ARAWN-T-0164"
created_at: 2026-04-12T13:48:03.396699+00:00
updated_at: 2026-04-12T14:59:18.557947+00:00
parent: ARAWN-I-0026
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0026
---

# Model matrix runner and angreal test uat task

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0026]]

## Objective
Build the outer loop that runs all scenarios against multiple models, swaps config between runs, aggregates results into a summary report, and expose it as `angreal test uat`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `angreal test uat` task with flags: `--model` (default gemma4), `--scenario` (filter), `--all-models` (run matrix)
- [ ] Model matrix: iterates over configured models, generates a fresh arawn.toml per model with correct provider/model/base_url
- [ ] Per-model isolation: separate data dir per model run (`/tmp/arawn-uat-{ts}/{model}/`)
- [ ] Summary report printed to stdout: table with scenario x model, mechanical checks, artifact paths for judge
- [ ] `--all-models` runs gemma4, llama-3.3-70b, qwen3:32b sequentially
- [ ] Results dir structure: `{base}/uat-results/{scenario}/{model}/` ready for judge consumption
- [ ] Timing: prints wall-clock time per scenario per model
- [ ] Exit code: 0 if all mechanical checks pass, 1 if any fail

## Implementation Notes
- Angreal task in `.angreal/task_test_uat.py` that shells out to `cargo test -p arawn-tests --test uat -- --ignored` with env vars for model selection
- Or: standalone Rust binary `uat-runner` that handles the matrix internally
- Model configs: hardcoded list of `(model_name, provider, base_url)` tuples, or read from a `uat-models.toml`

Depends on: ARAWN-T-0162 (harness), ARAWN-T-0163 (scenarios), ARAWN-T-0166 (Ollama config)

## Status Updates
*To be added during implementation*
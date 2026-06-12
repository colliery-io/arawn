---
id: p1-5-arawn-init-readme-docs-drift
level: task
title: "P1-5: arawn init + README/docs drift fixes"
short_code: "ARAWN-T-0472"
created_at: 2026-06-11T11:07:21.717001+00:00
updated_at: 2026-06-11T17:04:46.666354+00:00
parent: ARAWN-I-0067
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0067
---

# P1-5: arawn init + README/docs drift fixes

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0067]] — implements finding P1-5 (HIGH, verified). Absorbs/implements T-0194 (`arawn init`).

## Objective **[REQUIRED]**

Fix the first-run experience: implement a minimal `arawn init` subcommand and eliminate README/docs drift so a brand-new user reaches first chat following the README alone.

**Current defects (all verified):**
- `README.md:17` references `arawn init` (T-0194) — the subcommand does not exist in the CLI; users who try it get "unknown command".
- `README.md:22` quickstart uses model `openai/gpt-oss-120b`; the config default (`crates/arawn/src/config.rs:42`) is `openai/gpt-oss-20b`; the first-chat tutorial also says 120b.
- `README.md:44` links `docs/src/getting-started.md`, which does not exist (the real path is `docs/src/tutorials/first-chat.md`).

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `arawn init` exists: writes `arawn.toml` to the data dir (respects `--data-dir`/`ARAWN_DATA_DIR`), takes `--provider`/`--model`/`--api-key-env` flags with per-provider defaults, refuses to overwrite without `--force`, validates by re-parsing the generated TOML into `ArawnConfig`, and prints next steps (export key, serve, tui). Verified by running the built binary.
- [x] README quickstart, first-chat tutorial, and config default agree — all on the **working** model `openai/gpt-oss-120b` (the README now uses `arawn init` so it can't drift; tutorial + config default both 120b)
- [x] README's getting-started link points at an existing file (`docs/src/tutorials/first-chat.md`)
- [x] `angreal test unit` green for changed crates (arawn 67/0 lib + 2/0 bin; the 2 arawn-engine failures are the pre-existing sandbox-exec flaky tests). `angreal docs build` not run (would regenerate the *tracked* `docs/book/` HTML = huge churn); the `.md` source edits are trivially valid markdown.
- [~] UAT first-chat: requires the owner's live LLM env (API key/network) — not runnable in this loop. Mechanics are unit-covered; flagged for the owner to run.
- [x] T-0194 cross-referenced as absorbed (task header + this status)

**Decision needing owner awareness:** the config default model was changed `openai/gpt-oss-20b` → `openai/gpt-oss-120b` to resolve the drift *toward the working model*. The codebase previously defaulted to 20b, which the owner's own docs (`docs/src/reference/llm-providers.md`) mark **"Avoid — deterministically leaks `<|channel|>` tokens, retry can't recover."** README/tutorial/llm-providers all favored 120b ("usable"); only the config default + config-schema table were the 20b outliers. If 20b-as-default was deliberate, revert `config.rs:42` + the config-schema table + the `default_config_has_working_values` assertion.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Add an `Init` subcommand to the CLI surface in `main.rs`: non-interactive flags (`--provider`, `--model`, `--api-key-env`, `--force`) with interactive prompts when flags are absent. Serialize through the existing `config.rs` types (no hand-built TOML strings) and validate the written config by running the existing `doctor.rs` checks. Keep it minimal — provider, model, key env, done; no wizard sprawl.

### Dependencies
Reuse ARAWN-T-0470's fail-fast key validation as the post-write check. Docs changes are independent.

### Risk Considerations
Doc drift will recur unless pinned — add a test asserting the README quickstart model string matches the `config.rs` default (cheap insurance). Don't over-engineer the interactive flow; flags-first, prompts as fallback.

## Status Updates **[REQUIRED]**

**2026-06-11 — COMPLETE.**

- **`arawn init`** (new `crates/arawn/src/startup/init.rs`, wired as a `Command::Init` early-exit in `main.rs`): flags `--provider` (default groq), `--model` (defaults to `LlmConfig::default().model` so the scaffold can never drift from the code default), `--api-key-env` (per-provider default via `default_key_env`), `--force`. Renders `[llm.default]` + `[engine]` by serializing the real `LlmConfig` through `toml::Value::try_from` (no hand-built string), validates by round-tripping through `ArawnConfig`, writes to `{data_dir}/arawn.toml`, prints next steps. Refuses to overwrite without `--force`. Registered `pub mod init` in `startup/mod.rs`.
- **Docs drift fixed:** README quickstart rewritten to use `arawn init` (no hardcoded model → can't drift); dead `docs/src/getting-started.md` link → `docs/src/tutorials/first-chat.md`.
- **Model alignment:** changed the config default `openai/gpt-oss-20b` → `openai/gpt-oss-120b` across `config.rs` (default, embedded example, default-asserting tests) + the `config-schema.md` reference table. Rationale in the Acceptance Criteria note — 20b was the outlier *and* the owner's docs flag it as broken; 120b is what README/tutorial/llm-providers already recommend. llm_pool.rs test fixtures left at 20b (self-consistent, unrelated to the default).
- **Anti-drift pin:** new test `first_chat_tutorial_matches_default_model` reads `docs/src/tutorials/first-chat.md` and asserts it mentions `LlmConfig::default().model` — also fails if the tutorial file is missing, fencing the README link target.

**Tests (new, passing):** 5 in `init` (`rendered_config_parses_and_uses_code_default_model`, `provider_picks_conventional_key_env`, `explicit_overrides_win`, `refuses_to_overwrite_without_force`, `writes_then_force_overwrites`) + the drift pin. Manually ran `arawn --data-dir <tmp> init --provider groq` → valid config + correct next-steps output.

**Verification:** `cargo build --workspace` clean; arawn **67/0** lib + **2/0** bin; my changed source is `rustfmt --check` clean (lone main.rs:523 diff is the pre-existing classifier churn). The 2 arawn-engine sandbox-exec test failures are the known pre-existing flaky pair (fail identically on clean HEAD under full-suite parallelism). No new dependency.
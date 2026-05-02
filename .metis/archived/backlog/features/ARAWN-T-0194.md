---
id: first-run-config-ux-arawn-init-env
level: task
title: "First-run config UX: arawn init, env vars, actionable errors"
short_code: "ARAWN-T-0194"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T04:37:10.107117+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# First-run config UX: arawn init, env vars, actionable errors

## Objective

Today, configuring arawn requires hand-writing `~/.arawn/arawn.toml`, knowing which env var holds your API key, and reading source comments to discover defaults. Errors when something is missing are opaque (silent fall-through to default LLM, missing API key fails mid-request). A non-developer should be able to: run a single `arawn init` command that interactively builds the config, override anything via env var, and get an actionable message when something is wrong.

## Type / Priority
- Feature
- P1 — Blocker. Pairs with T-0193 (docs); together they cover "user can stand it up."

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `arawn init` subcommand (no args required) that interactively asks: provider (Groq / Ollama Cloud / OpenAI / local Ollama / custom URL), model name (with sensible default per provider), API key env var name (or value paste, stored to `~/.arawn/env`), data dir. Writes a complete `~/.arawn/arawn.toml`.
- [ ] `arawn init --non-interactive` form for scripted setup — accepts `--provider`, `--model`, `--api-key-env` flags.
- [ ] Refuses to overwrite existing `~/.arawn/arawn.toml` without `--force`.
- [ ] Env-var overrides for the most-changed fields: `ARAWN_DEFAULT_MODEL`, `ARAWN_DEFAULT_PROVIDER`, `ARAWN_DEFAULT_API_KEY_ENV`, `ARAWN_DATA_DIR`. Documented in README.
- [ ] On startup, if `[llm.default]` is missing → `error!` with link to `arawn init`.
- [ ] On startup, if `api_key_env` points at an env var that's unset → `error!` with the exact var name and a one-line "set it like: `export OLLAMA_API_KEY=...`".
- [ ] T-0190 warmup failures already surface clearly; verify the message guides the user toward fixing config rather than just stating the HTTP body.

## Implementation Notes

- `arawn init` lives in `crates/arawn/src/main.rs` as a clap subcommand; reuses `LlmConfig` defaults from `config.rs`.
- Use `dialoguer` or `inquire` for the prompts (both already common in the Rust ecosystem). Check `Cargo.toml` first.
- The "API key value paste" path: store to `~/.arawn/env` as `KEY=value`, ensure 0600 perms, source it on startup if present. Don't write keys into `arawn.toml`.
- This task does NOT cover OAuth flows for integrations (that's I-0033's territory) — only API-key style providers.

## Status Updates

### 2026-05-02 — Deferred (scope change)

Moved back to backlog without implementing. Two reasons:

1. **Design call from owner**: TOML is the source of truth for configuration. Env vars hold *secrets* via `api_key_env`, not config. So the env-var override layer (`ARAWN_DEFAULT_*`) and the `arawn init` wizard both fall away — there's no config to interactively choose if the user is expected to edit the TOML directly.
2. **Not currently painful**: the only user today already has a working `arawn.toml` and knows the env-var-for-secrets dance. The pain belongs to a future fresh-machine setup or a handoff to someone else, neither of which is on the immediate roadmap.

**What survives as a smaller follow-up** (worth filing if/when picked up):
- Validate minimum config at startup and **abort** on:
  - Missing `arawn.toml` at `<data_dir>/arawn.toml` → fail with pointer to `docs/src/getting-started.md`
  - Missing `[llm.default]` in the file → same
  - Engine LLM's `api_key_env` is set but the env var is unset → fail with `try: export FOO=...`
- Drop `apply_env_overrides`'s `GROQ_MODEL` branch (keep `ARAWN_DATA_DIR` — runtime path concern, also a clap arg).
- Update getting-started.md to remove any "configure via env var" guidance once the override is removed.
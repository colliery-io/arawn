---
id: first-run-config-ux-arawn-init-env
level: task
title: "First-run config UX: arawn init, env vars, actionable errors"
short_code: "ARAWN-T-0194"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T00:00:00+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


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

*To be added during implementation*

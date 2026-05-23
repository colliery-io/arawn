---
id: t-e-split-arawn-src-main-rs
level: task
title: "T-E: Split `arawn/src/main.rs` — startup phases under `startup/`"
short_code: "ARAWN-T-0401"
created_at: 2026-05-22T01:46:58.951638+00:00
updated_at: 2026-05-22T01:46:58.951638+00:00
parent: ARAWN-I-0054
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0054
---

# T-E: Split `arawn/src/main.rs`

## Parent Initiative

[[ARAWN-I-0054]]

## Direction (decided at task start)

`main.rs` is 2,521 lines. The bulk is one massive `if serve_mode { ... }` block — 2,077 lines spanning ~15 startup phases — plus 8 helper functions at the bottom (lines 2116-2521).

**Hitting the 800-line cap on `main.rs` is not realistic** without rewriting the startup flow: most phases share locals (`config`, `data_dir`, `store`, `workstream`, `service`, `registry`, `llm_pool`, `projections`, `feed_runtime`, etc.), so function-extraction means long parameter lists, and a context-struct rewrite is a heavier refactor than I-0054 allows.

**Scope-trimmed plan (consistent with T-D):** extract the four largest cohesive chunks where the function boundary is reasonably natural. Target a ~50% reduction (2,521 → ~1,250 lines), accept that the cap isn't hit, document the deferred work.

## Plan

Create `crates/arawn/src/startup/`. Extract these blocks:

1. **Bottom helper fns** (lines 2221-2521, ~300 lines) — `build_llm_client`, `register_default_tools`, `connect_mcp_servers`, `register_workflow_tools`, `build_engine_config`, `expand_github_org`, `register_one_feed`, `dirs_path`. Already proper fns; move them verbatim. Easiest win.
2. **`run_cli_via_server`** (lines 2116-2220, ~105 lines) — standalone fn for the CLI client path.
3. **Ceremony engine block** (lines 1517-1995, ~479 lines) — extract as `startup::ceremonies::wire_ceremony_engine(&config, workflow_runner_handle, &data_dir, &llm_pool, projections.as_ref(), &mut registry, &service)`.
4. **OAuth integration blocks** (lines 1051-1417, ~367 lines) — extract one fn per provider under `startup::integrations`: `register_gmail`, `register_calendar`, `register_drive`, `register_atlassian`, `register_github`, `register_slack`.
5. **Continual data feeds block** (lines 1441-1516, ~76 lines) — extract as `startup::feeds::wire_continual_feeds(...)`.

Helpers stay in `main.rs`: CLI argument struct uses, ctrl-c handler, broadcast wiring, config watcher spawn — these stitch the phases together and aren't useful as standalone modules.

## Acceptance Criteria

- [ ] Directory `crates/arawn/src/startup/` created with `mod.rs` + per-phase files.
- [ ] `main.rs` reduced from 2,521 to ≤ ~1,300 lines (target zone — not the 800 cap, see scope-trim above).
- [ ] All 8 bottom helper fns moved to `startup/`.
- [ ] Ceremony engine block extracted to `startup/ceremonies.rs`.
- [ ] 6 OAuth integration blocks each extracted to their own fn under `startup/integrations.rs`.
- [ ] Continual feeds block extracted to `startup/feeds.rs`.
- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean.
- [ ] All workspace lib tests pass.

## Status Updates

### 2026-05-22 — landed (scope-trimmed)

**Extraction summary:** main.rs went from **2,521 → 1,237 lines (-1,284, -51%)**. Doesn't hit the 800 cap (acknowledged at task start) but the four largest cohesive blocks are out.

| New file | Lines | Contents |
|---|---|---|
| `startup/mod.rs` | 18 | submodule declarations + re-exports |
| `startup/cli.rs` | 112 | `run_cli_via_server` — CLI client streaming path |
| `startup/helpers.rs` | 155 | `build_llm_client`, `register_default_tools`, `connect_mcp_servers`, `register_workflow_tools`, `dirs_path` |
| `startup/engine.rs` | 48 | `build_engine_config` |
| `startup/feeds_helpers.rs` | 112 | `expand_github_org`, `register_one_feed` (per-feed cron registration) |
| `startup/ceremonies.rs` | 505 | `wire_ceremony_engine` — retro/daily/weekly plugin construction, dispatcher, cron registration, agent tools |
| `startup/integrations.rs` | 409 | `wire_integrations` — Gmail/Calendar/Drive/Atlassian/GitHub/Slack OAuth registration; returns `IntegrationsForFeeds` bundle |
| `startup/feeds.rs` | 100 | `wire_continual_feeds` — runtime startup, hot-bind hook wiring |

**Patterns used:**
- Function signatures take `&Arc<...>` for shared registries and `&mut LocalService` for state mutation.
- `IntegrationsForFeeds` struct returns the six integration handles the continual-feeds block needs — passed `&` to `wire_continual_feeds` to avoid moving the bundle.
- Internal references to `arawn_bin::*` rewrote to `crate::*` (the modules live inside the `arawn_bin` library).
- Closures inside extracted blocks (cron schedulers, event forwarders) retain their `move` semantics — they capture by clone of `Arc` already.

**Deferred:**
- The remaining 1,237 lines in main.rs are the connective tissue: CLI parse, store/workstream open, embedder/memory bring-up, projection store, plugins, MCP, permissions, steward, workstream tools, late-bound hooks, workflow runner, TodoEvents broadcast, config watcher, shutdown. Each shares 5-10 locals with the next phase; extracting them would either need a ServeContext struct (bigger refactor than I-0054 allows) or 8-12-parameter signatures per fn (poor ergonomics).

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (34s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test --workspace --lib`: ✅ **1,758 tests pass**, 0 fail.

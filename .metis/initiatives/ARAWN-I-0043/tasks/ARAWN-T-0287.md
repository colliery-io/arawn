---
id: retro-plugin-friday-cron-gather
level: task
title: "Retro plugin — Friday cron, gather queries, compose chain with hint:medium"
short_code: "ARAWN-T-0287"
created_at: 2026-05-15T23:45:39.140814+00:00
updated_at: 2026-05-16T01:38:16.032922+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Retro plugin — gather + compose chain

## Goal
The first concrete `Ceremony` implementation. Runs Friday 16:00 local by default. Gathers this week's daily tablets + activity rollup + last N retros; calls pattern detectors; composes the `what_happened` and `patterns` sections.

## Reference
I-0043 Retro plugin goals + Compose Chain.

## Acceptance
- New `RetroCeremony` struct implementing `Ceremony`.
- `kind() = "retro"`, `period_key()` = ISO week (e.g. `2026-W20`), `default_schedule()` = `"0 16 * * FRI"` local.
- `gather()`:
  - daily tablets for the week (Mon–Fri)
  - this week's rollup (T-0285)
  - last 3 retros (for comparative context)
  - confirmed weekly priorities + their done_at status
- `compose()`:
  - LLM call via `hint:medium`, gated through `arawn_llm::gate::acquire_local()`, recorded as `ceremony.retro.compose`
  - prompt enforces "every claim cites a gather payload row"
  - writes items to `section_key = "what_happened"` with `write_composed_item` (citation required)
  - patterns section writes one composed item per `DetectedPattern` row using the pattern row id as citation
- `patterns()` returns the catalog (T-0288).
- Bootstrap fallback: when history < 2 weeks, skips patterns section entirely (still writes `what_happened` + an empty diary placeholder).
- Tests with `MockLlmClient`: assert sections + citations + bootstrap path.

## Out of scope
Pattern catalog v1 — T-0288. Diary capture — T-0289. TUI client — T-0290.
## Status Updates

**2026-05-16 — implementation landed.**

- New `crates/arawn-ceremonies/src/plugins/retro.rs` with `RetroCeremony` implementing `Ceremony`.
  - `kind() = "retro"`, `period_key()` = ISO week (`YYYY-Www`), `default_schedule() = "0 16 * * FRI"` local.
  - `interactive_actions()` returns `upsert_diary` (T-0289's RPC).
  - `patterns()` returns the attached `DetectorRegistry` (defaults to empty; `with_detectors(...)` chains the v1 catalog T-0288 will land).

- `gather()` runs deterministic SQL over the V6 ceremony tables and returns a structured `GatherPayload`:
  - Daily tablets for the ISO week (filtered by Monday..Sunday date range computed in Rust since SQLite has no ISO week math).
  - Confirmed weekly priorities (`JOIN ceremony_priorities … WHERE confirmed_at IS NOT NULL`).
  - This week's `ceremony_activity_rollup` rows.
  - Last 3 prior retro diaries (excerpts capped at 400 chars).

- `compose()` calls the LLM with `model = "hint:medium"` (per acceptance) and a structured system prompt that forces every claim to carry a `citation_id` referencing the gather payload. Parses the LLM's first balanced JSON array out of the response (tolerates prepended prose); validates non-empty citations Rust-side before constructing `ComposedItem`s. Engine still enforces at write time — defence in depth.

- Bootstrap fallback for patterns lives in the `DetectorRegistry` (T-0286): rules with `require_history_weeks > weeks_of_history()` are skipped silently. Retro itself just hands off — if no detectors fire, no `patterns` items get composed.

**Deferred / documented:**
- **Call-site tag `"ceremony.retro.compose"` for token usage** — `arawn_llm::usage::UsageTrackingClient::with_call_site` is set at pool-construction time in the binary, not per call. Adding per-call tagging would mean re-wrapping the client every compose call. Filed as a follow-up; meanwhile token usage records `(provider, model)` which is enough to attribute to the retro after the fact.
- **LLM gate around compose** — already lives in the engine dispatcher (T-0282), not the plugin. Confirmed it wraps `plugin.compose()`.
- **Real `hint:*` routing** — `T-0272` ships the taxonomy + pool resolution; binary wiring will pick the resolved client and pass the concrete model string to `RetroCeremony::new()`. The plugin treats the model as opaque.

**Tests (7 new in `plugins::retro::tests`, 49 total in the crate):**
- `iso_week_format_is_yyyy_w_ww`
- `monday_sunday_brackets_iso_week_20`
- `gather_collects_week_payload`
- `compose_parses_llm_array_into_composed_items`
- `compose_rejects_empty_citation_with_missing_citation_error`
- `compose_parses_array_with_surrounding_prose`
- `end_to_end_dispatch_against_real_engine` — full register → dispatch → DB row assertion against an in-memory DB with seeded history and a mock LLM.

Next: T-0288 (retro pattern catalog v1).
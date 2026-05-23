# ARAWN-I-0053 — Discovery inventory

*Comprehensive cruft sweep across code, tests, docs, and config/schema. Produced 2026-05-21 by the discovery phase of ARAWN-I-0053.*

## How this was produced

- Seven parallel read-only Explore agents, each scoped to a crate cluster or surface:
  - Agent 1: arawn-engine
  - Agent 2: arawn (binary) + arawn-tui
  - Agent 3: arawn-feeds + arawn-integrations + arawn-projections
  - Agent 4: arawn-ceremonies + arawn-storage + arawn-memory + arawn-steward
  - Agent 5: small crates (auth/core/embed/extractor/llm/mcp/service/tool/workflow + arawn-tests)
  - Agent 6: docs/src/**/*.md
  - Agent 7: Cargo.toml + config schema + SQL migrations + vendor/
- Dead-code validation experiment: all 15 `#[allow(dead_code)]` / `#[allow(unused_*)]` / `#[allow(deprecated)]` sites stripped, then rebuilt under three profiles (`cargo check --workspace`, `cargo build --workspace --release`, `cargo test --workspace --no-run`). Warnings captured per profile, then the tree was reverted via `git checkout -- crates/`.

## Verification baseline

| Profile | Result | Notes |
|---|---|---|
| `cargo build --workspace --release` | ✅ clean (1m07s incremental, 3m31s cold) | Zero warnings, zero errors. |
| `cargo check --workspace` | ✅ clean | Zero warnings. |
| `cargo test --workspace --no-run` | ❌ **FAILS** | Pre-existing compile error in `arawn-ceremonies/tests/brief_pipeline.rs:59`. Cargo's exit code surprisingly reports `0` through pipes; the failure is silent in piped contexts. See Tier 4. |

## Macro findings

1. **The composition root (arawn + arawn-tui) is exceptionally clean.** Agent 2 found zero `#[allow(...)]`, zero re-export aliases, zero fake-use functions. Despite `main.rs` being 2,513 lines and `event_loop.rs` 2,280 lines, the code is tightly used.
2. **Cruft concentrates in three crates:** arawn-engine (re-export shims, pre-T-0276 API), arawn-feeds (`JsonValue` alias, fake-use helpers, deprecated upstream API), arawn-ceremonies (legacy `since=`, test-only helpers, broken trait impl test).
3. **Many `#[allow(dead_code)]` markers are SUPERFLUOUS.** Rust's `dead_code` lint exempts names starting with `_`. The strip experiment confirmed `_unused`, `_value_marker`, `_ts`, `_force_use_traits`, `_add_item_unused` all silently satisfy the lint without their `#[allow]` markers. The markers were a misunderstanding of the language.
4. **Backward-compat shims fall into two camps:**
   - **Defensive against stored data** (FK columns, archived enum variants, dual-field migrations): must stay — data outlives code.
   - **Pre-cutover API surfaces** (pre-T-0276 wildcard permissions, hard-delete workstream): can be deprecated then removed.
5. **The dead-code experiment proved 8 dead symbols and 1 false positive caused by the broken test.** Empirical data ≠ initial agent assessments — verification mattered.
6. **Docs are in excellent shape.** Agent 6 found exactly ONE broken anchor link across 61 audited pages.
7. **Config/schema surface is clean.** Agent 7 found one dead workspace dep (`ignore`) and one duplicate Cargo dep declaration.

---

## Tier 1 — Mechanical removals

*Each row is a confirmed candidate for direct removal. Validated by the strip experiment unless otherwise noted.*

### A. Confirmed dead code (delete the symbol, drop the allow)

| File:Line | Symbol | Evidence | Action |
|---|---|---|---|
| `crates/arawn-ceremonies/src/service.rs:910` | `fn status_str(TabletStatus) -> &str` | Strip experiment: "never used" under debug + release + test profiles. Zero callers via grep. | Delete the function and its `#[allow(dead_code)]`. |
| `crates/arawn-ceremonies/src/engine.rs:526-535` | `fn rollback(conn) -> Result<...>` | Strip experiment: "never used" under all profiles. Zero callers via grep (begin/commit ARE used at engine.rs:830/840, rollback is not). | Delete the function and its `#[allow(dead_code)]`. |
| `crates/arawn-engine/src/tools/steward.rs:84-87` | `fn resolve_workstream(...)` | Strip experiment: "never used" under all profiles. Only "caller" is the `_unused()` fake-use at line 381, which is itself dead. | Delete both `resolve_workstream` and `_unused`. |
| `crates/arawn-engine/src/tools/steward.rs:380-383` | `fn _unused(...)` | Pure fake-use of `resolve_workstream`. | Delete. The `#[allow(dead_code)]` is also redundant because the name starts with `_`. |
| `crates/arawn-engine/src/plan.rs:32-33` | `PlanModeInner::stripped_rules: Vec<PermissionRule>` field | Strip experiment: "never read". Field is initialized as `Vec::new()` at line 54, never written elsewhere. Comment says "Reserved for future use" but the feature is not implemented. | Delete the field and its `#[allow(dead_code)]`. |
| `crates/arawn-engine/src/system_prompt.rs:208` | `PromptSection.name: String` field | Strip experiment: "never read". The `PromptSection` struct IS used (~15 constructions), but the `name` field is never read after construction. | Delete the field; the `#[allow(dead_code)]` at line 206 was on the struct but the actual unused item is just `name`. |
| `crates/arawn-feeds/src/store.rs:176-179` | `pub use serde_json::Value as JsonValue` + `fn _value_marker(_: Value) {}` | Agent 3 confirmed zero external callers of `arawn_feeds::JsonValue`. The `_value_marker` is a fake-use to prevent the alias from being flagged. | Delete the alias and the marker function. Drop both `#[allow(unused_imports)]` and `#[allow(unused)]`. |

### B. Superfluous `#[allow(dead_code)]` markers (keep the symbol, drop the allow)

These are `_`-prefixed names that are already exempted by Rust's `dead_code` lint convention. The `#[allow(dead_code)]` annotation was unnecessary from the start.

| File:Line | Symbol | Rationale |
|---|---|---|
| `crates/arawn-engine/src/tools/ceremony.rs:366-367` | `fn _add_item_unused(_: AddItemRequest) {}` | `_`-prefixed. If `AddItemRequest` is also imported for future use, verify import; if dead, remove import too. |
| `crates/arawn-steward/src/dust.rs:309-312` | `fn _ts() -> DateTime<Utc>` | `_`-prefixed; "no-op alias to keep import set obvious to readers" per comment. The `chrono::Utc::now()` is presumably called elsewhere; if not, delete the chrono import too. |
| `crates/arawn-feeds/src/templates/github/repo_mirror.rs:374-379` | `fn _force_use_traits()` | `_`-prefixed. Verify the "deliberately reference imports" goal is actually needed — if the imports are used in real code, this is redundant. |
| `crates/arawn-extractor/src/cot.rs:629-632` | `fn push_classify(&self, v: Value)` inside test module | NOT `_`-prefixed — this one's allow IS meaningful, but the symbol is genuinely unused per test build. **Move to Tier 1 A (delete) instead.** |

### C. Test-only helpers — convert to `#[cfg(test)]`

| File:Line | Symbol | Action |
|---|---|---|
| `crates/arawn-ceremonies/src/engine.rs:504-513` | `fn begin(conn) -> Result<...>` | Used by `mod tests` at engine.rs:830. Wrap the function in `#[cfg(test)]` (alongside `commit` below) instead of using `#[allow(dead_code)]`. The strip experiment misreported these as "never used" but that's a false positive from the broken `brief_pipeline.rs` aborting lint analysis. |
| `crates/arawn-ceremonies/src/engine.rs:515-524` | `fn commit(conn) -> Result<...>` | Used by `mod tests` at engine.rs:840. Same treatment as `begin`. |

### D. Backward-compat alias re-exports to remove

| File:Line | What | Evidence | Action |
|---|---|---|---|
| `crates/arawn-engine/src/lib.rs:32-34` | `pub use context::EngineToolContext as ToolContext;` | Agent 1: zero internal callers (no `arawn_engine::ToolContext` references in arawn-bin or arawn-tests). | Delete (after confirming no external callers — see Tier 3 if uncertain). |
| `crates/arawn-ceremonies/src/plugins/retro_detectors.rs:26` | `use ...monday_sunday_for_iso_week_public as monday_sunday_for_iso_week;` | The `_public` suffix exists only to differentiate from a private helper inside `retro.rs`. The alias collapses the suffix at the use site. | Rename `monday_sunday_for_iso_week_public` → `monday_sunday_for_iso_week` in `retro.rs` and drop the alias. |

### E. Workspace / dependency cleanup

| What | Where | Evidence | Action |
|---|---|---|---|
| `ignore = "0.4"` workspace dep | Root `Cargo.toml:62` + `crates/arawn-engine/Cargo.toml` | Agent 7: zero `use ignore::` or `ignore::` references workspace-wide. | Delete from both files. |
| `arawn-embed` listed in both `[dependencies]` and `[dev-dependencies]` | `crates/arawn-memory/Cargo.toml:7` and `:23` | Agent 4 + Agent 7 both flagged. Dev-deps inherit regular deps. | Remove the `[dev-dependencies]` line. |

### F. Pre-existing warnings (surfaced by the experiment, but pre-existed)

Discovered while comparing experiment output to baseline; these warnings exist independent of any `#[allow(...)]` site and represent additional small cleanups:

| File:Line | Warning |
|---|---|
| `crates/arawn-engine/src/hooks/executor.rs:226` | unused variable `result` |
| `crates/arawn-engine/src/system_prompt.rs:574` | unused import `std::path::PathBuf` |
| `crates/arawn-llm/src/retry.rs:96-98` | `FailThenSucceed.error_type` field never read |
| `crates/arawn-memory/tests/longmemeval_bench.rs` | Multiple unused: `parse_date_to_days`, `reciprocal_rank_fusion`, `temporal_score` functions; `question_id`/`question_date`/`haystack_dates` fields |
| Several test files | unused imports: `crate::types::ToolCall`, `std::sync::Arc`, `AtomicU32`/`Ordering`; unused variables `i`, `inner`, `request` |

### G. Documentation fix

| File:Line | Issue | Action |
|---|---|---|
| `docs/src/explanation/three-layer-data-model.md:112` | Broken anchor link `#when-to-bother` — actual anchor in `palaces.md` is `## When a palace makes sense`. | Update link to `#when-a-palace-makes-sense`. |

---

## Tier 2 — Documented "kept for X" cases

*Each row carries a one-line judgment. The judgments are proposals — operator can override during decompose.*

### KEEP — legitimate forward-compat / defensive

| File:Line | What | Why keep |
|---|---|---|
| `crates/arawn-engine/src/background.rs:122` | `BackgroundTask.handle: Option<JoinHandle<()>>` field | Held for destructor effect / future abort capability. Field IS written; just not read. Strip experiment confirmed warning fires in all profiles. The `#[allow(dead_code)]` is *necessary* and should be retained with an improved comment. |
| `crates/arawn-engine/src/ceremony_sources.rs:482-485` | `ProjectionsAttentionSource.store` field | Documented as reserved for future workstream joins. Low overhead. |
| `crates/arawn-engine/src/plugins/settings.rs:82-83` | Name-based plugin lookup fallback | Living convenience fallback. Not cruft. |
| `crates/arawn-engine/src/permissions/checker.rs:151-160` | `is_granted_shape()` with wildcard fallback | New preferred API; fallback handles the deprecated wildcard cleanly. |
| `crates/arawn-engine/src/system_prompt.rs:950-956` | `environment_no_longer_emits_date_line` test | Regression-prevention test for T-0368. Keep. |
| `crates/arawn-feeds/src/clients/slack.rs:513-519` | `.display_name` first, fallback to `.name` (legacy username) | Active backward-compat for workspaces with legacy username configs. |
| `crates/arawn-feeds/src/clients/github.rs:110-116` | `list_org_repos()` trait method | Reserved for T-0327 org-feed expansion. Documented. |
| `crates/arawn-feeds/src/clients/atlassian.rs:521-528` | `get_all_projects` deprecated upstream API | Documented choice with rationale (pagination + test ergonomics). **Note:** Strip experiment showed no deprecation warning fired even without `#[allow(deprecated)]` — the annotation may be redundant. Operator can decide. |
| `crates/arawn-integrations/src/slack/integration.rs:19-23` | Slack `search:read` scope deferral comment | Forward-reference for `slack_search` feature. Documented. |
| `crates/arawn-projections/src/github.rs:1105-1109, 1476-1484` | Skip legacy user-scoped feed dirs in walker | Defensive data-corruption prevention. Has test coverage. |
| `crates/arawn-memory/src/types.rs:186` | `Entity.tags` field | Part of documented dual-field migration (`tags` + `tags_ontology`). Stored data depends on this name. |
| `crates/arawn-storage/src/store.rs:67` | `create_workstream("scratch")` routes through `ensure_scratch_workstream()` | Reserved-name indirection. Defensive. |
| `crates/arawn-storage/src/workstream_store.rs:6` | `workstreams.id` Uuid column | FK target from `sessions.workstream_id`. Cannot remove without migration. |
| `crates/arawn-core/src/workstream.rs:114` | `Workstream.id` Uuid | Same FK reason; mirrors the storage row. |
| `crates/arawn-ceremonies/src/types.rs:18` | `TabletStatus::Archived` enum variant | Needed for historical stored rows. |
| `crates/arawn-ceremonies/src/runner.rs:112` | "removed first so..." comment | Explanatory, not cruft. |
| `crates/arawn-memory/src/store.rs:72-90` | "Drop the T-0239 legacy schema" defensive cleanup | Idempotent, documented, safe. |
| `crates/arawn-llm/src/gate/mod.rs:59-61` | `RemotePermit` marker type | Placeholder for T-0278 telemetry. |
| `crates/arawn/src/main.rs:1746` | "legacy `ceremony_event` category" parallel notice channels | Both branches actively in use during identified transition (I-0035 Phase 4). |

### KEEP — but consider improving / annotating

| File:Line | What | Suggested action |
|---|---|---|
| `crates/arawn-engine/src/background.rs:119-124` | `handle` field comment | Update comment to be precise about why the `#[allow]` is needed (held for Drop semantics; Rust's lint can't see Drop usage as a read). |
| `crates/arawn-ceremonies/src/service.rs:1005` | `tablet_id_prefix` parameter "unused but kept for naming intent" | Either rename to `_tablet_id_prefix` (lint exempted) or remove the parameter and call site. |

### DELETE proposed (Tier 2 candidates with no real future need)

| File:Line | What | Why delete |
|---|---|---|
| `crates/arawn-engine/src/permissions/checker.rs:132-149` | `SessionGrants::grant()` + `SessionGrants::is_granted()` (pre-T-0276 wildcard API) | Replaced by `grant_shape()` / `is_granted_shape()`. New API already falls back to wildcards internally. Two-step: mark `#[deprecated]` first to nudge any external callers, then remove. |
| `crates/arawn-engine/src/tool.rs` (entire file) | Pure 6-line re-export shim of `arawn-tool` types | Lib.rs can re-export directly from `arawn-tool`. The tests in `tool.rs` belong with the canonical types in `arawn-tool` or as integration tests. |
| `crates/arawn-storage/src/workstream_store.rs:222` | `WorkstreamStore::delete` (hard-delete by id) | `soft_delete(name)` is the preferred API. Agent 4 found no external callers. Two-step deprecation. |
| `crates/arawn-ceremonies/src/plugins/gather_sources.rs:51-54` | `AttentionSource::since(cursor)` legacy method | Replaced by `between(start, end)`. Audit production plugins for any `since=` users; if none, delete. |
| `crates/arawn-feeds/src/clients/atlassian.rs:525` | `#[allow(deprecated)]` annotation itself | Strip experiment showed no deprecation warning even when the annotation was removed. Either delete the annotation (if no warning ever fires) or document why it's defensive. |

---

## Tier 3 — Stale concept candidates (operator decisions LOCKED)

*Operator review completed 2026-05-21. Decisions recorded inline below.*

### Summary of decisions

| # | Candidate | Decision | Notes |
|---|---|---|---|
| 3.1 | `AtlassianFeedClient::resolve_project()` | **KEEP** | Operator believes it should be USED. Spawn follow-up task to wire it into the appropriate Jira/Confluence template. |
| 3.2 | Pre-T-0276 wildcard permission API | **DROP** | Delete `grant`/`is_granted` wildcard methods. |
| 3.3 | `arawn_engine::tool` re-export shim | **UPDATE + DROP** | Migrate callers to `arawn-tool`, then delete the shim. |
| 3.4 | `EngineToolContext as ToolContext` alias | **DROP** | Kill the backward-compat alias. |
| 3.5 | `WorkstreamStore::delete` hard-delete | **DROP** | Kill it. |
| 3.6 | `AttentionSource::since()` legacy cursor | **DROP** | Kill it. |
| 3.7 | arawn-memory benchmarks | **KEEP** | Benchmarks are important for memory-model capability tracking. Separate follow-up: update scenarios to match current use cases / loaded data. |
| 3.8 | ceremonies `tablet_id_prefix` parameter | **DROP** | Remove the parameter from `build_service_with_items` (9 call sites pass `"retro"` redundantly with `kind`). |
| 3.9 | Forward-reservation audit | **PARTIAL DROP** (see audit below) | Audited each reservation against Metis. Three of four turn out to be cruft or live code; one stays. |

### 3.9 Audit details

The 3.9 audit verified each "reserved for T-XXXX" pattern against Metis and grep:

| Reservation | Site | Audit finding | Action |
|---|---|---|---|
| `list_org_repos()` | `feeds/clients/github.rs:112` | T-0327 (completed, archived). **Live in production** at `crates/arawn/src/main.rs:2410`. Agent 3's "never called" was wrong. | **No action.** Live code, comment is historical attribution. |
| `RemotePermit` marker | `llm/gate/mod.rs:59-61` | T-0278 (completed). The routing layer shipped but T-0278's own status notes "Wiring deferred" — the agent loop still uses `resolve_hint()` not `routing_provider()`. Comment at `query_engine.rs:750` confirms pending wiring. | **Keep.** Genuine pending-wiring case. Could file a backlog task to do the wiring, but that's separate from this initiative. |
| `_add_item_unused` (for `retro_add_item`) | `engine/tools/ceremony.rs:366-367` | No Metis task for `retro_add_item`. Grep shows `AddItemRequest` is **already used in production** (ws_server.rs, daily.rs, ceremony service). The "keep the import" rationale was misunderstood — the import is needed for the inline tests at line 446+, not for future expansion. | **Drop.** Delete `_add_item_unused`. The `AddItemRequest` import stays for the tests. |
| Slack `search:read` scope reservation | `integrations/slack/integration.rs:19-23` | No Metis task for `slack_search` template. Comment says "Re-add when `slack_search` lands" but there's no work item to land. | **Drop comment, keep behavior.** The implementation behavior (don't request `search:read`) is correct. The detailed multi-line comment can be replaced with a one-liner or removed entirely. If `slack_search` is ever filed, the scope addition will happen then. |

### Follow-up tasks generated by Tier 3 decisions

These do NOT belong inside this initiative (per non-goal: "no new behavior"), but should be filed separately:

- **Wire `AtlassianFeedClient::resolve_project()` into Jira/Confluence templates** (T-3.1-followup) — identify which templates should resolve user-provided project names by ID/key vs accepting them raw. Operator approval gate before any code changes.
- **Update arawn-memory benchmark scenarios to match current use cases** (T-3.7-followup) — refresh the longmemeval / recall_eval datasets to reflect arawn-specific memory patterns.
- **Optional: wire `RoutingProvider` into the engine agent loop** (T-3.9b-followup) — finish the deferred wiring noted in T-0278's status.

### Candidate 3.1: `AtlassianFeedClient::resolve_project()` trait method

- **What it is:** Trait method that looks up a Jira project by ID/key.
- **Lives at:** `crates/arawn-feeds/src/clients/atlassian.rs:128` (trait), `:502-516` (impl), test fakes in `jira_trackers.rs`, `confluence_space_archive.rs`, `discovery.rs`.
- **Evidence of staleness:** Defined and implemented, but never called by any production template. All Jira/Confluence templates use `list_jira_projects()`, `list_confluence_spaces()`, `jql_search()`, or `issue_full()` instead. Comment at module top says "landed in T-0223" but the integration never followed.
- **Blast radius:** Trait signature change. Removing requires deleting the method from `RealAtlassianClient` and all three test fakes. No external callers in arawn-bin or arawn-tests.
- **Recommendation:** **delete-candidate.**
- **Operator decision:** _____________

### Candidate 3.2: Pre-T-0276 wildcard permission API (`SessionGrants::grant`, `SessionGrants::is_granted`)

- **What it is:** Wildcard grant methods on `SessionGrants` superseded by shape-aware variants.
- **Lives at:** `crates/arawn-engine/src/permissions/checker.rs:132-149`.
- **Evidence of staleness:** Explicit "Kept for backwards-compat with the pre-T-0276 API" comment. New shape-aware API (`grant_shape`, `is_granted_shape`) supersedes; the new API already falls back to wildcard internally.
- **Blast radius:** External callers of `arawn_engine::SessionGrants::grant`/`is_granted` would break. Internal callers: none found by Agent 1. Tests use the new API.
- **Recommendation:** Two-step. Mark `#[deprecated]` first (one release), then delete.
- **Operator decision:** _____________

### Candidate 3.3: `arawn_engine::tool` re-export shim module

- **What it is:** Six-line `pub use arawn_tool::{...}` shim with a few inline tests.
- **Lives at:** `crates/arawn-engine/src/tool.rs`.
- **Evidence of staleness:** Pure re-export with "backward-compatible re-exports" comment. `arawn-tool` is the canonical home.
- **Blast radius:** Any external code using `use arawn_engine::tool::{Tool, ToolCategory, ...}` breaks. Lib.rs currently re-exports through this module.
- **Recommendation:** Update `lib.rs` to re-export directly from `arawn-tool`, delete the shim module, move tool registry tests to `arawn-tool` (or delete if redundant with arawn-tool's own tests).
- **Operator decision:** _____________

### Candidate 3.4: `EngineToolContext as ToolContext` backward-compat alias

- **What it is:** `pub use context::EngineToolContext as ToolContext;` at `crates/arawn-engine/src/lib.rs:32`.
- **Evidence of staleness:** "Backward-compatible alias" comment. Agent 1 confirmed zero internal callers in arawn-bin or arawn-tests.
- **Blast radius:** External code using `arawn_engine::ToolContext` would break. Same audit as 3.3.
- **Recommendation:** Delete (after a final grep through any vendored or example code).
- **Operator decision:** _____________

### Candidate 3.5: `WorkstreamStore::delete` (hard-delete)

- **What it is:** Hard-delete-by-id method predating soft-delete-by-name.
- **Lives at:** `crates/arawn-storage/src/workstream_store.rs:222`.
- **Evidence of staleness:** "Retained for backward compatibility with the V1 surface; new code paths should prefer `soft_delete(name)`" comment. Agent 4 found no external callers.
- **Blast radius:** Tooling that hard-deletes workstreams would break. None known.
- **Recommendation:** Two-step deprecation, then delete.
- **Operator decision:** _____________

### Candidate 3.6: `AttentionSource::since()` legacy cursor method

- **What it is:** Open-ended "everything newer than this cursor" gather query.
- **Lives at:** `crates/arawn-ceremonies/src/plugins/gather_sources.rs:51-54`.
- **Evidence of staleness:** Explicit "legacy" comment. New code uses `between(start, end)`.
- **Blast radius:** Any ceremony plugin or attention source still using `since` would break. Need to audit `crates/arawn-ceremonies/src/plugins/` for production users.
- **Recommendation:** Audit → migrate any remaining callers → delete.
- **Operator decision:** _____________

### Candidate 3.7: arawn-memory benchmark suites (longmemeval_bench, recall_eval)

- **What they are:** Two `--ignored` benchmark tests requiring external model downloads and ~5 minutes per run.
- **Live at:** `crates/arawn-memory/tests/longmemeval_bench.rs` (500+ lines), `crates/arawn-memory/tests/recall_eval.rs` (300+ lines).
- **Evidence of staleness:** Not run by CI; require external setup; no `arawn` documentation pointer; the strip experiment surfaced multiple pre-existing dead-code warnings inside them (functions/fields never used).
- **Blast radius:** Removing them does not affect the binary or library. The benchmarks become unrecoverable from the repo (though git history retains them).
- **Recommendation:** Operator decision — are these still actively used? If yes, fix their dead-code warnings as part of Tier 1 cleanup. If no, archive (move to a `benches/` directory or delete).
- **Operator decision:** _____________

### Candidate 3.8: ceremonies `tablet_id_prefix` unused parameter

- **What it is:** Parameter at `crates/arawn-ceremonies/src/service.rs:1005` documented as "unused but kept for naming intent."
- **Evidence of staleness:** The parameter value is never consumed; only the parameter name documents intent.
- **Blast radius:** Minimal — internal function signature change.
- **Recommendation:** Operator decision — rename to `_tablet_id_prefix` (lint silently exempted) or remove the parameter from the signature and all call sites.
- **Operator decision:** _____________

### Candidate 3.9: Should we KEEP forward-reservation patterns or DELETE them?

The codebase has multiple "reserved for T-XXXX" patterns where a symbol exists today because a future task is planned. Examples:
- `list_org_repos()` (feeds/clients/github.rs:112) — reserved for T-0327.
- `RemotePermit` marker type (llm/gate/mod.rs:59-61) — reserved for T-0278 telemetry.
- `_add_item_unused()` (engine/tools/ceremony.rs:366-367) — reserved for a future `retro_add_item` tool.
- Slack `search:read` scope comment (integrations/slack/integration.rs:19-23) — reserved for `slack_search` template.

Per the initiative's non-goal of "adding new abstractions or helper layers", we leave these alone. But operator may want to audit which tasks are still planned vs which have been forgotten.

- **Operator decision:** ☐ Keep all (default). ☐ Audit each for whether the planned task is still real. ☐ Remove all (and re-add when the actual work lands).

---

## Tier 4 — Critical bugs (not cruft, but surfaced by the sweep)

### 4.1 — Broken integration test: `brief_pipeline.rs`

- **Where:** `crates/arawn-ceremonies/tests/brief_pipeline.rs:59`
- **Symptom:** `ScriptedPlugin` implements `Ceremony` but is missing the required `period_window` trait method. The compiler error is `E0046: not all trait items implemented`.
- **Impact:**
  - The `brief_pipeline.rs` integration test target does not compile.
  - Any tests in that file are NOT being run.
  - Cargo's exit code through piped commands reports `0` despite this failure (bash `$?` after pipe captures only the last command's exit code, not cargo's; `${PIPESTATUS[0]}` is needed). This means CI may also be silently passing — worth verifying CI catches this.
  - The strip experiment was further contaminated by this: lint analysis aborts for `arawn-ceremonies` test targets when this file fails to compile, masking dead-code warnings on `begin`/`commit` that ARE used by inline tests.
- **Recommendation:** Fix as a separate backlog task — not part of this initiative's scope. Likely a 5-line fix: add the missing `period_window` method to `ScriptedPlugin`. Also recommend auditing CI's cargo exit-code handling.
- **Severity:** Medium. Silent test-coverage hole.

---

## Tier 5 — Pre-existing dead-code warnings (cleanup opportunity)

These warnings exist in the tree today, *independently* of any `#[allow(...)]` site. They were surfaced by the strip experiment but exist regardless. Each one is a small, mechanical Tier 1 candidate:

- `crates/arawn-engine/src/hooks/executor.rs:226` — `unused variable: result` (prefix with `_` or use it)
- `crates/arawn-engine/src/system_prompt.rs:574` — `unused import: std::path::PathBuf`
- `crates/arawn-llm/src/retry.rs:96-98` — `FailThenSucceed.error_type` field never read (test fixture)
- `crates/arawn-memory/tests/longmemeval_bench.rs` — `parse_date_to_days`, `reciprocal_rank_fusion`, `temporal_score` functions never used; `question_id`, `question_date`, `haystack_dates` fields never read (this is the bench Tier 3.7 candidate)
- `crates/arawn-engine/src/...` — `unused imports: crate::types::ToolCall`, `std::sync::Arc`, `AtomicU32`/`Ordering`; `unused variables: i, inner, request` (locations to be pinpointed during task execution)

These should be folded into the engine / extractor / llm Tier 1 deletion tasks rather than getting their own task.

---

## Cross-cutting observations

1. **The `_`-prefix exemption pattern is the single biggest cleanup lever.** Multiple `#[allow(dead_code)]` markers exist on `_`-prefixed names where Rust's lint already exempts them. Removing those allow markers (and the now-pointless fake-use functions themselves, since some have no real purpose) cleans up a noticeable fraction of the cruft surface with zero behavioral risk.

2. **The dead-code experiment was essential — agent guesses were not enough.** Agent 1 said `PromptSection` was "alive, the allow is spurious." That's half right: the *struct* is alive but its `name` *field* is dead. Agent 4 said `begin`/`commit`/`rollback` are used by tests. Half right: `begin`/`commit` are used, `rollback` is not. Only empirical compilation could distinguish these.

3. **Pre-existing broken test masks coverage.** The `brief_pipeline.rs` issue isn't just a regression — it's actively distorting lint analysis. Fixing it should precede the Tier 1 ceremonies tasks so the lint signal is reliable.

4. **The composition root is in much better shape than the subsystems.** `arawn` and `arawn-tui` are large but tight. The cruft is in the subsystems where iteration churn was highest: engine, feeds, ceremonies.

5. **Documentation is in remarkably good shape.** Only one broken anchor and a handful of `file:LINE` references with minor line-number drift. The Diataxis restructure (I-0051) and the integration-docs rewrite (I-0038) clearly raised the doc quality bar.

6. **Config / schema surface is essentially clean.** One unused workspace dep, one dup dep declaration. That's it across 20 Cargo.toml files, 12 migrations, and the full config struct.

7. **The "reserved for T-XXXX" pattern is well-documented but unaudited.** Many forward-reservation symbols exist with task references. Some references may be stale (the task has been deferred or cancelled). Operator should consider an audit as a follow-up.

---

## Locked task clusters (input for the decompose phase)

*Operator decisions on Tier 3 are recorded. Below is the final task list for Phase 2 (decompose). Ordering is by blast radius — mechanical / dependency / docs first, then Tier 2 deprecations, then Tier 3 removals.*

| # | Cluster | Scope |
|---|---|---|
| **T-A** | Workspace dependency cleanup | Remove `ignore = "0.4"` from root `Cargo.toml` and `crates/arawn-engine/Cargo.toml`. Remove duplicate `arawn-embed` from `crates/arawn-memory/Cargo.toml` `[dev-dependencies]`. Tiny; cargo check workspace only. |
| **T-B** | Docs fixes | Fix broken anchor in `docs/src/explanation/three-layer-data-model.md:112` (`#when-to-bother` → `#when-a-palace-makes-sense`). Optionally update `explanation/index.md` to list integrations-overview and oauth-primer. Pure docs. |
| **T-C** | Tier 1 — arawn-engine mechanical removals | Delete `_unused` + `resolve_workstream` in `tools/steward.rs`. Delete `_add_item_unused` in `tools/ceremony.rs:366-367` (note: `AddItemRequest` import stays — it's used by tests). Drop `PromptSection.name` field in `system_prompt.rs:208`. Drop `PlanModeInner.stripped_rules` field in `plan.rs:32`. Fix pre-existing warnings in `hooks/executor.rs:226` and `system_prompt.rs:574`. Includes strip-and-rebuild validation per methodology. |
| **T-D** | Tier 1 — arawn-ceremonies mechanical removals | Delete `status_str` in `service.rs:910`. Delete `rollback` in `engine.rs:526`. Convert `begin`/`commit` in `engine.rs:504/515` to `#[cfg(test)]`. Rename `monday_sunday_for_iso_week_public` → `monday_sunday_for_iso_week` in `plugins/retro.rs`; drop the alias at `plugins/retro_detectors.rs:26`. Remove `tablet_id_prefix` parameter from `build_service_with_items` (9 call sites). **Depends on T-Z** for clean lint signal. |
| **T-E** | Tier 1 — arawn-feeds mechanical removals | Delete `JsonValue` alias + `_value_marker` in `store.rs:175-179`. Clean up `_force_use_traits` in `templates/github/repo_mirror.rs:374` (verify whether trait imports it "forces" are actually needed by real code). Strip the `#[allow(deprecated)]` on `get_all_projects` in `clients/atlassian.rs:525` and verify no deprecation warning fires (per strip experiment finding). |
| **T-F** | Tier 1 — small-crate cleanup | Delete `_ts` in `arawn-steward/dust.rs:309`. Delete `push_classify` in `arawn-extractor/cot.rs:629`. Fix pre-existing warnings in `arawn-llm/retry.rs:96-98`. |
| **T-G** | Tier 3 (3.3) — drop `arawn_engine::tool` re-export shim | Update `lib.rs` to re-export directly from `arawn-tool`. Move tool registry tests to `arawn-tool` (or delete if redundant). Delete `crates/arawn-engine/src/tool.rs` entirely. Update any external consumers. |
| **T-H** | Tier 3 (3.4) — drop `EngineToolContext as ToolContext` alias | Remove the alias at `crates/arawn-engine/src/lib.rs:32-34`. Final grep through arawn-bin, arawn-tests, vendor/, examples/ before deletion. |
| **T-I** | Tier 3 (3.2) — delete pre-T-0276 permission API | Delete `SessionGrants::grant()` and `SessionGrants::is_granted()` in `checker.rs:132-149`. Per operator decision: skip deprecation cycle, drop directly. Verify zero internal callers remain. |
| **T-J** | Tier 3 (3.5) — delete `WorkstreamStore::delete` hard-delete | Remove `WorkstreamStore::delete(id: Uuid)` at `workstream_store.rs:222`. Verify no callers via grep. |
| **T-K** | Tier 3 (3.6) — delete `AttentionSource::since()` legacy cursor | Audit `crates/arawn-ceremonies/src/plugins/` for any `since=` callers in production attention sources. If none, remove the method from the trait + impls. If some, migrate to `between` first. |
| **T-L** | Tier 3 (3.9 audit cleanup) — Slack scope reservation comment trim | Shorten/remove the multi-line "Re-add when `slack_search` lands" comment at `integrations/slack/integration.rs:19-23` since no `slack_search` task exists. Keep the implementation behavior unchanged. |
| **T-Z** | (Out of initiative scope) Backlog: fix `brief_pipeline.rs` ScriptedPlugin | Add the missing `period_window` trait method. File as a separate backlog bug task. Prerequisite for clean T-D lint signal. |

**Total: 11 in-scope tasks** (T-A through T-L, skipping I/O letters as commonly confusing) + **1 out-of-scope prerequisite** (T-Z).

### Follow-up backlog (filed separately, NOT part of this initiative)

Generated by Tier 3 decisions that involve adding behavior, which this initiative explicitly excludes:

- **(T-3.1-followup) Wire `AtlassianFeedClient::resolve_project()` into Jira/Confluence templates.** Operator decision: keep + use. Spawn separate task after this initiative closes.
- **(T-3.7-followup) Refresh arawn-memory benchmark scenarios to match current use cases / loaded data.** Operator decision: benchmarks important; need updated scenarios.
- **(T-3.9-routing-followup) Wire `RoutingProvider` into the engine agent loop.** Completion of T-0278's deferred wiring (the routing layer landed, but the agent loop still uses `resolve_hint()` instead of `routing_provider()`).

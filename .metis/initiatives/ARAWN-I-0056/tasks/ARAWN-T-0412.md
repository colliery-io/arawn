---
id: t-a-startup-loader-load-merged
level: task
title: "T-A: Startup loader — load merged hooks + attach HookRunner"
short_code: "ARAWN-T-0412"
created_at: 2026-05-23T03:31:00+00:00
updated_at: 2026-05-23T03:50:46.411810+00:00
parent: ARAWN-I-0056
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0056
---

# T-A: Startup loader — load merged hooks + attach HookRunner

## Parent Initiative

[[ARAWN-I-0056]]

## Objective

Load hook config at startup from `~/.arawn/settings.json` (user-level) merged with `<workstream_root>/.arawn/settings.json` (project-level) and attach a `HookRunner` to both `QueryEngine` and `LocalService` for downstream fire-site tasks. No fire sites in this task — just the wire-up.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `crates/arawn/src/startup/hooks.rs` module with a `pub fn load_and_build_hook_runner(data_dir: &Path, workstream_root: &Path) -> Arc<HookRunner>` helper.
- [ ] Helper resolves user settings path as `<data_dir>/settings.json` (e.g., `~/.arawn/settings.json`).
- [ ] Helper resolves project settings path as `<workstream_root>/.arawn/settings.json`.
- [ ] Helper calls `arawn_engine::hooks::load_merged_hooks(Some(user), Some(project))` and constructs `HookRunner::new(config, workstream_root.to_path_buf())`.
- [ ] `LocalService` has a new field `hook_runner: Option<Arc<HookRunner>>` populated at startup; `with_hook_runner(self, runner) -> Self` builder method.
- [ ] `QueryEngine`'s existing `with_hook_runner` is invoked in `LocalService::build_engine_*` paths so every engine instance gets the same runner.
- [ ] `startup/mod.rs` exports `hooks` and `main.rs`/`local_service` call into it during the serve-mode setup.
- [ ] Tracing log line at startup: `info!(user_hooks = ?user_loaded_count, project_hooks = ?project_loaded_count, "hooks loaded")` (or similar) so operators can verify their config parsed.
- [ ] Unit tests in `startup/hooks.rs` covering:
  - [ ] Empty paths → returns empty `HookConfig`.
  - [ ] User-only settings → hooks loaded from user file.
  - [ ] Project-only settings → hooks loaded from project file.
  - [ ] Both present → merged correctly (uses existing `load_merged_hooks` semantics).
- [ ] `cargo check --workspace` clean, `cargo test --workspace --lib` green.

## Implementation Notes

- The hooks system already has `load_merged_hooks` that handles deduplication and merging — no need to write it ourselves.
- `HookRunner` is cheap to construct (just stores config + cwd); cloning the `Arc` is the right shape for sharing between `LocalService` and `QueryEngine`.
- Do NOT add any `.fire_hook(...)` call sites in this task. That's T-B/C/D's job. Keep T-A focused on wire-up only.
- Out of scope: hot reload of hooks config. The existing `HookFileWatcher` could enable this in a follow-up; not in V1.

## Status Updates

### 2026-05-23 — landed

**New module:** `crates/arawn/src/startup/hooks.rs` (~125 lines) with `load_and_build_hook_runner(data_dir, workstream_root) -> Arc<HookRunner>`. Resolves `<data_dir>/settings.json` and `<workstream_root>/.arawn/settings.json`, calls `arawn_engine::hooks::load_merged_hooks`, constructs `HookRunner::new(config, workstream_root)`, logs total hook groups at INFO. Missing files silently treated as empty config.

**LocalService:**
- New field `hook_runner: Option<Arc<HookRunner>>`.
- New builder method `with_hook_runner(self, runner) -> Self`.
- `build_engine` attaches the runner to every `QueryEngine` instance via the existing `QueryEngine::with_hook_runner` setter (guarded `if let Some(ref hook_runner)`).

**Startup wire-up:** `main.rs` at serve-mode setup loads the runner just before constructing `LocalService` and chains `.with_hook_runner(hook_runner)` into the builder. `cwd = workstream.root_dir` per the operator-confirmed cwd choice.

**Module exports:** `startup/mod.rs` declares `pub mod hooks` and re-exports `load_and_build_hook_runner`.

**Unit tests (4 in `startup/hooks.rs`):**
- `returns_empty_runner_when_no_settings_files_exist` — neither path present.
- `loads_user_only_settings` — only user file present.
- `loads_project_only_settings` — only project file present.
- `merges_user_and_project_settings` — both present.

**Behavior change scope:** zero. Hooks still don't fire — the runner is attached but no `.fire_hook(...)` call sites exist yet (that's T-B/C/D).

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 04s).
- `cargo test --workspace --lib`: ✅ **1,742 tests pass** (was 1,738; +4 T-A loader tests). Pre-existing harness flake `testing::harness::tests::harness_shell_tool_receives_arguments` fired once but cleared on re-run; not caused by T-A.
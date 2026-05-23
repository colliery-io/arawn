---
id: t-a-startup-loader-attach-hookrunner
level: task
title: "T-A: Startup loader — load merged hooks + attach HookRunner"
short_code: "ARAWN-T-0412"
created_at: 2026-05-23T03:31:00.000000+00:00
updated_at: 2026-05-23T03:31:00.000000+00:00
parent: ARAWN-I-0056
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0056
---

# T-A: Startup loader — load merged hooks + attach HookRunner

## Parent Initiative

[[ARAWN-I-0056]]

## Objective

Load hook config at startup from `~/.arawn/settings.json` (user-level) merged with `<workstream_root>/.arawn/settings.json` (project-level) and attach a `HookRunner` to both `QueryEngine` and `LocalService` for downstream fire-site tasks. No fire sites in this task — just the wire-up.

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

*To be added during implementation*

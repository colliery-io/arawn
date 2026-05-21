---
id: t-e-tier-1-arawn-feeds-mechanical
level: task
title: "T-E: Tier 1 — arawn-feeds mechanical cruft removal"
short_code: "ARAWN-T-0382"
created_at: 2026-05-21T14:53:21.578365+00:00
updated_at: 2026-05-21T15:39:21.644005+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-E: Tier 1 — arawn-feeds mechanical cruft removal

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Delete the dead `JsonValue` re-export, the paired `_value_marker` fake-use,
and the `_force_use_traits` fake-use in the github repo_mirror template.
Verify whether the `#[allow(deprecated)]` annotation on the Atlassian
`get_all_projects` call is necessary.

## Acceptance Criteria

## Acceptance Criteria

### Dead re-export + fake-use

- [ ] Delete `pub use serde_json::Value as JsonValue;` at `crates/arawn-feeds/src/store.rs:175-177` along with its `#[allow(unused_imports)]`.
- [ ] Delete `fn _value_marker(_: Value) {}` at `crates/arawn-feeds/src/store.rs:178-179` and its `#[allow(unused)]`.
- [ ] Grep workspace-wide for `arawn_feeds::JsonValue` to confirm zero external callers (Agent 3 reported none, but verify).

### `_force_use_traits` cleanup

- [ ] Delete `fn _force_use_traits()` at `crates/arawn-feeds/src/templates/github/repo_mirror.rs:374-379` and its `#[allow(dead_code)]`.
- [ ] After deletion, run `cargo check -p arawn-feeds` and verify no unused-import warnings appear for the trait imports the function pretended to use. If warnings appear, the trait imports themselves were dead — remove them too.

### `#[allow(deprecated)]` audit

- [ ] Strip `#[allow(deprecated)]` at `crates/arawn-feeds/src/clients/atlassian.rs:525` and rebuild on all three profiles.
- [ ] If NO deprecation warning fires under any profile (as the discovery experiment found), keep the annotation stripped — it was redundant. Update the comment block at lines 521-524 to drop the now-misleading reference to the allow.
- [ ] If a deprecation warning DOES fire under some profile, restore the annotation with a precise `#[cfg_attr(...)]` form that scopes the suppression to the configuration that needs it, and document the configuration in a one-line comment.

### Validation

- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean (modulo T-Z).
- [ ] `angreal test unit` passes (specifically feed template tests).
- [ ] `angreal test integration` passes.

## Implementation Notes

### Technical Approach

1. Mechanical deletions in store.rs (alias + marker).
2. Delete `_force_use_traits`, then verify trait imports needed by real code.
3. Audit the `#[allow(deprecated)]` per the methodology in the initiative.
4. Validate.

### Dependencies

None internal to the initiative.

### Risk Considerations

- `_force_use_traits` was supposed to "deliberately break the build if these imports are removed." If removing the function unmasks that some imports really are unused, the build will warn — those imports should also be removed.

## Verification

Per the initiative's dead-code methodology:
- `angreal check workspace`
- `cargo build --workspace --release`
- `cargo test --workspace --no-run`
- `angreal test unit`
- `angreal test integration`
- `angreal check clippy` for arawn-feeds

## Status Updates

### 2026-05-21 — landed

- Deleted `pub use serde_json::Value as JsonValue;` + `#[allow(unused_imports)]` and the paired `fn _value_marker(_: Value) {}` + `#[allow(unused)]` in `crates/arawn-feeds/src/store.rs:175-179`. Grep confirmed zero `arawn_feeds::JsonValue` callers anywhere (the only other `JsonValue` references in the workspace are arawn-memory's own local alias, unrelated).
- Removed the now-unused `use serde_json::Value;` from store.rs (line 10) — it was only there because `_value_marker` referenced it.
- Deleted `fn _force_use_traits()` + its `#[allow(dead_code)]` in `crates/arawn-feeds/src/templates/github/repo_mirror.rs:374-379`.
- Removed the now-unused `use crate::clients::GithubFeedClient;` from repo_mirror.rs:25 — same reason as above. The test mod has its own `use crate::clients::{FeedClients, GithubFeedClient as Gh};` so test code is unaffected. Production code accesses the GitHub client via `TemplateCtx`, not via this trait import.
- Stripped `#[allow(deprecated)]` at `crates/arawn-feeds/src/clients/atlassian.rs:525`. Rebuilt on debug/release/test — **no deprecation warning fired in any profile**, confirming the annotation was redundant. Updated the surrounding comment to drop the now-stale reference to the allow ("get_all_projects is deprecated upstream but still works...").

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (48.62s incremental).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test -p arawn-feeds`: ✅ all tests pass across all targets (cloacina_fire, discovery, gmail_archive, github_repo_mirror_smoke, jira_trackers, slack_*, drive_*, calendar_*, confluence_space_archive, dynamic_register, plus the lib unit tests).
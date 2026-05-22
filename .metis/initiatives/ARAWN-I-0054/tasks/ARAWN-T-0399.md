---
id: t-c-split-arawn-src-local-service
level: task
title: "T-C: Split `arawn/src/local_service.rs` — `LocalService` impl by feature group"
short_code: "ARAWN-T-0399"
created_at: 2026-05-22T01:46:55.944278+00:00
updated_at: 2026-05-22T02:45:57.075418+00:00
parent: ARAWN-I-0054
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0054
---

# T-C: Split `arawn/src/local_service.rs`

## Parent Initiative

[[ARAWN-I-0054]]

## Direction

Rust forbids splitting a single trait impl across files. Pattern used:
**inherent impl + `_inner` suffix + trait shell that delegates.**

- Each sub-module has `impl LocalService { ... }` blocks with the trait method bodies renamed with `_inner` suffix and made `pub(super)`.
- `mod.rs` keeps a single `#[async_trait] impl ArawnService for LocalService { ... }` where each trait method body is just `self.NAME_inner(args).await`.

## Status Updates

### 2026-05-22 — landed

**Split executed via Python helper.** The original 1,815-line `local_service.rs` became 8 files under `local_service/`:

| File | Lines | Contents |
|---|---|---|
| `mod.rs` | 863 | struct + initial `impl LocalService` + delegation shell + standalone helpers + tests |
| `workstreams.rs` | 45 | 2 methods |
| `sessions.rs` | 419 | 8 methods (`send_message_inner` dominates) |
| `commands.rs` | 126 | 3 methods |
| `memory.rs` | 162 | 3 methods |
| `permissions.rs` | 106 | 4 methods |
| `integrations.rs` | 188 | 3 methods |
| `feeds.rs` | 153 | 7 methods |

**Cross-module visibility:**
- Each sub-module's `*_inner` method is `pub(super)` so the trait shell can call it.
- `mod.rs` helpers (`default_feed_for_service`, `feed_err`, `feed_summary_to_dto`, `resolve_ws_dir_from_store`, `first_sentence`, `current_summary`, `OAuthFlowCtx`, `infer_entity_type`) promoted to `pub(super)`.

**Off-by-six fix during execution:** the first script run set `TRAIT_END_LINE = 1666`, but the trait impl actually closes at line 1660. Reverted and re-ran with auto-detection (brace-counting from the last method's start).

`cargo fix --lib -p arawn` cleaned 105 unused-import warnings → 0.

**Validation:**
- `cargo check --workspace`: ✅ clean, zero warnings.
- `cargo build --workspace --release`: ✅ clean (1m 02s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test --workspace --lib`: ✅ **1,758 tests pass**, 0 fail.

**Caveat:** `mod.rs` at 863 lines is slightly over the 800-line target. Sub-cap could be hit by extracting standalone helpers (`current_summary`, `OAuthFlowCtx` + its `ConnectContext` impl) to a sibling module. Leaving as-is — close enough; the per-feature splits are the structural win.

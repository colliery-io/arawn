---
id: t-d-split-arawn-src-ws-server-rs
level: task
title: "T-D: Split `arawn/src/ws_server.rs` — RPC dispatch by method-name prefix"
short_code: "ARAWN-T-0400"
created_at: 2026-05-22T01:46:57.447354+00:00
updated_at: 2026-05-22T02:46:41.510189+00:00
parent: ARAWN-I-0054
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0054
---

# T-D: Split `arawn/src/ws_server.rs`

## Parent Initiative

[[ARAWN-I-0054]]

## Direction (decided at task start)

**Scope-trimmed extraction.** The file is structurally trickier than T-C:
- Most arms call `sender.send(...)` inline rather than building a `Response` and returning it.
- `send_message` (line 710) streams events using `sender` + a receiver simultaneously — it cannot be cleanly extracted to a "returns-Response" dispatcher without a larger refactor.

So T-D extracts only the **two largest cohesive blocks** — the ceremonies prefix-match arm and the standalone `handle_todo_rpc` function. The remaining per-method arms (hello, list_workstreams, send_message, feed_*, etc.) stay inline in `mod.rs`. Meaningful reduction, doesn't hit the 800-line cap; the feed_* arms remain as a candidate for future per-arm extraction if needed.

## Acceptance Criteria

- [x] `crates/arawn/src/ws_server.rs` deleted; replaced by `crates/arawn/src/ws_server/` directory.
- [x] Ceremonies prefix-match arm extracted to `ws_server/ceremonies.rs::dispatch`.
- [x] `handle_todo_rpc` extracted to `ws_server/todos.rs::dispatch`.
- [x] Public API surface unchanged.
- [x] `cargo check --workspace` clean.
- [x] `cargo build --workspace --release` clean.
- [x] `cargo test --workspace --no-run` clean.
- [x] All workspace lib tests pass.

## Status Updates

### 2026-05-22 — landed (scope-trimmed)

**Split executed via Python helper** (brace-counting auto-detection for arm/fn boundaries):

| File | Lines | Contents |
|---|---|---|
| `mod.rs` | 1,442 | `handle_connection` + most per-method arms + utilities |
| `ceremonies.rs` | 297 | `dispatch(id, method, params, service, sender)` — handles 14 `ceremonies.*` methods |
| `todos.rs` | 133 | `dispatch(id, method, params, service)` + private `handle_todo_rpc` |

**Net reduction on the hotspot:** `ws_server.rs` was 1,835 lines; `mod.rs` is now 1,442 (-393, -21%). Doesn't hit the 800-line target — but the two cohesive blocks are gone and the file is structurally lighter.

**Patterns used:**
- `ceremonies::dispatch` takes `&mut sender` because the original arm sends responses (and error responses) inline. Function-form parity, not "build Response and return" — that would have required reworking error paths through the outer loop.
- `todos::dispatch` returns `Response` directly (the original `handle_todo_rpc` was already shaped that way). Caller does the `sender.send`.
- Python script extracted arms by locating the `method if method.starts_with(...)` pattern and brace-counting to find the closing `}`. Then patched references: `request.params` → `params` (multi-line regex), `continue;` → `return;` (loop semantics gone), and added the `tracing` macro import that the extracted code needs.

**Cleanups:**
- `cargo fix --lib -p arawn` removed 2 unused imports (`json`, unused tracing macros).
- `mod ceremonies; mod todos;` declared at the top of `mod.rs`.

**Deferred / out of scope:**
- `send_message` streaming refactor: would need to split the receive loop from the send path; postponed to a future initiative if/when ws_server gets further attention.
- Feed_* arms (~120 lines, 7 methods): cohesive enough to extract on their own, but each is small (10–20 lines). Diminishing returns; left inline.

**Validation:**
- `cargo check --workspace`: ✅ clean, zero warnings.
- `cargo build --workspace --release`: ✅ clean (31s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test --workspace --lib`: ✅ **1,758 tests pass**, 0 fail.

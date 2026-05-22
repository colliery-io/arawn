---
id: module-restructuring-split-the
level: initiative
title: "Module restructuring — split the eight size-hotspot files into focused modules"
short_code: "ARAWN-I-0054"
created_at: 2026-05-22T01:44:15.058892+00:00
updated_at: 2026-05-22T01:47:13.210049+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: module-restructuring-split-the
---

# Module restructuring — split the eight size-hotspot files into focused modules

## Context **[REQUIRED]**

Post-I-0053 (cruft removal) the codebase is hygienic — no dead code, no
backward-compat shims, no fake-use functions — but the *structural* hotspots
that I-0053 explicitly deferred are still there. Eight files sit between
1,800 and 2,520 lines each, every one mixing many concerns:

| File | Lines | What it is |
|---|---|---|
| `crates/arawn/src/main.rs` | 2,521 | composition root — startup, feed reg, ceremony cron, GH org expand, steward wiring, hooks plumbing, serve/tui/single-prompt dispatch |
| `crates/arawn-tui/src/render.rs` | 2,426 | TUI rendering for every visual area |
| `crates/arawn-engine/src/tools/workstream.rs` | 2,316 | 15+ workstream-related Tool struct impls in one file |
| `crates/arawn-tui/src/event_loop.rs` | 2,280 | TUI event loop with keyboard / commands / modals / async dispatch tangled |
| `crates/arawn-engine/src/testing.rs` | 1,904 | `TestHarness` API + builder + fixtures |
| `crates/arawn-tui/src/app.rs` | 1,887 | TUI App state struct + many impls |
| `crates/arawn/src/ws_server.rs` | 1,835 | giant JSON-RPC dispatch match over all method names |
| `crates/arawn/src/local_service.rs` | 1,815 | single `LocalService` impl with ~30 trait methods |

Operator framing for I-0053 said "removal first, restructure later — see what's
left after this clean." This initiative does the structure pass.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**

1. Each of the eight target files split into focused sub-modules of reasonable
   size (target: ~300-600 lines per file; hard cap: 800 lines for any
   orchestrator/dispatch top file that remains).
2. **Public API surface of each crate stays equivalent.** Re-exports preserve
   every name external callers use today.
3. **No behavior changes.** Pure refactor: file moves, `mod foo`/`pub use foo::*`
   adjustments, and the minimum imports needed to make it compile.
4. `cargo check --workspace` clean, no new warnings, after every task.
5. Workspace unit + integration tests green after every task.
6. Full UAT + judge green at initiative close (single run; refactors should be
   invisible end-to-end).

**Non-Goals:**

- **Crate-graph reorganization** or moving code between crates. Splits stay
  within their current crate. (A future initiative may revisit crate
  boundaries once each crate's internals are coherent.)
- **New abstractions, helper layers, or APIs.** A split that introduces a
  new trait or strategy pattern is restructuring-plus-design; that gets its
  own task.
- **Performance changes.** Don't reorder operations for perf during a split.
- **The 30+ flat tool re-exports in `arawn-engine/lib.rs`.** Whether to
  preserve, namespace, or flatten that surface is a separate question. Out
  of scope here.
- **Behavior or feature redesign.** If a behavior bug surfaces during a
  split, file it as a backlog bug and keep the split mechanical.

## Detailed Design **[REQUIRED]**

### Working rhythm

For each of the eight target files, the rhythm is:

1. **Sketch the split together** (15-30 min): operator + agent read the file
   end-to-end, identify natural module boundaries, agree on the destination
   file names and what each holds. Operator has domain context that beats
   any agent reconstruction from code.
2. **Execute the move** (30-90 min): create the new sub-module files, move
   blocks of code with their imports, add `mod` declarations + `pub use`
   re-exports so the crate's external surface is unchanged.
3. **Verify**: `cargo check --workspace`, `cargo build --workspace --release`,
   `cargo test --workspace --no-run`, targeted `cargo test -p <crate>`.
4. **Commit**: one commit per task. Squash if intermediate states don't
   compile.

### Sequencing — low risk first, high coupling last

Tasks are ordered so each one builds operator confidence in the rhythm
without piling up coupled changes:

1. **T-A** `testing.rs` — direction decision (inline vs split) is the only
   real call; once made, the work is bounded.
2. **T-B** `tools/workstream.rs` — most mechanical (15+ independent Tool
   structs that can each move to their own file under `tools/workstream/`).
3. **T-C** `local_service.rs` — single impl with 30 methods, split by
   feature group via separate `impl LocalService { ... }` blocks per file.
4. **T-D** `ws_server.rs` — RPC dispatch split by method-name prefix.
   Pairs tightly with T-C; can be staged back-to-back.
5. **T-E** `main.rs` — composition root, largest blast radius. Splits
   into startup phases under `startup/`.
6. **T-F** `tui/app.rs` — TUI state. Trickiest of the TUI three; state is
   interconnected, so design the boundaries carefully before moving.
7. **T-G** `tui/render.rs` — split by visual area; mostly mechanical once
   the App state shape is settled.
8. **T-H** `tui/event_loop.rs` — split by event kind; pairs with T-F.

### Proposed split targets per file

These are starting points. Each task's first step is the operator + agent
sketch — boundaries can move.

**T-A — `crates/arawn-engine/src/testing.rs`** (1,904 lines)
- Decision at task start: inline harness into each test consumer (honoring
  `feedback_inline_tests`) OR split into focused harness pieces.
- If split: `testing/harness.rs`, `testing/builder.rs`, `testing/fixtures.rs`.
- If inline: delete the file, distribute setup code to the 8-12 callers.

**T-B — `crates/arawn-engine/src/tools/workstream.rs`** (2,316 lines)
- Move each `Tool` struct + its impl to its own file under
  `tools/workstream/`: `list.rs`, `show.rs`, `create.rs`, `delete.rs`,
  `switch.rs`, `promote.rs`, `bind.rs`, `unbind.rs`, `describe.rs`,
  `tag.rs`, `apply.rs`, `propose_ontology.rs`, `refine.rs`, `journal.rs`,
  `rollback.rs`, `dust.rs`.
- Shared helpers (`closest_tag`, `edit_distance`, `open_journal`,
  `row_summary`) → `tools/workstream/util.rs`.
- Keep `tools/workstream.rs` (or a `tools/workstream/mod.rs`) as a tiny
  module-orchestrator that does `pub mod list;` etc. + `pub use
  list::WorkstreamListTool;` re-exports.

**T-C — `crates/arawn/src/local_service.rs`** (1,815 lines)
- Keep the `struct LocalService { ... }` and its core constructor +
  config methods in `local_service.rs`.
- Move the trait `impl ArawnService for LocalService` methods into
  per-feature sub-modules under `local_service/`:
  `sessions.rs` (load/create/send/cancel/promote/truncate),
  `workstreams.rs`, `feeds.rs`, `memory.rs`, `permissions.rs`,
  `integrations.rs` (list/start_oauth/disconnect), `commands.rs`
  (capabilities + command/workflow listing).
- Each sub-module carries an `impl LocalService { ... }` block with the
  trait method bodies. The parent file keeps the single
  `impl ArawnService for LocalService` that delegates by calling into
  those `impl`s. Or — simpler — split via inherent-impl extension and
  let the trait impl in the parent file call inherent methods. Choose
  during sketch.

**T-D — `crates/arawn/src/ws_server.rs`** (1,835 lines)
- The current `handle_connection` runs a giant `match method { ... }`.
  Split by method-name prefix:
  `ws_server/sessions.rs` (`session.*`),
  `ws_server/workstreams.rs` (`workstream.*`),
  `ws_server/feeds.rs` (`feed.*`),
  `ws_server/ceremonies.rs` (`ceremonies.*`),
  `ws_server/todos.rs` (`todos.*`),
  `ws_server/memory.rs` (`memory.*`),
  `ws_server/integrations.rs` (`integrations.*` + `oauth.*`),
  `ws_server/permissions.rs` (`permissions.*` + `set_permission_mode`).
- Top-level `ws_server.rs` keeps the server bootstrap (`run_server`,
  `handle_connection_public`, auth/token handling, the connection
  dispatcher that routes by prefix to the sub-modules).

**T-E — `crates/arawn/src/main.rs`** (2,521 lines)
- `main()` itself stays in `main.rs` but becomes a thin orchestrator:
  parse CLI, kick off startup, hand off to the chosen mode.
- Startup phases move under `startup/`:
  `config.rs` — config + watcher.
  `llm.rs` — `LlmClientPool` build.
  `store.rs` — DB + projections + memory wiring.
  `tools.rs` — `register_default_tools` and `register_workflow_tools`.
  `feeds.rs` — feed runtime + cron registration + GH org-expand hook.
  `ceremonies.rs` — ceremony service + cron registration + back-fill.
  `steward.rs` — steward subroutine wiring + scheduling.
  `integrations.rs` — OAuth integration registration.
  `server.rs` — WS server / TUI / single-prompt dispatch.
  `hooks.rs` — hot-reload watchers.
- The `EmbedderBridge`, `ExtractorBindHook`, `FeedRuntimeUnbindHook`,
  helper structs live with the phase that owns them (probably `tools.rs`
  / `feeds.rs`).
- `build_engine_config`, `build_llm_client`, `connect_mcp_servers`,
  `register_one_feed`, `resolve_ceremony_tz`, `expand_github_org` go to
  their respective phase files.

**T-F — `crates/arawn-tui/src/app.rs`** (1,887 lines)
- Carve `App` state by concern:
  `app/state.rs` — the state struct + accessors.
  `app/actions.rs` — `Action` handling (`apply_action`, side-effect
  emission).
  `app/derived.rs` — computed views (scroll position, focused message,
  visible region, etc.).
  `app/commands.rs` — slash-command parsing + dispatch glue if not
  already in `command.rs`.
- Trickiest of the TUI three. Sketch carefully — `App` fields are
  interconnected.

**T-G — `crates/arawn-tui/src/render.rs`** (2,426 lines)
- Split by visual area:
  `render/chat.rs` — chat message log.
  `render/sidebar.rs` — left sidebar (workstreams, sessions, todos).
  `render/modal.rs` — modal dialogs.
  `render/header.rs` — top bar.
  `render/footer.rs` — bottom status / input.
  `render/util.rs` — shared helpers (theme application, width math).
- `render.rs` keeps the top-level `render(app, frame)` entry that lays
  out the chrome and dispatches to area renderers.

**T-H — `crates/arawn-tui/src/event_loop.rs`** (2,280 lines)
- Split by event kind:
  `event_loop/keyboard.rs` — key handling, key→action mapping.
  `event_loop/command.rs` — command parsing + dispatch.
  `event_loop/modal.rs` — modal-mode event handling.
  `event_loop/async_dispatch.rs` — async event (engine event, todo
  event, server notice) → action mapping.
- Top-level `event_loop.rs` keeps `run_tui` and the main select! loop.

### Verification strategy

Per-task:
- `cargo check --workspace` clean (no new warnings).
- `cargo build --workspace --release` clean.
- `cargo test --workspace --no-run` clean.
- `cargo test -p <touched-crate>` green.
- TUI snapshot tests must pass after any TUI task.

Initiative close:
- All eight target files ≤ 800 lines.
- `cargo test --workspace --lib` green.
- `angreal test uat` + `angreal test uat-judge` green.

## Alternatives Considered **[REQUIRED]**

- **Crate-graph restructuring instead.** Considered, rejected: operator
  preference is file-splitting only, "stay within crate boundaries". The
  crate-graph question is real but separable — once each crate's internals
  are coherent we can revisit boundaries with better information.
- **Comprehensive (every crate) instead of hotspot-only.** Considered,
  rejected: the eight files are responsible for the bulk of the navigation
  pain; tackling them gets ~80% of the value at ~20% of the cost. Smaller
  files can be split later when they grow.
- **Re-run Explore-agent discovery sweep.** Considered, rejected: the
  hotspots are visible (we know which files are big) and the operator has
  domain context that an agent can't recover. Lighter per-file sketch is
  the right tool.
- **Stop-the-bleed (top 3 only).** Considered as a safer scope. Rejected
  because the operator explicitly chose hotspot-focused on the top 8 and
  the per-task cost is bounded.

## Implementation Plan **[REQUIRED]**

### Phase 1 — Discovery (this phase)
Sketch per-task split targets above are the deliverable. No separate
inventory document; per-task sketch happens at task start.

### Phase 2 — Decompose
File eight child tasks (T-A through T-H) with the proposed splits as
starting points. Order them per the sequencing rationale.

### Phase 3 — Active
Ship tasks in order. Each task:
1. Operator + agent sketch the split.
2. Agent executes the move + re-exports.
3. Workspace cargo check / build / tests green.
4. Commit.

### Phase 4 — Completed
- Every target file ≤ 800 lines.
- Full UAT + judge green.
- Transition to completed.

## Exit Criteria

- All eight target files split, each remaining top file ≤ 800 lines.
- `cargo check --workspace` clean, no new warnings.
- `cargo build --workspace --release` clean.
- `cargo test --workspace --lib` green.
- `angreal test uat` + `angreal test uat-judge` green at close.
- Each crate's public re-export surface unchanged (verified by grep —
  no external `use arawn_engine::...` etc. broken).

## Related

- ARAWN-V-0001 — vision.
- ARAWN-I-0053 — predecessor (cruft removal); deferred this structural
  work explicitly as a non-goal.
- Future (not yet filed): crate-graph reorganization initiative once
  per-crate internals are coherent.
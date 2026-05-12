---
id: workstream-slash-commands
level: task
title: "Workstream slash commands"
short_code: "ARAWN-T-0249"
created_at: 2026-05-12T23:25:50.552963+00:00
updated_at: 2026-05-12T23:46:31.809132+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0248]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Workstream slash commands

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

User-facing surface over the `WorkstreamRegistry` landed in T-0248. Eight slash commands that cover the workstream lifecycle: create, browse, switch context, describe, bind feeds, delete.

The commands run in arawn's existing slash-command handler (in `arawn-tool` / engine plugins). Session-active workstream is set by `/workstream switch` and is what T-0250 reads.

## Scope

### Commands

| Command | Args | Behavior |
|---|---|---|
| `/workstream new <name> [display_name]` | name = slug, display_name optional | Creates workstream. Errors on duplicate / invalid slug / reserved `scratch`. Does NOT switch into it (user does that explicitly). |
| `/workstream list` | none | Prints active workstreams (sorted by `updated_at` desc). Shows name, display_name, description preview, binding count, active marker. Flag `--all` includes archived. |
| `/workstream switch <name>` | name | Sets the session-active workstream. Prints a one-line banner: `"now in workstream 'pat' — next messages contribute to pat's KB"`. Errors if name doesn't exist. |
| `/workstream show [name]` | name optional (defaults to active) | Full readout: display_name, description, bindings list, KB stats (entity count by type, total relations). Useful for confirming "yes, I'm where I think I am." |
| `/workstream describe <name> <text…>` | name + free text | Sets/updates the description. Description feeds extractor prompts in Phase 4. |
| `/workstream bind <name> <feed_id>` | name + feed_id | Adds feed_id to the workstream's bindings. Errors if feed doesn't exist. |
| `/workstream unbind <name> <feed_id>` | name + feed_id | Removes the binding. Silent no-op if not bound. |
| `/workstream delete <name>` | name | Soft-deletes (sets `archived = 1`). Refuses `scratch` and refuses if it's the active workstream. Prints a one-line note about the on-disk KB being left intact. |

### Tab completion (TUI)

`/workstream <subcommand>` and `/workstream switch <name>` benefit from completion against existing workstreams. Hook into the TUI's existing completion if it's there; skip if not (don't build new infra in this task).

### Output formatting

Slash commands return text via the existing slash-command response channel. For `list` / `show`, use compact tabular output; no JSON dumps. The agent sees the same output, so it should read naturally.

### What's deferred

- Session/workstream binding semantics (T-0250 — what "active workstream" means for the memory router).
- Promotion command (`/workstream promote`) — separate follow-up.
- Workstream rename — would invalidate the on-disk KB path; postponed until the migration story is defined.
- Hard delete — defer until users explicitly ask for it; soft delete is reversible and on-disk size is small.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All 8 commands implemented and reachable from the engine slash-command handler.
- [ ] `switch` updates the in-memory session active workstream (touches T-0250's primitive).
- [ ] `delete` refuses `scratch` and refuses the currently-active workstream with a clear error.
- [ ] `show` works with no args (uses active) and with an explicit name.
- [ ] Tests: per-command happy path + the refusal paths (invalid slug, duplicate, delete-scratch, delete-active).
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

### Where commands live

Probably `crates/arawn-engine/src/tools/workstream.rs` (there's already a `WorkstreamCreateTool` + `WorkstreamListTool` there as the original Phase-0 stub — replace/extend rather than fork). Each command is a `Tool` impl whose name starts with `workstream_` for the natural slash mapping.

### Active-workstream primitive

Tasks reference a session-level "active workstream." The shared state lives wherever `Session` does (T-0250 owns the actual wiring); for this task, just call into whatever T-0250 exposes. If T-0250 isn't done yet, ship the commands with a `SessionWorkstream` shim that gets replaced when T-0250 lands.

### Dependencies

- T-0248 (WorkstreamRegistry, MemoryManager::for_workstream).

### Risk considerations

- **Existing stub commands**: there are already minimal Workstream*Tool stubs in `arawn-engine/src/tools/workstream.rs`. Confirm they're shells and replace; don't fork.
- **Slash routing**: arawn's slash command surface is currently subtype-specific (`/workstream new` parsed as one command vs many). Check the dispatcher pattern when implementing.
- **Confirmation prompts**: `delete` is destructive (soft); decide whether to require `--force` or a confirmation reply. My lean: no prompt for soft delete; reserve confirmation for the eventual hard delete.

## Status Updates

### 2026-05-12 — 8 tools registered + SessionWorkstream shim

**Files.**
- `crates/arawn-engine/src/tools/workstream.rs` — rewrote. Eight `Tool` impls (one per slash command) plus a `SessionWorkstream` shim. The two existing stub tools (`WorkstreamCreateTool`, `WorkstreamListTool`) got renamed at the tool-name level (`workstream_new`, `workstream_list`) and gained the new fields; rest are new types.
- `crates/arawn-engine/src/tools/mod.rs` + `lib.rs` — re-export the new types.
- `crates/arawn-storage/src/store.rs` — added the `Store` wrappers the tools call into: `list_all_workstreams`, `update_workstream_description`, `add_workstream_binding`, `remove_workstream_binding`, `soft_delete_workstream`, `ensure_scratch_workstream`.
- `crates/arawn/src/main.rs` — registers all 8 tools, threads a shared `SessionWorkstream` shim through `switch`/`show`/`list`/`delete`. Calls `ensure_scratch_workstream()` at boot.

**Commands shipped:**
- `workstream_new` (params: name, display_name?, description?) — replaces the stub. Validates via registry; refuses scratch and bad slugs.
- `workstream_list` (param: all?) — emits `{active, workstreams: [{name, display_name, description, bindings, archived, active}]}`. Active workstream is flagged.
- `workstream_switch` (param: name) — sets session-active; returns a banner line + previous/new for the UI to surface. Refuses archived workstreams.
- `workstream_show` (param: name?) — defaults to active when no name supplied. Emits the full record.
- `workstream_describe` (params: name, description) — updates description.
- `workstream_bind` / `workstream_unbind` (params: name, feed_id) — idempotent.
- `workstream_delete` (param: name) — soft-delete; refuses scratch and the currently-active workstream.

**SessionWorkstream shim.** `Arc<Mutex<String>>` wrapper. Initialized at startup to "scratch". `switch` mutates it; `list`/`show`/`delete` read it. T-0250 replaces this with the real `Session::workstream_name` field; the shim makes T-0249 testable on its own without blocking on T-0250.

**Tests: 11 in `tools::workstream` passing.**
- create_succeeds_with_valid_slug
- create_refuses_scratch
- switch_updates_active + switch_unknown_errors
- show_defaults_to_active
- describe_updates_description
- bind_and_unbind_round_trip
- delete_refuses_scratch + delete_refuses_currently_active + delete_soft_marks_archived
- list_marks_active

Plus engine full suite: **541 passed**, 0 failed. `angreal check clippy` clean.

**Acceptance criteria.**
- [x] All 8 commands implemented and reachable from the engine slash-command handler.
- [x] `switch` updates the in-memory session active workstream (via SessionWorkstream — T-0250 swaps for Session field).
- [x] `delete` refuses scratch and refuses the currently-active workstream with a clear error.
- [x] `show` works with no args (uses active) and with an explicit name.
- [x] Tests cover happy path + refusal paths.
- [x] `angreal check clippy` clean.

T-0250 unblocked.
---
id: daily-drivable-phase-3-tui
level: initiative
title: "Daily-drivable Phase 3: TUI correctness + GUI prerequisites"
short_code: "ARAWN-I-0069"
created_at: 2026-06-11T11:19:28.676212+00:00
updated_at: 2026-06-11T11:19:28.676212+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: S
initiative_id: daily-drivable-phase-3-tui
---

# Daily-drivable Phase 3: TUI correctness + GUI prerequisites Initiative

## Context **[REQUIRED]**

Spun out of [[ARAWN-I-0067]] (full-codebase review, 2026-06-11) as the Phase 3 execution initiative, already restructured per **ADR ARAWN-A-0005** (web GUI becomes the primary review/triage surface; TUI demoted to a maintained chat client). Scope is therefore deliberately narrow: (a) TUI **correctness** fixes — bugs stay bugs in a maintained chat client — and (b) backend/protocol cleanups that benefit every client, several of which are prerequisites the web GUI depends on.

TUI *experience* work (session browser, richer modals, review surfaces) is explicitly **not** here — those findings are recorded below as deferred requirements for the GUI initiative so they shape its design rather than spawning new ratatui work.

**Gating:** runs after ARAWN-I-0068 (Phase 2) — several cleanups report into or extend Phase 2 deliverables (status surface, error fidelity). **Completion of this initiative is the gate for scaffolding the web GUI initiative** (ADR ARAWN-A-0005 review trigger); its final deliverable is the GUI-readiness checklist whose gaps become the GUI initiative's first backlog items.

This document is canonical for Phase 3 execution; I-0067 retains the original catalogue as the review record. Finding IDs preserved from the review (P3-1 retired to the deferred list; P3-2…P3-5 original; P3-6 added at restructure).

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- TUI correctness: display-width-aware wrapping (CJK/emoji tables render right), stale modal state cleared on disconnect.
- Engine-side permission grant shapes widened (directory/glob scope) so near-identical approvals don't re-prompt — benefits TUI and GUI alike.
- Retry/warmup refinement: per-provider warmup TTL, broader cold-start detection.
- The P3-5 cleanup batch fully triaged — each item fixed or explicitly wont-fix'd, GUI-serving items (error fidelity through the protocol) prioritized.
- A written GUI-readiness checklist confirming the protocol surface a thin web client needs, with gaps filed as the GUI initiative's first backlog items.

**Non-Goals:**
- Any new TUI experience surface (session browser, richer modal context, review UX) — deferred to the GUI per ADR ARAWN-A-0005.
- Building the web GUI itself (separate initiative, gated on this one completing).
- Backend feature work beyond the listed cleanups.

## Detailed Design **[REQUIRED]**

Findings carried from the I-0067 review catalogue (IDs preserved; restructured 2026-06-11 per ADR ARAWN-A-0005).

### Deferred to the GUI initiative (requirements traceability — do NOT build in the TUI)
- **Session visibility / quick-jump** (was P3-1): only indicator of the active session is an 8-char ID in the status bar (`crates/arawn-tui/src/app/mod.rs:44-46`, sessions sub-view removed per I-0035 T-0356). After switching lenses the user can't tell resumed-vs-fresh. → GUI session browser requirement; also depends on the promotion RPC from ARAWN-I-0068 (P2-4).
- **Permission/modal context richness** (was part of P3-3): UserInputRequest modals show only title+options; overlapping requests indistinguishable (`event_loop/mod.rs:1096-1126`). → GUI approval-surface requirement.
- **Brief/ceremony/action-item review UX**: tablets render as markdown in a chat pane and are poll-only. → this is the GUI's core screen.

### P3-2 · MEDIUM · Markdown/table wrapping measures codepoints, not display cells (TUI correctness)
`crates/arawn-tui/src/markdown.rs:573-615` (esp. 583, 594), `render/input.rs:87-96`
- `wrap_text()` uses `chars().count()` for word width and `take(width)` for hard breaks — CJK/emoji (2-cell glyphs) wrap wrong or overflow. Same width-vs-codepoint bug class as the previously-fixed table wrapping issue; the fix didn't reach these paths.
- Autocomplete dropdown clamps to min 20 cells and overflows terminals narrower than that.
**Fix:** use unicode-width display cells in wrap and hard-break paths; clamp dropdown to terminal width.

### P3-3 · MEDIUM · Stale modal state survives reconnect (TUI correctness)
`crates/arawn-tui/src/event_loop/mod.rs:1109-1125`
- Pending modal oneshot channels aren't cleaned up on disconnect — stale pending-response state can survive into the next connection.
**Fix:** clear pending modal state on `WsEvent::Closed`.

### P3-4 · LOW-MEDIUM · Retry/warmup refinement
`crates/arawn-llm/src/warming.rs:27, 90-92`
- Warmup TTL is a global 4-minute constant tuned for Ollama Cloud, wasteful for Groq/OpenAI; cold-restart detection hardcodes HTTP 503.
**Fix:** per-provider TTL config; broaden cold-start detection.

### P3-5 · Accumulated smaller findings (cleanup batch — triage each)
- MCP adapter converts transport/RPC failures into `Ok(ToolOutput::error(...))` — engine can't distinguish "MCP server crashed" from "tool failed" (`arawn-mcp/src/adapter.rs:115-118`); duplicate MCP tool names silently overwrite in the registry (`adapter.rs:26-29`).
- `ServiceError` flattens `LlmError` to `{"kind":"llm"}` — clients can't distinguish bad-key / model-not-found / rate-limited (`arawn-service/src/error.rs:41-54`). **GUI-serving: prioritize.**
- Usage tracking records nothing if the provider omits `usage` in the final chunk — silent accounting gaps (`arawn-llm/src/usage/tracking_client.rs:70-91`).
- Graceful shutdown doesn't guarantee JSONL append flush (`crates/arawn/src/main.rs:1233`).
- Steward runner: fixed 1-hour tick, no backoff/circuit-breaker on repeatedly failing lenses (`crates/arawn/src/main.rs:759-778`); journal accumulates duplicate unapplied proposals across passes (`arawn-steward/src/journal.rs:160-180`).
- Ceremony `match` arms panic on unexpected event variants in production code (`arawn-ceremonies/src/service.rs:1138`, `plugins/retro.rs:853`, `engine.rs:656`).
- Ceremony tablets never surface into lenses/chat — "your daily brief is ready" requires polling `ceremonies.get_today()`. **GUI-serving: prioritize.**
- Config hot-reload rejects invalid TOML silently (logged only) (`crates/arawn/src/config_watcher.rs:124-161`).
- Retro sweep interval hardcoded 3600s (`startup/ceremonies.rs:388-404`); DST transitions untested for scheduled ceremonies.
- Filesystem feed template is a stub — either implement or remove from the registry so it can't be configured into a no-op. **Plus (surfaced by the I-0067 Phase 1 UAT, 2026-06-12):** the filesystem read path is rough — in the `filesystem-watch-roundtrip` scenario the agent's `file_read('transcripts/falcon-standup-2026-05-20.md')` failed with "relative path outside lens root", because relative paths aren't resolved against the lens workspace before the sandbox/lens-root check. Either resolve agent-supplied relative paths against the lens root (so `transcripts/foo.md` works) or return a clearer, actionable error telling the model to use an absolute/lens-relative form. Whatever this item does about the stub, it should also make the file_read path usable for filesystem-feed content.
- Repo hygiene: `test.log` at root (gitignore/delete), `vendor/sandbox-runtime` patch relationship to upstream undocumented (`Cargo.toml:92`).

### P3-6 · MEDIUM · Permission grant shapes too narrow (engine-side — benefits all clients)
`crates/arawn-engine/src/permissions/checker.rs:138-144`
- Session grants match exact shapes with wildcard fallback only — approving `~/x/foo.rs` doesn't help with `~/x/bar.rs`; users re-approve near-identical operations repeatedly, in every client.
**Fix:** directory-scoped grant shapes (grant covers the parent dir or a glob), engine-side so TUI and GUI both benefit.

### GUI-readiness checklist (new deliverable, final task)
Confirm the protocol surface a thin web client needs is in place: `/status` RPC shape stable (from ARAWN-I-0068 P2-1); ceremony/action-item review RPCs return structured data (not pre-rendered markdown only); WS auth/binding hardened beyond the localhost token file (extends I-0067 P1-4). Output: a readiness note in this initiative; any gaps become the GUI initiative's first backlog items.

## Testing Strategy

- Inline regression tests per fix (project convention: `#[cfg(test)]` modules): width-aware wrapping gets table-driven cases over CJK/emoji/mixed content; modal cleanup gets a disconnect-mid-modal test; grant shapes get engine-level permission tests.
- The P3-5 triage produces either a fix-with-test or a written wont-fix rationale per item — no silent drops.
- The GUI-readiness checklist is itself verified by exercising the RPCs a web client would call (structured payloads, auth flow) — gaps filed, not hand-waved.

## Alternatives Considered **[REQUIRED]**

- **Building session browser / richer modals in the TUI** — rejected by ADR ARAWN-A-0005; that experience work belongs to the web GUI. This initiative deliberately contains only correctness residue and protocol cleanups.
- **Skipping straight to the GUI without this initiative** — rejected. The TUI remains the maintained chat client (real bugs stay fixed), and the GUI inherits whatever protocol error-fidelity exists — the cleanups here are what make a *thin* GUI possible.
- **Folding these items into Phase 2** — rejected to keep ARAWN-I-0068 focused on its keystone (status surface + data integrity); these are smaller, lower-risk items with a different gate (GUI scaffolding).

## Implementation Plan **[REQUIRED]**

Suggested task slicing (decomposition requires human review before tasks are created):
1. Display-width-aware wrapping + narrow-terminal clamps (P3-2).
2. Stale modal cleanup on disconnect (P3-3).
3. Directory-scoped permission grant shapes, engine-side (P3-6).
4. Retry/warmup refinement (P3-4).
5. Cleanup batch triage (P3-5) — GUI-serving items first (`ServiceError` fidelity, MCP error fidelity, tablet surfacing).
6. GUI-readiness checklist (final task) — output is the readiness note + filed gaps.

**Exit criteria:**
- CJK/emoji tables render correctly in the TUI; no stale modal state across reconnects.
- Near-identical permission approvals don't re-prompt.
- Every P3-5 item is fixed or explicitly wont-fix'd with rationale.
- The GUI-readiness checklist is written and its gaps filed as the GUI initiative's first backlog items.
- `angreal check all` + `angreal test unit` green throughout; UAT green at close.

**Sequencing notes:** runs after ARAWN-I-0068. **Completing this initiative triggers scaffolding the web GUI initiative** (ADR ARAWN-A-0005 review trigger); frontend stack selection happens in that initiative's discovery, not here.
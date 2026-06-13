---
id: p3-7-gui-readiness-checklist
level: task
title: "P3-7: GUI-readiness checklist — verify the protocol surface a thin web client needs"
short_code: "ARAWN-T-0490"
created_at: 2026-06-13T14:43:34.110216+00:00
updated_at: 2026-06-13T15:32:55.220127+00:00
parent: ARAWN-I-0069
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0069
---

# P3-7: GUI-readiness checklist — verify the protocol surface a thin web client needs

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0069]] — the **gating deliverable**: completing this (and the rest of I-0069) triggers scaffolding the separate web-GUI initiative (ADR ARAWN-A-0005).

## Objective **[REQUIRED]**

Verify the protocol surface a *thin* web client needs is actually in place, and turn any gaps into the GUI initiative's first backlog items. Output is a written readiness note in this task.

### Type
- [x] Chore — verification + planning gate

### Priority
- [x] P2 - Medium (the gate; do last)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] **`/status` + `/health` shape stable** (from I-0068 P2-1) — documented as the contract a web client renders.
- [ ] **Review RPCs return structured data, not pre-rendered markdown only** — ceremony tablets / action items consumable by a non-TUI client (depends on the P3-5 tablet-surfacing item).
- [ ] **Error fidelity through the protocol** — typed `ServiceError`/`LlmError` reach the client (depends on the P3-5 error-fidelity item).
- [ ] **WS auth/binding** reviewed beyond the localhost token file (extends I-0067 P1-4) — documented as adequate or filed as a GUI gap.
- [ ] **Session/promotion + permission-approval RPCs** the GUI needs (session list/promote from I-0068 P2-4; approval surface) confirmed present or filed.
- [ ] A **GUI-readiness note** written enumerating what's ready + each gap filed as a backlog item for the GUI initiative.
- [ ] Verified by exercising the RPCs a web client would call (structured payloads, auth) — not hand-waved.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Audit the WS-RPC surface against "what a thin web review/triage client calls": status/health, session list+load+promote, ceremony/action-item read (structured), permission mode/approval, feeds/lenses read. For each: confirmed-ready or gap-filed. The deliverable is the note + filed backlog items, not code (code gaps become GUI-initiative tasks).

### Dependencies
Last task of I-0069. Depends on the P3-5 GUI-serving items (tablet surfacing, error fidelity) and the I-0068 status/promotion surface.

### Risk Considerations
The point is honesty: a gap hand-waved here becomes a nasty surprise mid-GUI-build. Exercise the actual RPCs.

## Status Updates **[REQUIRED]**

**2026-06-13 — GUI-readiness audit complete (verified by exercising the RPCs, not hand-waved).**

The WS-RPC surface (`crates/arawn/src/ws_server/mod.rs:30` — 58 allowlisted methods) was audited against "what a thin web review/triage client calls." Verification = the `arawn-tests/tests/websocket.rs` (17) + `local_service.rs` (18) integration suites, which exercise the real RPCs over a live WS connection with the JSON response envelope. All 35 green.

### Ready — the contract a web client can build on

- **`health` / `status`** — structured + **versioned** (`SystemStatus.version` = `SYSTEM_STATUS_VERSION`, T-0476). Per-subsystem fields (feeds, ceremonies, embedding/extraction, LLM, steward). Round-trip covered by `health_and_status_rpc_round_trip`. **This is the render contract** a web dashboard consumes.
- **Sessions** — `list_sessions` / `create_session` / `load_session` / `promote_session` / `truncate_session_at_user_message`, all structured (serde DTOs), covered by `create_and_load_session` + `list_sessions_via_ws`. Promotion (I-0068 P2-4) confirmed present.
- **Permission control + approval** — `get_permission_mode` / `set_permission_mode` (tested: `get_and_set_permission_mode_via_ws`), `get_permissions_status` (audit surface), and the **interactive approval loop**: engine emits a `UserInputRequest` event, client replies via the `user_input_response` RPC. Present and wired.
- **Review data is structured, not markdown** — every ceremonies/todos read RPC serializes a typed struct via `serde_json::to_value(&dto)` (`ws_server/ceremonies.rs`, `todos.rs`): `ceremonies.get_by_period`, `list_items`, `list_priorities`, `get_diary`, `list_notifications`, `todos.*`. A non-TUI client renders these itself. (AC satisfied; depended on the P3-5 tablet-surfacing work, which was already in place.)
- **Error fidelity through the protocol** — error envelope is `{code, message, details}` (`Response::from_service_error`), and `details` now carries the typed sub-kind incl. `llm_kind` + `retry_after_secs` (fixed in T-0489). Covered by `from_service_error_preserves_structured_detail_for_typed_variants`.
- **Tablet-ready push** — `briefing_ready` ServerNotice on `TabletGenerated` (not poll-only); broadcast over the same WS notice channel a web client subscribes to.
- **Streaming** — `send_message` streams engine-event envelopes (covered by `send_message_streams_complete_event`, `..._with_tool_call_streams_events`); browser-friendly JSON text frames.
- **Lenses / feeds / integrations reads** — `list_lenses`, `feed_list`/`feed_templates`/`feed_schema`/`feed_discover`, `list_integrations` all present + structured.

### Gaps — seed backlog for the web-GUI initiative (ADR ARAWN-A-0005)

These are NOT blockers to scaffolding the initiative; they are its first tasks. Recorded here (the initiative doesn't exist yet) to be created with it:

1. **GUI-G1 — Auth beyond the localhost token file (P1).** Today auth is a single `server.token` bearer file checked at WS upgrade (`ws_server/mod.rs:400`), plus a startup warning on non-loopback binds ("arawn has no auth layer"). A remotely-served web GUI needs: a non-file credential story (the token is plaintext on local disk), per-client identity if multi-user is ever in scope, and TLS termination guidance. Extends I-0067 P1-4.
2. **GUI-G2 — Browser origin/CORS + WS `Origin` validation (P1).** A cross-origin browser client needs CORS headers and WS `Origin` checking; neither exists today. Without it the server either rejects browsers or accepts any origin (CSRF-style risk once auth is cookie/header-based).
3. **GUI-G3 — Machine-readable protocol schema (P2).** `RPC_METHODS` is a Rust `const` + per-handler param parsing; there's no OpenRPC/JSON-Schema a web client can codegen types from. Publishing one (and versioning it alongside `SYSTEM_STATUS_VERSION`) would prevent client/server drift.
4. **GUI-G4 — Documented streaming-event envelope (P2).** The engine-event/notice envelope shapes are stable and tested but only defined in Rust (`arawn-service` types + TUI parsers). Document them as the streaming contract so a web client doesn't reverse-engineer from the TUI.
5. **GUI-G5 — TLS / reverse-proxy deployment note (P3).** The server binds plain TCP; a web GUI deployment needs a documented TLS-terminating reverse-proxy story (or native TLS).

### Verdict
The **data-plane** surface a thin web review/triage client needs (status, sessions+promotion, structured ceremony/todo reads, typed errors, permission control + approval, streaming, push notices) is **present, structured, and exercised**. The gaps are all **edge/transport concerns** (auth, CORS, TLS, schema/doc publishing) — exactly what a web-GUI initiative should own as its first slice. I-0069's correctness work has cleared the path; ADR A-0005's GUI initiative can be scaffolded with GUI-G1…G5 as its opening backlog.
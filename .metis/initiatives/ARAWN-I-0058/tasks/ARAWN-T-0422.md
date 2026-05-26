---
id: t-b-feed-schema-rpc-ws-client
level: task
title: "T-B: feed_schema RPC + ws_client wiring"
short_code: "ARAWN-T-0422"
created_at: 2026-05-26T17:28:03.896868+00:00
updated_at: 2026-05-26T18:34:40.206905+00:00
parent: ARAWN-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0058
---

# T-B: feed_schema RPC + ws_client wiring

## Parent Initiative

[[ARAWN-I-0058]]

## Objective

Expose each template's `param_schema()` (from [[ARAWN-T-0421]]) to the TUI over
a new `feed_schema` RPC, so the modal can fetch the form definition for a chosen
template. Kept separate from `feed_discover` to keep "what params exist" (schema)
distinct from "what values are pickable" (discovery).

## Type
Feature — `arawn-service`, RPC dispatch, `arawn-tui/ws_client`.

## Technical Approach

1. **Service method**: add `ArawnService::feed_schema(template: &str)` returning
   the template's `Vec<ParamSpec>` (plus the template's default cadence, so the
   modal can pre-fill the advanced field without a second call). Look the
   template up in the same registry `feed_register`/`feed_discover` use.
2. **DTO**: a `FeedSchemaDto { template, params: Vec<ParamSpec>, default_cadence }`
   in `arawn-service/src/types.rs`. `ParamSpec` is reused from `arawn-feeds`
   (re-exported / serde-shared) rather than redefined.
3. **RPC wiring**: register a `feed_schema` method in the ws_server dispatch
   (alongside `feed_discover` / `feed_register`).
4. **Client**: `WsClient::feed_schema(template)` mirroring `feed_discover`
   (`ws_client.rs`).
5. **Unknown template** → structured error the modal can show, not a panic.

## Acceptance Criteria

## Acceptance Criteria

- [x] `feed_schema(template)` returns the template's param schema + default
      cadence; unknown template returns a clean error (via `feed_err` mapping of
      `FeedError`, like `feed_discover`).
- [x] `FeedSchemaDto` serializes/deserializes. **Deviation:** `ParamSpec` is
      *mirrored* as `FeedParamSpecDto`/`FeedParamKindDto` in `arawn-service`, not
      reused — `arawn-service` deliberately doesn't depend on `arawn-feeds`
      (same pattern as `FeedSummaryDto` mirrors `FeedSummary`). The `arawn` crate
      maps between them in `param_spec_to_dto`. `arawn-tui` depends on
      `arawn-service`, not `arawn-feeds`, so this keeps the layering intact.
- [x] RPC method `feed_schema` registered in ws_server dispatch + methods list;
      `WsClient::feed_schema` added (returns raw JSON `result`, like the other
      feed_* client methods).
- [x] Test asserts the filesystem/folder schema DTO shape (keys root/recursive/
      include/exclude, root=Path+required) and default cadence `*/15 * * * *`,
      plus a mapping test covering every `ParamKind` → DTO variant incl. Enum.
- [x] `feed_discover` and `feed_register` untouched.
- [x] `angreal check workspace` clean; `cargo test -p arawn` feeds tests pass.

## Dependencies
Depends on [[ARAWN-T-0421]] (`ParamSpec`/`param_schema()` must exist). Blocks
[[ARAWN-T-0424]] (modal needs the data to render).

## Risk Considerations
- Keep the schema DTO a thin pass-through of `arawn-feeds` types to avoid a
  second source of truth.

## Status Updates

**2026-05-26 — Implemented + tested.**
- `arawn-feeds/runtime.rs`: added `FeedRuntime::template_schema(name) -> (Vec<ParamSpec>, cadence)`
  — registry lookup + `param_schema()` + `defaults(empty).cadence`.
- `arawn-service`: added mirror DTOs `FeedParamKindDto`, `FeedParamSpecDto`,
  `FeedSchemaDto` (types.rs + lib.rs exports) and the trait method
  `feed_schema(&str) -> FeedSchemaDto`.
- `arawn` (local_service): `feed_schema_inner` + trait shell delegate;
  `param_spec_to_dto` maps feeds→service types. ws_server: `feed_schema`
  dispatch arm + methods-list entry.
- `arawn-tui/ws_client.rs`: `WsClient::feed_schema(template)`.
- Tests in `local_service/feeds.rs`: every-ParamKind mapping (incl. Enum) +
  filesystem schema DTO shape & default cadence.

**Deviation:** mirrored `ParamSpec` in arawn-service rather than reusing the
feeds type — arawn-service doesn't depend on arawn-feeds (consistent with
`FeedSummaryDto`); arawn-tui depends on arawn-service, not arawn-feeds. Keeps
the layering clean; the `arawn` crate owns the mapping.
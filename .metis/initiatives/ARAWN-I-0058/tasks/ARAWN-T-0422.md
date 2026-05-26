---
id: t-b-feed-schema-rpc-ws-client
level: task
title: "T-B: feed_schema RPC + ws_client wiring"
short_code: "ARAWN-T-0422"
created_at: 2026-05-26T17:28:03.896868+00:00
updated_at: 2026-05-26T17:28:03.896868+00:00
parent: ARAWN-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] `feed_schema(template)` returns the template's param schema + default
      cadence; unknown template returns a clean error.
- [ ] `FeedSchemaDto` serializes/deserializes; `ParamSpec` is shared with
      `arawn-feeds` (not duplicated).
- [ ] RPC method registered and reachable; `WsClient::feed_schema` added.
- [ ] A service-level test fetches the schema for `filesystem/folder` and asserts
      the expected param keys + default cadence come back.
- [ ] `feed_discover` and `feed_register` behavior unchanged.
- [ ] `angreal check workspace` + tests pass.

## Dependencies
Depends on [[ARAWN-T-0421]] (`ParamSpec`/`param_schema()` must exist). Blocks
[[ARAWN-T-0424]] (modal needs the data to render).

## Risk Considerations
- Keep the schema DTO a thin pass-through of `arawn-feeds` types to avoid a
  second source of truth.

## Status Updates

*To be added during implementation*
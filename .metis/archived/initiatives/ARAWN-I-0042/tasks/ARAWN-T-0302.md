---
id: priorities-service-rpc-methods
level: task
title: "Priorities service + RPC methods (confirm/reject/add/list)"
short_code: "ARAWN-T-0302"
created_at: 2026-05-16T16:37:55.787456+00:00
updated_at: 2026-05-16T16:50:43.763246+00:00
parent: ARAWN-I-0042
blocked_by: [ARAWN-T-0301]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0042
---

# Priorities service + RPC methods

## Parent Initiative

[[ARAWN-I-0042]]

## Objective

The weekly plugin emits priority *candidates* as `ceremony_items`.
The Monday confirmation flow needs a separate code path that
writes rows into `ceremony_priorities` so daily's `alignment`
gather (and retro's `priority_completion_ratio` detector) can
read them. This task adds the service methods + WS-RPC surface
for that flow.

## Acceptance Criteria

## Acceptance Criteria

- [ ] New methods on `CeremonyService` (`arawn-ceremonies/src/service.rs`):
      - `confirm_priority(item_id: &str) -> Result<PriorityDto, CeremonyError>`
        — looks up the item, copies its body+citation+tablet_id
        into a new `ceremony_priorities` row with
        `confirmed_at = now()`. Returns the new priority row.
      - `reject_priority(item_id: &str) -> Result<(), CeremonyError>`
        — deletes the item from `ceremony_items`. If a
        `ceremony_priorities` row already references it, also
        deletes that.
      - `add_priority(req: AddPriorityRequest) -> Result<PriorityDto, CeremonyError>`
        — user-write path. Inserts directly into
        `ceremony_priorities` with `citation_id = None`. Body +
        rationale supplied by the user.
      - `list_priorities(tablet_id: &str) -> Result<Vec<PriorityDto>, CeremonyError>`
        — returns all priorities for a tablet, including
        unconfirmed candidates fetched indirectly via the items
        in `priorities` section. Output is the union of
        (a) ceremony_priorities rows tied to the tablet and
        (b) ceremony_items where section_key='priorities' AND
        kind='priority' AND no corresponding priority row yet
        (candidates awaiting confirmation).
- [ ] `PriorityDto` struct in the service module: `id, tablet_id,
      body, rationale, citation_id, confirmed_at, done_at,
      ordinal, source: "confirmed" | "candidate"`. The `source`
      field tells callers whether the row came from
      ceremony_priorities or is a yet-unconfirmed item candidate.
- [ ] New `CeremonyEvent::PriorityConfirmed { priority_id, tablet_id }`
      variant emitted by `confirm_priority`.
- [ ] Eight new WS-RPC methods in `crates/arawn/src/ws_server.rs`:
      `ceremonies.confirm_priority`, `ceremonies.reject_priority`,
      `ceremonies.add_priority`, `ceremonies.list_priorities`.
      Mounted via the existing `ceremonies.*` prefix dispatch.
- [ ] Service-level unit tests covering each method's happy path,
      error cases, and the events-emission contract.

## Implementation Notes

### Technical Approach

1. `confirm_priority` SQL:
   ```sql
   INSERT INTO ceremony_priorities (id, tablet_id, body, rationale, citation_id, confirmed_at, ordinal)
     SELECT ?1, tablet_id, body, '' as rationale, citation_id, ?2, ordinal
     FROM ceremony_items WHERE id = ?3
   ```
   `id` is a fresh UUID, `confirmed_at` is now(). Rationale is
   empty for v1 — the candidate item's body carries it inline.
   Future extension: split body into `{ text, rationale }` JSON
   so confirm can preserve both fields properly.
2. `list_priorities` is two SELECTs UNIONed in Rust (simpler than
   complex SQL): rows from `ceremony_priorities` (tagged
   `source: "confirmed"`), plus rows from `ceremony_items` in
   the `priorities` section whose item_id doesn't already appear
   as a citation_id in ceremony_priorities (tagged
   `source: "candidate"`).
3. The two-write-path contract from T-0282 still applies — but
   here we're writing into a *different* table, not via the
   compose pipeline. The engine doesn't see these writes; they're
   service-level mutations bracketing the engine's transaction.

### Dependencies

- Blocked by [[ARAWN-T-0301]] — needs the weekly plugin to be
  emitting priority candidates as ceremony_items.
- Unblocks [[ARAWN-T-0303]] (agent tools wrap these service
  methods).

### Risk Considerations

- **Race between confirm + reject**: two concurrent agent calls
  could fight. Service uses the existing `Arc<Mutex<Connection>>`,
  so SQL is serialised. The semantics — confirm-then-reject
  deletes both the item AND the just-created priority row — are
  documented; the agent shouldn't issue both for the same item.
- **Body/rationale shape**: v1 carries them mashed into the item
  body. Splitting into structured JSON is a follow-up if the LLM
  judge starts demanding cleaner separation.

## Status Updates

### 2026-05-16 — priorities service + RPC shipped

- Four new methods on `CeremonyService`: `confirm_priority`,
  `reject_priority`, `add_priority`, `list_priorities`. Plus new
  `PriorityDto` (with `source: "confirmed"|"candidate"`
  discriminator) and `AddPriorityRequest` types.
- `confirm_priority` is idempotent — second call on the same
  item returns the existing priority row. Rejects items that
  aren't in the `priorities` section.
- `reject_priority` deletes both the item and any priority row
  that cited it.
- `list_priorities` UNIONs ceremony_priorities (tagged
  confirmed) with priority-section items not yet referenced by
  any priority row (tagged candidate), sorted by ordinal.
- New `CeremonyEvent::PriorityConfirmed { priority_id,
  tablet_id }` variant emitted by `confirm_priority` only.
- Four new RPC arms in `ws_server.rs`:
  `ceremonies.confirm_priority/reject_priority/add_priority/list_priorities`.
  Registered in `RPC_METHODS`.
- 7 new service-level tests pass; existing 87 lib tests still
  pass (94 total now). arawn lib tests unchanged (57).

### Divergence noted

`confirm_priority` sets the priority row's `citation_id` to the
**item id** rather than the item's upstream `citation_id`. This
is the v1 idempotency handle (`citation_id = item_id` is the
dedupe key). The item's original source citation is reachable via
a join if downstream consumers need it.

Completed 2026-05-16.
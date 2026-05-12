---
id: wire-graphqlite-into-the-workspace
level: task
title: "Wire graphqlite into the workspace + smoke test"
short_code: "ARAWN-T-0238"
created_at: 2026-05-12T01:33:01.093535+00:00
updated_at: 2026-05-12T01:33:01.093535+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Wire graphqlite into the workspace + smoke test

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Get graphqlite available as a workspace dependency and prove it works inside the arawn process. Lays the foundation for T-0239 (MemoryStore rewrite) without touching memory yet.

graphqlite v0.4.4 lives at `/Users/dstorey/Desktop/graphqlite/bindings/rust`. It's the Rust binding for a SQLite extension that adds Cypher query support. Per the Phase 0 spike (see I-0040), the `bundled-extension` feature embeds the compiled extension via `include_bytes!()`, so no external install step is needed.

## Scope

- Add `graphqlite` to `[workspace.dependencies]` in the root `Cargo.toml`. Path dependency to `/Users/dstorey/Desktop/graphqlite/bindings/rust` for now (we'll switch to a published version when one lands or when graphqlite ships its own workspace publication).
- Create a small scratch test crate or inline test in `arawn-memory` that:
  - Opens a graphqlite DB at a temp path.
  - Creates a node + an edge via Cypher.
  - Reads them back.
  - Verifies the node and edge round-trip.
- Confirm the bundled extension loads cleanly inside the arawn workspace's sqlite version (graphqlite uses `rusqlite` with `bundled` + `load_extension` features). If our existing crates use a conflicting `rusqlite` major version, surface the conflict here so T-0239 doesn't trip over it.
- Document the result in this task's status updates: which crate(s) carry the dep, any rusqlite version coordination needed, and the exact path/version pinned.

## Acceptance Criteria

- [ ] `graphqlite` listed in `[workspace.dependencies]` and consumed from at least one crate (arawn-memory).
- [ ] Smoke test runs `cargo test` clean — opens DB, writes `(:Test {id: 1})` + `(:Test {id: 2})-[:R]->(:Test {id: 1})`, reads them back via Cypher.
- [ ] `angreal check workspace` clean across the whole workspace; no rusqlite version conflicts.
- [ ] Status update on this task documents the rusqlite version graphqlite pulls in, and whether our other sqlite-using crates (arawn-feeds, arawn-memory, arawn-storage) end up on the same version or need pinning.

## Implementation Notes

### Technical approach

1. Edit `Cargo.toml` workspace.dependencies:
   ```toml
   graphqlite = { path = "../../graphqlite/bindings/rust" }
   ```
   (Or absolute path; the workspace root is `/Users/dstorey/Desktop/arawn` so `../graphqlite/bindings/rust` from the workspace root.)
2. Edit `crates/arawn-memory/Cargo.toml` to depend on `graphqlite = { workspace = true }`.
3. Inline test in `crates/arawn-memory/src/lib.rs` (or a new module) gated `#[cfg(test)]`:
   ```rust
   use graphqlite::Graph;
   let dir = tempfile::tempdir().unwrap();
   let g = Graph::open(dir.path().join("smoke.db")).unwrap();
   g.upsert_node("n1", [("name", "alice")], "Test").unwrap();
   g.upsert_node("n2", [("name", "bob")], "Test").unwrap();
   g.upsert_edge("n1", "n2", std::iter::empty::<(&str, &str)>(), "R").unwrap();
   // assert counts via stats() or via a Cypher MATCH
   ```
4. Run `angreal test unit` to confirm.

### Dependencies

None upstream within arawn. Downstream: T-0239 depends on this.

### Risk considerations

- **rusqlite version coordination.** graphqlite uses `rusqlite = ">=0.31"`. Our other sqlite consumers (arawn-feeds, arawn-memory, arawn-storage, arawn-auth's token store, etc.) may be on a different version. A workspace-level mismatch will surface as a `cargo` build error. Mitigation: pin to a common version in `[workspace.dependencies]` and let cargo resolve.
- **Bundled extension size.** `bundled-extension` embeds the compiled `.dylib`/`.so` via `include_bytes!()`. This grows the binary. Acceptable cost; document the delta in the status update if it's surprising.

## Status Updates

*To be added during implementation*

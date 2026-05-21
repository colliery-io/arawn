---
id: t-k-delete-attentionsource-since
level: task
title: "T-K: Delete `AttentionSource::since()` legacy cursor method"
short_code: "ARAWN-T-0388"
created_at: 2026-05-21T14:53:30.914168+00:00
updated_at: 2026-05-21T14:53:30.914168+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-K: Delete `AttentionSource::since()` legacy cursor method

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Remove the legacy `since=` open-ended cursor method from the `AttentionSource`
trait, used by ceremony gather paths. Per operator decision (Tier 3 candidate
3.6): kill it. The `between(start, end)` method is the modern surface.

## Acceptance Criteria

### Audit

- [ ] Grep `crates/arawn-ceremonies/src/plugins/` (and any other plugin home) for production callers of `AttentionSource::since()`.
- [ ] If callers exist, migrate them to `between(start, end)` first, picking sensible end-of-window times for the previously-open queries.

### Removal

- [ ] Delete the `since` trait method from the `AttentionSource` trait at `crates/arawn-ceremonies/src/plugins/gather_sources.rs:51-54`.
- [ ] Delete every implementation of `since` across all `AttentionSource` impls in `crates/`.
- [ ] Update any tests that exercised the `since` path.

### Validation

- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean.
- [ ] `angreal test unit` passes.
- [ ] `angreal test integration` passes.
- [ ] Verify that ceremony gather behavior (daily/weekly/retro) is unchanged in a smoke test — the migrated `between` callers should produce the same result set as the old `since` callers.

## Implementation Notes

### Technical Approach

1. Audit current usage first. Production code may or may not still use it.
2. Migrate callers if any exist.
3. Delete the trait method and impls.
4. Validate.

### Dependencies

None internal to the initiative.

### Risk Considerations

- If any plugin genuinely relies on the open-ended semantics of `since`, migration to `between` requires picking an upper bound. Use a sensible default like "now" (or "end of current period_window").
- If a plugin is checked in but inactive, it may have stale `since` callers that don't fire today but would break in the future. Audit thoroughly.

## Verification

Per the initiative's dead-code methodology:
- `angreal check workspace`
- `cargo build --workspace --release`
- `cargo test --workspace --no-run`
- `angreal test unit`
- `angreal test integration`

Tier-3 boundary UAT (`angreal test uat` + `angreal test uat-judge`) should
run after T-I/T-J/T-K all land. Pay particular attention to ceremony brief
generation — if `since` was alive somewhere, that's where it manifested.

## Status Updates

*To be added during implementation*

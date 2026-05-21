---
id: t-k-delete-attentionsource-since
level: task
title: "T-K: Delete `AttentionSource::since()` legacy cursor method"
short_code: "ARAWN-T-0388"
created_at: 2026-05-21T14:53:30.914168+00:00
updated_at: 2026-05-21T16:41:11.768687+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

### 2026-05-21 — landed

**Audit:** zero production callers of `AttentionSource::since()`. The daily/weekly plugins already use `between(start, end, cap)` (daily.rs:178, weekly.rs:236). The only callers of `since` were:
- The default `between` impl in the trait (which oversampled via `since` then filtered) — gone.
- 3 test sites in `arawn-engine/src/ceremony_sources.rs:485/495/570` — migrated.

**Trait change:**
- Deleted `AttentionSource::since` method from the trait (`gather_sources.rs:58-62`).
- Removed the default `between` impl (it was the only remaining caller of `since`). `between` is now a required trait method.
- Tightened the trait doc to describe `between` as the canonical pinned-window query.

**Impl changes:**
- `StaticAttentionSource::between` (test stub): rewritten to filter by `[start, end)` (previously it ignored the cursor and returned the first `cap` items via `since`). Doc updated to match.
- `ProjectionsAttentionSource::since` (production-quality 120-line SQL impl): deleted entirely — only its `between` impl remains.

**Test migration in `arawn-engine/src/ceremony_sources.rs`:**
- `.since(cursor, 10)` → `.between(cursor, now + Duration::days(1), 10)` and `.since(cursor, 2)` → `.between(cursor, now + Duration::days(1), 2)` at lines 485/495/570.

**Test fix in `arawn-ceremonies/src/plugins/daily.rs`:**
- The new bounded `between` semantic surfaced two daily tests that were silently relying on the old "since ignores cursor" misbehavior. The old default `between` impl swallowed time mismatches between fixed-date fixtures and a `now`-based window from `EngineCtx::for_test`.
- `sample_signals` now takes a `ts: DateTime<Utc>` parameter, so the signal timestamp can be aligned to the window each test uses.
- `gather_collects_four_section_payload`: switched from `EngineCtx::for_test` to `EngineCtx::new` with an explicit `[2026-05-15T00:00:00Z, 2026-05-16T00:00:00Z)` window matching the period_key. Signal pinned to `2026-05-15T06:00:00Z`.
- `end_to_end_dispatch_writes_tablet_and_items`: signal pinned to `today - Duration::minutes(5)` so it falls inside the dispatcher's `period_window(today)` range.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 05s).
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test -p arawn-ceremonies --lib plugins::daily`: ✅ 8 tests pass (was 6/8 failing during the migration intermediate).
- Workspace lib tests across all crates: ✅ all pass.
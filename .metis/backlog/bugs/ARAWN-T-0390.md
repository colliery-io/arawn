---
id: fix-broken-brief-pipeline-rs
level: task
title: "Fix broken `brief_pipeline.rs` ScriptedPlugin — missing `period_window` trait method"
short_code: "ARAWN-T-0390"
created_at: 2026-05-21T14:53:33.521653+00:00
updated_at: 2026-05-21T15:11:41.613038+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Fix broken `brief_pipeline.rs` ScriptedPlugin — missing `period_window` trait method

## Backlog Item Details

### Type
- [x] Bug — Production issue that needs fixing
- [ ] Feature
- [ ] Tech Debt
- [ ] Chore

### Priority
- [ ] P0 — Critical
- [x] P1 — High (blocks ARAWN-I-0053 T-D for clean lint signal; silent test-coverage hole)
- [ ] P2 — Medium
- [ ] P3 — Low

### Impact Assessment

- **Affected Users**: developers running `cargo test --workspace --no-run`; possibly CI (depending on how exit codes are handled).
- **Reproduction Steps**:
  1. From repo root, run `cargo test -p arawn-ceremonies --test brief_pipeline --no-run`.
  2. Observe `error[E0046]: not all trait items implemented, missing: period_window` at `crates/arawn-ceremonies/tests/brief_pipeline.rs:59`.
  3. Note that `cargo test --workspace --no-run` (when piped) silently reports exit code 0 despite this — bash's `$?` after a pipe captures the LAST command's exit code, not cargo's. Use `${PIPESTATUS[0]}` to see the truth.
- **Expected vs Actual**: Expected: the test target compiles. Actual: missing trait-method impl on `ScriptedPlugin` prevents compilation, so the tests in `brief_pipeline.rs` are NOT being run.

## Objective

Restore compilation of the `brief_pipeline.rs` integration test by implementing
the missing `period_window` trait method on `ScriptedPlugin`. Then audit CI's
cargo exit-code handling to ensure pipeline failures of this kind are caught.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Add the missing `period_window(&self, period_key: &str) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError>` method to `impl Ceremony for ScriptedPlugin` at `crates/arawn-ceremonies/tests/brief_pipeline.rs:59-82`. Reasonable behavior: return a synthetic 1-day window derived from `self.period` (parse it as ISO date or week), or return a fixed window if the test only needs a stable value.
- [ ] `cargo test -p arawn-ceremonies --test brief_pipeline --no-run` returns clean (no E0046 error, exit code 0).
- [ ] `cargo test -p arawn-ceremonies --test brief_pipeline` runs all tests in the file; all pass.
- [ ] `cargo test --workspace --no-run` (the full suite compile) — verify clean exit by checking `${PIPESTATUS[0]}`, not bare `$?` through a pipe.
- [ ] **Bonus / follow-up:** investigate `.angreal/test.py` or whatever wraps `cargo test`. If it uses `cargo test 2>&1 | tee ...` style, ensure it uses `set -o pipefail` or checks `PIPESTATUS[0]` so a per-target compile failure aborts the run. Document the finding either inline as a comment or in the docs/contributing dir.

## Implementation Notes

### Technical Approach

The `Ceremony` trait requires `period_window`. Look at how the production
plugins (daily/weekly/retro) implement it — they typically return
`local_window::day_window_utc(...)` or `local_window::iso_week_window_utc(...)`.
For the test, use `local_window::day_window_utc(NaiveDate::from_str("2026-05-19").unwrap(), Tz::UTC)`
or similar, parameterized by the period string the test fixture is given.

### Dependencies

None. Pure test-only fix.

### Risk Considerations

Negligible. Bringing the test target back to life can ONLY reveal new
information. If the tests inside `brief_pipeline.rs` were passing before
the trait broke them (which seems likely — the trait change is recent),
they should pass again. If they don't, that's a separate finding worth
investigating.

## Status Updates

### 2026-05-21 — landed

- Added `period_window` method to `impl Ceremony for ScriptedPlugin` at `crates/arawn-ceremonies/tests/brief_pipeline.rs:66-72`. Used the same synthetic 1-day window pattern as `PluginStub` in `backfill.rs` — `Utc::now()` and `Utc::now() + Duration::days(1)`. Added `Duration` to the chrono import on line 24.
- Verification:
  - `cargo test -p arawn-ceremonies --test brief_pipeline --no-run`: ✅ clean (was E0046).
  - `cargo test -p arawn-ceremonies --test brief_pipeline`: ✅ **2 tests pass** (`brief_pipeline_missing_weekly_renders_placeholder`, `brief_pipeline_renders_daily_and_weekly_content`). Both were silently dead before this fix.
  - `cargo test --workspace --no-run`: ✅ clean (35.74s, all test targets compile).
- **Bonus / CI audit:** the angreal test wrapper (`.angreal/task_test.py`) uses `subprocess.run(..., check=True)` without piping through a shell. Python's subprocess captures cargo's real exit code, so `check=True` raises on non-zero. **CI was correctly catching the failure** — the bash-pipe quirk (`$?` returning the last pipeline command's exit code, not cargo's) is only a hazard when developers run `cargo test 2>&1 | tee ...` interactively. No CI fix needed. Documented inline here for posterity.
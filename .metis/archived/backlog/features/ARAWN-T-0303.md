---
id: e2e-tests-for-routes-logs-rs-0
level: task
title: "E2E tests for routes/logs.rs (0% coverage)"
short_code: "ARAWN-T-0303"
created_at: 2026-03-09T15:43:26.251243+00:00
updated_at: 2026-03-10T00:55:52.668127+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# E2E tests for routes/logs.rs (0% coverage)

## Objective

Add E2E tests for `crates/arawn-server/src/routes/logs.rs` which currently has 0% coverage from both unit and E2E tests. This is a brand new module with no test coverage at all.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (completely untested new code)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] E2E tests cover all log endpoints (list, get, stream)
- [ ] Error cases covered (not found, invalid params)
- [ ] Coverage for `routes/logs.rs` reaches at least 70%

## Implementation Notes

### Key Files
- `crates/arawn-server/src/routes/logs.rs` (0/118 lines covered)
- Add tests to `crates/arawn-server/tests/e2e_scenarios.rs` or new `e2e_logs.rs`

## Status Updates

### Complete
- Created `crates/arawn-server/tests/e2e_logs.rs` with 12 E2E tests:
  - `scenario_list_log_files` — lists .log files sorted by name descending
  - `scenario_list_log_files_empty` — empty log dir returns empty array
  - `scenario_list_log_files_filters_non_log` — only .log files returned
  - `scenario_list_log_files_no_directory_returns_empty` — no logs dir returns empty
  - `scenario_get_logs_latest` — fetches most recent log file by default
  - `scenario_get_logs_specific_file` — file param selects specific log
  - `scenario_get_logs_with_lines_limit` — lines param limits tail output
  - `scenario_get_logs_lines_capped_at_1000` — max 1000 lines enforced
  - `scenario_get_logs_full_filename` — full filename with .log extension works
  - `scenario_get_logs_file_not_found` — 404 for nonexistent file
  - `scenario_get_logs_no_files_returns_404` — 404 when no log files exist
  - `scenario_get_logs_no_directory_returns_404` — 404 when logs dir missing
- **Coverage**: `routes/logs.rs` went from 0% to **93.2%** (110/118 lines)
- **All 82 E2E tests pass**, clippy clean
- Tests use `ARAWN_CONFIG_DIR` env var + tempfile for isolation
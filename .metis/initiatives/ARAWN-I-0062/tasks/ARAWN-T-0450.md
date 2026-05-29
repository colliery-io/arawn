---
id: t-f-uatdriveclient-and-remaining
level: task
title: "T-F: UatDriveClient and remaining service stubs"
short_code: "ARAWN-T-0450"
created_at: 2026-05-28T22:01:44.832147+00:00
updated_at: 2026-05-29T20:15:07.157909+00:00
parent: ARAWN-I-0062
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0062
---

# T-F: UatDriveClient and remaining service stubs

## Parent Initiative
[[ARAWN-I-0062]]

## Objective
Round out the integration mock surface by adding Drive (and only then
Atlassian / GitHub if a scenario actually exercises them). No new scenarios
required — spike each stub against a smoke test that calls the read-side tool
and asserts a deterministic response.

## Technical Approach
- `UatDriveClient` — list/search files, get file metadata. Read-side rows
  come from the Drive projection.
- For Atlassian + GitHub: only build the stub if [[ARAWN-T-0451]] needs them.
  Otherwise leave it as a follow-up so scope stays bounded.
- Add a generic smoke test under `crates/arawn-tests/tests/uat.rs` that
  registers each mocked service and asserts at least one read tool succeeds.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `UatDriveClient` deterministic read-side coverage; smoke test green.
- [ ] If Atlassian/GitHub stubs are needed by T-G, they ship here with the
      same shape.
- [ ] `angreal check workspace` + `arawn-tests` green.

## Dependencies
Depends on [[ARAWN-T-0445]]. Independent of B/C/D ordering.

## Status Updates

**2026-05-29 — Done (Drive only; Atlassian/GitHub deferred).** No current UAT
scenario reaches for Drive, so this is a spike that lays the pattern down for
whenever one does.

- `arawn-integrations::drive::uat_tools::UatDriveSearchTool` reads
  `drive_files` via `ProjectionStore::fts_search` and emits the production
  `FileSummary` wire shape. Production has a larger surface (`drive_list`,
  `drive_get_metadata`, `drive_read`, plus writes); UAT ships
  `drive_search` only — the minimum needed for read-side scenarios.
- `strip_drive_query` strips Drive-syntax operators (`name contains 'x'`,
  `mimeType = 'pdf'`, …) down to an FTS-friendly token bag.
- `drive/mod.rs` re-exports; `wire_uat_mock_integrations` gains a
  `"google_drive"` arm.
- 3 unit tests on `uat_tools` (by-filename, by-body-text, operator strip).

**Verification:** unit tests green; `angreal check workspace` exit 0. Not
exercised end-to-end yet — no scenario uses Drive. When one is added in T-G
or later, the wiring is in place.

**Atlassian + GitHub** deliberately stayed out per the original task plan
("only build the stub if T-G needs them"). T-G's required_evidence work
doesn't add scenarios that need them, so this stays a follow-up. The
pattern is now obvious (each service is ~150 lines of `uat_tools` + a
4-line registration arm), so adding them when the first scenario lands is
cheap.
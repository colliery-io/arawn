---
id: t-f-uatdriveclient-and-remaining
level: task
title: "T-F: UatDriveClient and remaining service stubs"
short_code: "ARAWN-T-0450"
created_at: 2026-05-28T22:01:44.832147+00:00
updated_at: 2026-05-28T22:01:44.832147+00:00
parent: ARAWN-I-0062
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
- [ ] `UatDriveClient` deterministic read-side coverage; smoke test green.
- [ ] If Atlassian/GitHub stubs are needed by T-G, they ship here with the
      same shape.
- [ ] `angreal check workspace` + `arawn-tests` green.

## Dependencies
Depends on [[ARAWN-T-0445]]. Independent of B/C/D ordering.

## Status Updates
*To be added during implementation*

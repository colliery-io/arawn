---
id: uat-secret-management-sops-age-for
level: task
title: "UAT secret management — sops + AGE for test API keys"
short_code: "ARAWN-T-0262"
created_at: 2026-05-13T13:35:35.098186+00:00
updated_at: 2026-05-13T13:35:44.433678+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# UAT secret management — sops + AGE for test API keys

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0040]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates

### 2026-05-13 — Scaffolding shipped

**Approach:** sops + AGE per-developer keypair. Asymmetric model — each dev holds their own AGE private key; their pubkey lives in `.sops.yaml`'s recipient list; ciphertext in `tests/secrets/*.enc.yaml` is encrypted to all recipients. Rotation = any recipient `sops edit`s and pushes; everyone pulls. No symmetric "team key" floating between humans.

**Files committed:**

- `.sops.yaml` — recipient list (placeholder pubkey to be replaced before first use) + `creation_rules` that match anything under `tests/secrets/*.enc.{yaml,json,toml}`. `encrypted_regex` limits encryption to `*_KEY` / `*_TOKEN` / `*_SECRET` / `*_PASSWORD` values so non-secret metadata stays plaintext for readable diffs.
- `tests/secrets/README.md` — onboarding (per-dev), rotation, add/remove recipient, bootstrap-first-file, and "how the test harness uses it" runbook. Treat it as the SOP doc.
- `.gitignore` — explicit allowlist: only `*.enc.*` survives under `tests/secrets/`; bare `.yaml/.json/.toml` and any `*.age` / `keys.txt` get blocked so an accidental plaintext drop can't land.

**Angreal tasks (in `.angreal/task_test.py`):**

- `angreal test uat` — now auto-detects `tests/secrets/uat.enc.yaml` and wraps the cargo invocation in `sops exec-env <file> '<cargo cmd>'`. Falls back to bare-env behavior when the file is absent, so devs who haven't onboarded to sops yet (or scenarios that don't need secrets) keep working unchanged. Logs which path it took.
- `angreal test secrets-edit [--file uat.enc.yaml]` — `sops edit` wrapper that finds the file relative to repo root regardless of CWD.
- `angreal test secrets-updatekeys` — re-encrypts every `*.enc.*` under `tests/secrets/` to the current recipient list. Run after PR-merging a new recipient.

**Verified:** `angreal tree` lists the new tasks; workspace tests + clippy still exit 0.

**What's NOT done yet (intentional — needs human input):**

1. **First recipient pubkey.** `.sops.yaml` currently has a `REPLACE_ME` placeholder. The user runs `age-keygen -o ~/.config/sops/age/keys.txt`, pastes the resulting `age1...` pubkey into `.sops.yaml`, and commits.
2. **The first encrypted bundle.** `tests/secrets/uat.enc.yaml` doesn't exist in the repo. After step 1 the user follows "Bootstrapping the first file" in the README:
   ```sh
   echo 'OLLAMA_API_KEY: "..."' > /tmp/uat.yaml
   sops --encrypt /tmp/uat.yaml > tests/secrets/uat.enc.yaml
   shred -u /tmp/uat.yaml
   ```
3. **gitleaks pre-commit hook.** Mentioned as "belt-and-suspenders" earlier; deferred — not adding new pre-commit framework tonight. Reasonable follow-up task when we want stricter guardrails.

**Verification path** (post-bootstrap):
- `angreal test secrets-edit` opens the file in `$EDITOR`.
- `angreal test uat --scenario signal-extraction-e2e` runs through `sops exec-env`, secrets reach the cargo subprocess as env vars.
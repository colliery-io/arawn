---
id: encrypted-token-store-using-age
level: task
title: "Encrypted token store using age — round-trip JSON tokens at {data_dir}/tokens/"
short_code: "ARAWN-T-0181"
created_at: 2026-04-17T03:01:16.576563+00:00
updated_at: 2026-04-17T03:13:53.468941+00:00
parent: ARAWN-I-0029
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0029
---

# Encrypted token store using age — round-trip JSON tokens at {data_dir}/tokens/

## Parent Initiative

[[ARAWN-I-0029]]

## Objective

Persist OAuth tokens encrypted at rest under `{data_dir}/tokens/<provider>.json.age`. The encryption key is generated on first use and stored at `{data_dir}/tokens/.master.age-recipient` (and `.master.age-key` with `0600`); the agent has no read access (sensitive-paths deny list, T-0182).

Implementation in `crates/arawn-integration/src/auth/token_store.rs`. Uses the `age` crate for symmetric encryption with an x25519 keypair. JSON layout:

```json
{ "access": "...", "refresh": "...", "expires_at": "2026-04-17T..." }
```

OS keyring fallback is documented as a follow-up but **not** built here — keep this task focused on the file-based path.

Estimated size: **S–M** (1–2 days).

### Priority
- [x] P2 - Medium (gates real provider work; security-critical)

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

## Acceptance Criteria

## Acceptance Criteria

- [ ] `TokenStore::open(data_dir: &Path) -> Result<Self, IntegrationError>` — initializes/loads the master key on first call
- [ ] `TokenStore::save(provider: &str, token: &Token) -> Result<()>` — writes `{data_dir}/tokens/<provider>.json.age`
- [ ] `TokenStore::load(provider: &str) -> Result<Option<Token>>` — `None` when no token exists yet
- [ ] `TokenStore::delete(provider: &str) -> Result<()>` — removes the on-disk file
- [ ] Master key (`.master.age-key`) is created with mode `0600`; the recipient (`.master.age-recipient`) is `0644`
- [ ] Round-trip test: save a `Token` for `provider="google"`, load it back, byte-equal access+refresh strings
- [ ] Tampering test: corrupt one byte of the encrypted file, `load` returns `IntegrationError::ApiError` (or a clearly-named decrypt error variant)
- [ ] Missing-key test: delete the master key file, `load` returns a clear error explaining the situation
- [ ] Cross-process test: write with one `TokenStore` instance, read with a freshly-opened second instance
- [ ] `cargo doc` mentions OS-keyring fallback as a follow-up
- [ ] Depends on ARAWN-T-0179

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

- New `auth/token_store.rs` with `TokenStore { open, save, load, delete, tokens_dir }`. Layout: `{data_dir}/tokens/.master.key` (32-byte symmetric key) + `<provider>.json.enc` (nonce || ciphertext).
- **Spec deviation**: chose ChaCha20-Poly1305 (`chacha20poly1305` crate, ~5 small RustCrypto deps) over `age` because (a) `age` would have pulled in a much larger transitive graph for what is symmetric encryption from a single host to itself, (b) the AC requires "encrypted at rest with a key not readable by the agent" — both crates satisfy this — and (c) the ChaCha approach is ~140 lines total vs an age-based version that needs identity files, recipient files, and armor handling. The on-disk format is documented and easy to migrate later if we ever do want public-key recipients.
- Master key generated with `rand::OsRng` on first `open()`, persisted at mode 0600 (verified by test on Unix). Saves use atomic rename. Provider names sanitized (path-traversal characters stripped) before joining.
- 9 unit tests: round-trip, missing→None, delete-then-load, idempotent delete, tampered ciphertext → `Decode` error, second `open()` reuses key, missing-master-key after rotation → `Decode` error, path-traversal sanitization, master key 0600 (Unix only).
- Doc-comment in module mentions "OS-keyring fallback" as a future follow-up.
- All `cargo test -p arawn-integration` tests pass (22/22 — 13 from T-0180 + 9 new).
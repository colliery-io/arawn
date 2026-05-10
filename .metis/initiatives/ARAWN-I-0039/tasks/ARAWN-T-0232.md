---
id: drive-folder-sync-bare-folder-name
level: task
title: "drive folder-sync: bare folder name should fall back to path lookup on 404"
short_code: "ARAWN-T-0232"
created_at: 2026-05-10T00:00:00+00:00
updated_at: 2026-05-10T00:00:00+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
---

# drive folder-sync: bare folder name should fall back to path lookup on 404

## Parent Initiative

[[ARAWN-I-0039]]

## Severity

P3 — UX rough edge surfaced during T-0218 UAT. Doesn't block; just confusing.

## What happens today

`/watch drive/folder-sync mine folder=Letters` → 404 from Drive ("File not found: Letters") because `RealDriveClient::resolve_folder` interprets bare strings (no `/`) as Drive file IDs:

```rust
if path_or_id == "root" { return Ok("root".into()); }
if !path_or_id.contains('/') {
    // Could be a literal id; verify by fetching metadata.
    let (_, file) = hub.files().get(path_or_id)...doit().await?;
    ...
}
```

A user typing the human-readable folder name (the most natural thing) hits a confusing 404 with no hint to use `/Letters` or the actual ID.

Workaround: `folder=/Letters` (leading slash → path-walk codepath) or paste the full ID from a Drive URL. Both work; neither is discoverable.

## Action

Smarter `resolve_folder`:

1. If `path_or_id == "root"` → "root" (existing behavior).
2. If `path_or_id` contains `/` → walk path under root (existing).
3. Otherwise: try ID lookup first. **On 404, fall back to a path-walk treating the input as a single segment under root** (i.e. as if the user wrote `/Letters`).
4. If both fail, raise a clear error message: `"no folder named or with id 'Letters' under My Drive"`.

The fallback should only fire on 404 specifically; any other error (auth, rate-limit, server) propagates.

## Acceptance Criteria

- [ ] `resolve_folder` falls back to single-segment path lookup on 404 from the ID-lookup path.
- [ ] Error message when both lookups fail mentions both forms tried.
- [ ] Add a unit test: mock client returns 404 for the ID lookup + a matching folder for the path lookup; assert resolve returns the folder's id.
- [ ] Add a unit test: both lookups fail; assert the error message names both forms.
- [ ] Existing behavior preserved for `root`, raw IDs, and slash-delimited paths.

## Status Updates

*To be added during implementation*

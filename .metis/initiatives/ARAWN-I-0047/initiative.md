---
id: google-docs-comment-mention
level: initiative
title: "Google Docs comment + mention monitoring — outstanding comments as a morning-brief signal"
short_code: "ARAWN-I-0047"
created_at: 2026-05-15T15:15:34.061418+00:00
updated_at: 2026-05-15T15:15:34.061418+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: S
initiative_id: google-docs-comment-mention
---


## Context **[REQUIRED]**

Arawn already reads Google Doc *content* via the Drive integration (`drive.files.export` returns text or markdown for native docs). What Drive does *not* surface is the personal-life-org signal that actually matters: **outstanding comments and @-mentions on docs the user co-owns or has been added to.**

The personal-coordination story for shared docs almost always lives in the comments, not the body: "Alice left a comment on the planning doc yesterday — unresolved, addressed to you." Today arawn cannot see that. The morning brief (I-0041) would surface this the same way it surfaces GitHub review-requested PRs or Slack DMs.

This initiative is deliberately scoped narrow. The full Google Docs API can do structured edits, suggestion accept/reject, style application — all out of scope. The unique value over what Drive already gives us is the comment + mention signal.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- Pull outstanding (unresolved) comments on Google Docs the user has access to.
- Identify @-mentions of the user across those comments.
- Surface both as items the morning brief can render alongside GitHub / Slack / email signals.
- Agent tool: `gdocs.reply_to_comment` (the one write operation worth having — replying to a comment closes the personal-coordination loop without arawn needing to be a doc editor).
- Reuse the existing Google OAuth scope already in use for Drive/Gmail/Calendar where possible. Add the `drive.readonly` + `documents.readonly` scopes incrementally if needed.

**Non-Goals:**
- Structured doc editing (insert/format/style). The Docs API supports it; we do not.
- Suggestion accept/reject. Doc-editing surface, not our problem.
- Comment *resolution* (closing a thread). Reply yes, resolve no — resolving is a workflow decision the human makes.
- Surfacing on docs the user is merely a viewer of and has not been mentioned in. Too much noise.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

### Functional sketch
- REQ-001: Periodic poll of `drive.comments.list` across recently-active docs the user has access to. (Comments API is on Drive, not Docs — discovery point.)
- REQ-002: Filter to unresolved comments addressed to the user or @-mentioning the user.
- REQ-003: Maintain a since-cursor per doc so we do not re-fetch resolved history.
- REQ-004: Agent tool `gdocs.reply_to_comment(doc_id, comment_id, body)`.
- REQ-005: Items expose enough context (doc title, comment author, snippet, timestamp, link) to render in the brief.

### Non-functional
- NFR-001: Existing Google OAuth token reused; no new credential surface to manage.
- NFR-002: Polling cadence configurable, default conservative (every 30 min); piggyback on the existing Drive/Gmail polling rhythm where reasonable.

## Detailed Design **[REQUIRED]**

### Pre-design open questions
- **Discovery scope.** We have to decide which docs to poll. Options: (a) all docs the user has touched in the last N days, (b) all docs in the user's "shared with me" view, (c) only docs explicitly registered in arawn. (a) is the most useful, (b) is broadest and noisiest, (c) is most controlled. Likely (a) with a configurable lookback.
- **Comments API surface.** Comments + replies live on Drive's API (`drive.comments`, `drive.replies`) not on Docs. The Docs API is for body-content operations. This is a discovery-phase factual check before we wire anything.
- **De-dup against email.** Google emails the user when they're @-mentioned in a Doc comment. Arawn will also see that via Gmail. We need a dedupe story so the same comment does not appear twice in the brief.

### Existing pattern to copy
`crates/arawn-integrations/src/drive/` and `crates/arawn-integrations/src/gmail/` between them already establish the Google OAuth + paginated poll + typed model shape. This is incremental work against an existing integration surface, not a new one.

## Alternatives Considered **[REQUIRED]**

- **Rely on Gmail notifications for Doc mentions.** Rejected as the primary signal: Gmail mention emails are noisy, can be filtered or rate-limited by the user, and do not give us the resolved/unresolved state. Useful as a *dedupe input*, not as the source of truth.
- **Full Google Docs integration (structured edits, suggestions, etc.).** Rejected on focus grounds — arawn is not a doc editor. The narrow-scope framing is the whole point of filing this as its own initiative.

## Implementation Plan **[REQUIRED]**

Discovery-phase deliverables, not decomposed yet:

1. Verify the Comments API surface and pick the discovery-scope option ((a)/(b)/(c) above).
2. Decide the Gmail-mention dedupe shape.
3. Decompose into tasks (~3–4 expected): scope-driven doc discovery; comment poll + filter; reply-comment agent tool; morning-brief renderer hookup.

## Exit Criteria

- Unresolved @-mentions and direct-addressed comments on the user's Google Docs surface in the morning brief.
- Agent can reply to a target comment via tool call, end-to-end, against a real account.
- No duplicate items between Gmail mention emails and the new Docs signal.

---
id: integration-docs-phase-2-google
level: task
title: "Integration docs Phase 2 — Google how-to rewrite (Gmail/Calendar/Drive)"
short_code: "ARAWN-T-0373"
created_at: 2026-05-20T16:00:00+00:00
updated_at: 2026-05-20T16:00:00+00:00
parent: ARAWN-I-0038
blocked_by: ["ARAWN-T-0372"]
archived: false

tags:
  - "#task"
  - "#docs"
  - "#integrations"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0038
---

# Phase 2 — Google how-to rewrite

## Objective

Rewrite `docs/src/how-to/connect-google.md` as a single page
covering Gmail + Calendar + Drive. Matches the singular Google
Cloud Console entry point — users create one OAuth client and
enable per-service APIs as needed.

## Scope

Single file: REWRITE `docs/src/how-to/connect-google.md`.

Structure (Diátaxis how-to template):

1. **What you get** — tools that land for each service
   (Gmail: `gmail_inbox_read`, `gmail_search`, `gmail_send`,
   etc.; Calendar: `calendar_*`; Drive: `drive_*`). Pull
   names from the integration crates, do not paraphrase.
2. **Prerequisites** — Google Cloud project + billing-not-
   required note + which APIs to enable.
3. **Setup** — sequential steps:
   - Enable APIs (Gmail / Calendar / Drive) in Cloud Console.
   - Create OAuth client (Desktop type).
   - Configure consent screen (External + Testing).
   - Add scopes — include the scope picker filtering quirk
     and the manual scope-add textarea path.
   - Drive scope warning — full read+write is the default,
     `.readonly` is opt-in.
4. **Configure arawn** — paste TOML snippet for
   `[integrations.google]` (shared) AND `[integrations.gmail]`
   etc. (per-service). Reference Phase 1's
   `integrations-config.md` for the resolution rules.
5. **Connect** — `/connect gmail` / `/connect google_calendar`
   / `/connect google_drive`, browser flow, what success looks
   like (`/integrations` row).
6. **Verification** — minimal exercise prompt per service.
7. **Troubleshooting** — link to Phase 5's matrix; this page
   only lists Google-specific gotchas (unverified-app warning,
   100-test-user cap, scope-cache-vs-revoke issue).

## Correctness mandate

- Scope strings: grep `crates/arawn-integrations/src/gmail/`,
  `.../google_calendar/`, `.../drive/` for the exact OAuth
  scope URIs. Paste verbatim — Google's scope picker matches
  exact strings.
- Tool names: from the same crates' `Tool::name()` impls.
- Service names in `/connect`: verify the slash-command
  registry — `connect-google` may dispatch to multiple
  service ids.
- TOML keys (`[integrations.gmail]` etc.): verify against the
  config parser.
- Redirect URI: arawn-auth's callback server uses
  `http://127.0.0.1:<port>/callback` for Google (per
  `crates/arawn-auth/src/oauth.rs` — verify exact form before
  writing). The 100-test-user cap and unverified-app warning
  come from Google's documented limits, not arawn code.

Anywhere the Google Cloud Console UI navigation can't be
verified from code, mark `<!-- VERIFY: 2026-05-20 -->` so the
docs-UAT walkthrough catches it.

## Acceptance criteria

- [ ] `connect-google.md` rewritten as a single page covering
  Gmail + Calendar + Drive.
- [ ] Every scope string is grep-able in the integration
  crates.
- [ ] Every tool name listed is registered in code.
- [ ] Drive's full-scope default warning is called out
  explicitly.
- [ ] Scope picker filtering quirk + manual-textarea path are
  documented.
- [ ] Unverified-app warning + 100-test-user cap are noted.
- [ ] All provider-UI nav lines that can't be verified from
  code are `VERIFY`-tagged with today's date.
- [ ] `angreal docs build` clean.
- [ ] User docs-UAT: follow the rewritten page on a clean
  Google account to a working `/connect gmail` — flag every
  `VERIFY` mismatch.

Parent: [[ARAWN-I-0038]].

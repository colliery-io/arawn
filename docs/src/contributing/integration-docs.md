# Maintaining integration docs

*Contributor guide. How to keep `how-to/connect-*.md` + `how-to/debug-oauth-failures.md` accurate as the code + provider UIs change.*

This guide exists because the integration docs are uniquely fragile:

1. **Provider UIs shift unpredictably.** Google rebranded "OAuth consent screen" → "Google Auth Platform" with new sub-section names in late 2024; Slack relocates settings; Atlassian moves between developer-console versions. None of this shows up in any test we run.
2. **Scope lists drift between code and docs.** Adding a new tool means adding a scope to the integration crate — easy to miss the doc update. The Atlassian audit during initiative I-0038 found 6 scopes missing from the docs.
3. **The docs UAT (user follows them on real provider UIs) is the only way to catch some drift.** It's not part of CI; it depends on a human walking through the consent flow.

## Audit cadence

Hard rule: **after any change that touches `crates/arawn-integrations/src/<svc>/`** (scope constants, OAuth flow shape, tool registration), update the corresponding `docs/src/how-to/connect-<svc>.md` in the same PR.

Soft rule: **once per quarter**, walk every `connect-*.md` against the current provider UI and update any stale section path. Renew the `<!-- VERIFY: YYYY-MM-DD -->` markers on the same pass.

## What to grep

Before declaring a doc fresh, grep the following sources of truth:

### Scope strings

For each integration crate:

```sh
# Google: 3 scopes per integration crate, in *_OAUTH_SCOPE constants.
grep -E 'OAUTH_SCOPE|googleapis.com/auth/' crates/arawn-integrations/src/gmail/integration.rs
grep -E 'OAUTH_SCOPE|googleapis.com/auth/' crates/arawn-integrations/src/calendar/integration.rs
grep -E 'OAUTH_SCOPE|googleapis.com/auth/' crates/arawn-integrations/src/drive/integration.rs

# Slack: SLACK_OAUTH_SCOPES (bot) + SLACK_OAUTH_USER_SCOPES (user).
grep -E 'SLACK_OAUTH_(USER_)?SCOPES' crates/arawn-integrations/src/slack/integration.rs

# Atlassian: ATLASSIAN_OAUTH_SCOPES + the per-tool *_SCOPES constants.
grep -E 'ATLASSIAN_OAUTH_SCOPES|_SCOPES:|read:|write:' crates/arawn-integrations/src/atlassian/integration.rs
grep -E '_SCOPES:' crates/arawn-integrations/src/atlassian/jira.rs
grep -E '_SCOPES:' crates/arawn-integrations/src/atlassian/confluence.rs
```

Count the scopes in code, count the scopes in the doc, compare. If they don't match, the doc is stale.

### Service names (the `/connect <name>` argument)

```sh
grep -E 'pub const SERVICE_NAME' crates/arawn-integrations/src/*/integration.rs
```

These are the strings users type. Don't paraphrase.

### Tool names

```sh
grep -E '^    fn name.*->.*&str' crates/arawn-integrations/src/<svc>/tools.rs -A 1
```

If the doc lists tool counts ("6 Slack tools", "11 Atlassian tools"), verify them.

### Env-var names

```sh
grep -E 'ARAWN_(GMAIL|GCAL|GDRIVE|GOOGLE|SLACK|ATLASSIAN|GITHUB)_' crates/arawn/src/main.rs
```

These are documented in `docs/src/reference/env-vars.md` and `docs/src/reference/integrations-config.md`. Both must match.

### Error messages

The exhaustive matrix in `debug-oauth-failures.md` must cover every variant of `IntegrationError` (`crates/arawn-integrations/src/error.rs`) and `AuthError` (`crates/arawn-auth/src/error.rs`), plus every `tracing::warn!` / `tracing::error!` in the integration + auth crates:

```sh
grep -E 'pub enum (IntegrationError|AuthError)' crates/arawn-integrations/src/error.rs crates/arawn-auth/src/error.rs
grep -nE 'tracing::(warn|error)!|warn!\(|error!\(' crates/arawn-integrations/src crates/arawn-auth/src
```

If a new variant is added, the matrix needs a new row in the same PR.

## The `<!-- VERIFY: YYYY-MM-DD -->` convention

Provider-UI navigation (menu paths, section names, button labels) can't be verified from arawn's code — they live in someone else's product. Mark every such claim with an HTML comment:

```markdown
<!-- VERIFY: 2026-05-20 — Google Auth Platform → Branding section path. -->

Left nav → **Google Auth Platform → Branding**.
```

The date is when the claim was last verified-by-walking-through.

When you do a docs UAT and find the path still correct, **bump the date** (no other change needed). When you find it broken, fix the navigation prose AND bump the date.

This makes drift easy to spot: any VERIFY marker more than ~3 months old is suspect. Easy to grep:

```sh
grep -rn 'VERIFY:' docs/src/how-to/
```

## Docs UAT workflow

The full walk-through whenever a docs change is non-trivial:

1. Pick a provider; create a **fresh** OAuth app (don't reuse an existing one — the goal is to validate the doc's full setup path).
2. Open the corresponding `connect-<svc>.md` in the rendered docs site.
3. Follow it literally, one step at a time. Don't fix mistakes inline — note them and continue.
4. After the agent successfully exercises one tool from each registered integration, the doc passes the UAT.
5. File the notes back into the doc with a status update on the relevant Metis task.

Common UAT findings:

- "Scope X doesn't appear in the picker" → API for it isn't enabled (need a doc cross-reference).
- "Menu says Y but doc says X" → provider UI shifted; bump the VERIFY marker and update the prose.
- "Step N says click W but W isn't there" → provider deprecated the button; find the replacement and document it.

## What this guide doesn't cover

- **Screenshots.** A future initiative may add dated screenshots for the highest-traffic walkthroughs. Until then, captions only.
- **i18n.** English-only for now.
- **Auto-generation.** The reference tables in `docs/src/reference/integrations.md` could be auto-generated from the integration crates; the walkthroughs (prose + screenshots + troubleshooting recipes) can't and won't be.

## See also

- [Integrations overview](../explanation/integrations-overview.md) — the BYO model + decision tree.
- [Integrations config reference](../reference/integrations-config.md) — the precise resolution rules.
- [Debug OAuth failures](../how-to/debug-oauth-failures.md) — the symptom-keyed matrix.

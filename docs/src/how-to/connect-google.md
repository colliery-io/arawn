# Connect Google (Gmail, Calendar, Drive)

*How-to. ~10 minutes. One Google Cloud project covers all three services.*

By the end of this guide arawn will have OAuth tokens for Gmail, Calendar, and Drive, and the agent will be able to list emails, check your calendar, and read Drive files.

If you've never set up an OAuth integration, skim the [OAuth primer](../explanation/oauth-primer.md) first — it explains the four pieces (client_id, client_secret, scope, redirect URI) the steps below ask you to assemble.

## Prerequisites

- arawn server running ([first chat session](../tutorials/first-chat.md)).
- A Google account.
- About 10 minutes in front of <https://console.cloud.google.com/>.

## 1. Create or pick a Google Cloud project

Go to <https://console.cloud.google.com/>. Create a new project (any name) or pick an existing one. Note the **project number** on the dashboard — you'll use it in the URLs below.

## 2. Enable the APIs you want

You only need to enable the APIs for the services you'll actually use:

- Gmail: <https://console.cloud.google.com/apis/library/gmail.googleapis.com>
- Calendar: <https://console.cloud.google.com/apis/library/calendar-json.googleapis.com>
- Drive: <https://console.cloud.google.com/apis/library/drive.googleapis.com>

Click **Enable** on each. If the button says "Manage", it's already enabled.

> **Gotcha:** the OAuth scope picker in step 4 only shows scopes for *enabled* APIs. If a scope you expect doesn't appear, come back to this step and enable the API first.

## 3. Configure the OAuth consent screen

<!-- VERIFY: 2026-05-20 — "Google Auth Platform" menu was renamed from "OAuth consent screen" in late 2024; sub-section names (Branding / Audience / Data Access) may shift further. -->

Left nav → **Google Auth Platform → Branding**. Direct URL: `https://console.cloud.google.com/auth/branding?project=<PROJECT>`.

- **User type:** External (unless you're inside a Google Workspace org and only want it for that org's users).
- **App name:** anything (`arawn-personal` works).
- **User support email:** your email.
- **Developer contact:** your email.

Save.

> **Important — publishing status.** A new app has the publishing status **Testing**. In Testing, Google issues refresh tokens that **expire after 7 days** when the app asks for scopes other than name, email and profile. arawn asks for Gmail, Calendar and Drive scopes, so in Testing you must reconnect every week. ([Google: Using OAuth 2.0](https://developers.google.com/identity/protocols/oauth2#expiration): "…a publishing status of 'Testing' is issued a refresh token expiring in 7 days…")
>
> For personal use, set the status to **In production**: **Google Auth Platform → Audience → Publish app**. Do not submit the app for verification. Google allows an app that is used only by you, or by a few people you know, to stay unverified ([Google: unverified apps](https://support.google.com/cloud/answer/13464323)). The consent page then shows "Google hasn't verified this app" each time you connect. Click **Advanced**, then continue (see step 8). The limit for an unverified app is 100 users. That limit does not affect one person.
>
> Google's pages do not say directly that the Gmail and full-Drive scopes work for an unverified app In production. If Google blocks the consent, set the status back to Testing and add yourself as a test user (step 5). Then reconnect weekly.
>
> Verification is necessary only to give the app to people you do not know. See [the BYO reality](../explanation/integrations-overview.md#the-byo-reality) for the long-term plan.

<!-- VERIFY: 2026-10-04 — publishing-status guidance per ARAWN-T-0508 (sources: developers.google.com/identity/protocols/oauth2, support.google.com/cloud/answer/13464323, /15549945). -->

## 4. Add OAuth scopes

<!-- VERIFY: 2026-05-20 — Google Auth Platform → Data Access section path. -->

Left nav → **Google Auth Platform → Data Access**. Direct URL: `https://console.cloud.google.com/auth/scopes?project=<PROJECT>`.

Click **Add or Remove Scopes**. The picker filters by enabled APIs; if a scope doesn't show up, scroll to the **"Manually add scopes"** textarea at the bottom and paste the URL there.

Add only the scopes for services you'll use. These strings are pulled verbatim from arawn's integration crates — they need to match exactly:

```
# Gmail (3 scopes)
https://www.googleapis.com/auth/gmail.readonly
https://www.googleapis.com/auth/gmail.send
https://www.googleapis.com/auth/gmail.modify

# Calendar (1 scope)
https://www.googleapis.com/auth/calendar.events

# Drive (1 scope — see warning below)
https://www.googleapis.com/auth/drive
```

Click **Update**, then **Save**.

> **Drive scope warning:** arawn requests the **full read+write** Drive scope (`/auth/drive`), not `drive.readonly`. This is intentional — the v1 Drive tools include upload/update/delete; downgrading to readonly would silently disable half of them at runtime. If you want a read-only Drive surface, don't connect Drive yet; track [I-0037 follow-ups](../explanation/integrations-overview.md) for a future scope-set toggle.

## 5. Add yourself as a test user

<!-- VERIFY: 2026-05-20 — Google Auth Platform → Audience section path. -->

Left nav → **Google Auth Platform → Audience**. Under **Test users**, click **+ Add Users** and enter the Google account you'll be connecting. Without this, the consent screen refuses access for non-test-user accounts.

## 6. Create the OAuth client

Left nav → **APIs & Services → Credentials**. Direct URL: `https://console.cloud.google.com/apis/credentials?project=<PROJECT>`.

**Create Credentials → OAuth client ID:**

- **Application type:** Desktop app.
- **Name:** anything (`arawn desktop` works).

Click Create. Copy the **Client ID** and **Client secret**.

> **Why Desktop type, not Web?** arawn's callback server binds to a localhost port chosen at runtime. The Desktop client type accepts any localhost redirect; the Web type requires the exact redirect URI to be registered in advance. See the [OAuth primer](../explanation/oauth-primer.md#the-four-moving-parts) for the redirect-URI mechanics.

## 7. Paste into arawn.toml (or set env vars)

The recommended default — one OAuth client shared across all three Google services:

```toml
# ~/.arawn/arawn.toml

[integrations.google]
client_id     = "955517163683-xxxxxxxxxxxxxxxxxxxxxxxx.apps.googleusercontent.com"
client_secret = "GOCSPX-xxxxxxxxxxxxxxxxxxxxxxxx"
```

If you'd rather keep the secret out of `arawn.toml`, the env-var path works too:

```toml
[integrations.google]
client_id = "955517163683-xxxxxxxxxxxxxxxxxxxxxxxx.apps.googleusercontent.com"
# client_secret comes from ARAWN_GOOGLE_CLIENT_SECRET in the shell
```

```sh
export ARAWN_GOOGLE_CLIENT_SECRET="GOCSPX-xxxxxxxxxxxxxxxxxxxxxxxx"
```

For per-service OAuth clients (`[integrations.gmail]` / `[integrations.calendar]` / `[integrations.drive]`) or other config shapes, see the [integrations config reference](../reference/integrations-config.md).

## 8. Restart the server and connect

In the terminal running `arawn serve`, hit `Ctrl+C` and restart:

```sh
arawn serve
```

In the TUI:

```
/connect gmail
/connect google_calendar
/connect google_drive
```

For each `/connect`:

1. Your browser opens to Google's consent screen.
2. Sign in using the Google account you added as a test user in step 5.
3. You'll see "**Google hasn't verified this app**". Click **Advanced → Go to `<app name>` (unsafe)**. This is expected for unverified apps; the consent screen still lets you proceed.
4. Click **Allow**.
5. The browser shows arawn's success page; close the tab.
6. The TUI shows `ℹ [integration] connected: <service>`.

## 9. Verify

```
/integrations
```

Every connected service should show as `connected`. Then send the agent a real prompt:

| Service | Test prompt |
|---|---|
| Gmail | `list the last 5 emails in my inbox` |
| Calendar | `what's on my calendar this week?` |
| Drive | `list the files in my Drive root` |

## Troubleshooting

If the consent screen errors out or `/connect` doesn't work, see [debug OAuth failures](./debug-oauth-failures.md) for the symptom → cause → fix table.

A few Google-specific gotchas worth pre-empting:

- **`access_denied` on the consent screen.** You're not on the test-user list (step 5), or you signed in with a different Google account than the one you added. Add the right account.
- **A scope you expect doesn't appear in the picker.** The API for it isn't enabled (step 2). Enable it, refresh the consent-screen page, retry.
- **After changing scopes in the consent screen, the new scopes don't take effect.** Google's token grant is cached against the original scope set. `/disconnect <svc>` and `/connect <svc>` to issue a new grant. If even that doesn't work, revoke arawn at <https://myaccount.google.com/permissions> and reconnect.

## What's next

- [Integrations overview](../explanation/integrations-overview.md) — what arawn does with the tokens, where they live.
- [Integrations reference](../reference/integrations.md) — every tool that lands for each Google service.
- [Bind a lens to a feed](./bind-a-lens-to-a-feed.md) — turn your Gmail/Drive into a local knowledge base.

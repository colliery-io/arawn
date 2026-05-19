# Connect Google (Gmail, Calendar, Drive)

*How-to. ~10 minutes. One Google Cloud project covers all three services.*

By the end of this guide arawn will have OAuth tokens for Gmail, Calendar, and Drive, and the agent will be able to list emails, check your calendar, and read Drive files.

## Prerequisites

- arawn server running ([first chat session](../tutorials/first-chat.md)).
- A Google account.
- About 10 minutes in front of <https://console.cloud.google.com/>.

## 1. Create or pick a Google Cloud project

Go to <https://console.cloud.google.com/>. Create a new project (any name) or pick an existing one. Note the **project number** in the dashboard — you'll use it in URLs below.

## 2. Enable the APIs you want

You only need to enable the APIs for the services you'll actually use. Replace `<PROJECT>` in the URLs with your project number:

- Gmail: <https://console.cloud.google.com/apis/library/gmail.googleapis.com>
- Calendar: <https://console.cloud.google.com/apis/library/calendar-json.googleapis.com>
- Drive: <https://console.cloud.google.com/apis/library/drive.googleapis.com>

Click **Enable** on each. If the button says "Manage", it's already enabled.

> **Note:** the OAuth scope picker in the next step only shows scopes for *enabled* APIs. If a scope you expect doesn't appear, double-check you enabled the API first.

## 3. Configure the OAuth consent screen

Left nav → **Google Auth Platform → Branding** (the menu was renamed from "OAuth consent screen" in late 2024). Direct URL: `https://console.cloud.google.com/auth/branding?project=<PROJECT>`.

- **User type:** External (unless you're inside a Google Workspace org and only want it for that org's users).
- **App name:** anything (`arawn-personal` works).
- **User support email:** your email.
- **Developer contact:** your email.

Save.

> **Note:** your app will be in **Testing mode** by default. Google warns anyone who connects that "this app is unverified". That's fine for personal use; you're capped at 100 test users (yourself + anyone you explicitly add). Verification is only needed if you want to ship arawn to strangers.

## 4. Add OAuth scopes

Left nav → **Google Auth Platform → Data Access**. Direct URL: `https://console.cloud.google.com/auth/scopes?project=<PROJECT>`.

Click **Add or Remove Scopes**. The picker filters by enabled APIs; if a scope doesn't show up, scroll to **"Manually add scopes"** at the bottom and paste the URL.

Add only the scopes for services you'll use:

```
# Gmail
https://www.googleapis.com/auth/gmail.readonly
https://www.googleapis.com/auth/gmail.send
https://www.googleapis.com/auth/gmail.modify

# Calendar
https://www.googleapis.com/auth/calendar.events

# Drive (full read+write — arawn defaults to this so upload/update/delete work)
https://www.googleapis.com/auth/drive
```

Click **Update**, then **Save**.

## 5. Add yourself as a test user

Left nav → **Google Auth Platform → Audience**. Under **Test users**, click **+ Add Users** and enter the Google account you'll be connecting. Without this, the consent screen refuses access.

## 6. Create the OAuth client

Left nav → **APIs & Services → Credentials**. Direct URL: `https://console.cloud.google.com/apis/credentials?project=<PROJECT>`.

**Create Credentials → OAuth client ID:**

- **Application type:** Desktop app.
- **Name:** anything (`arawn desktop` works).

Click Create. Copy the **Client ID** and **Client secret**.

> **Note:** no redirect URI configuration is needed for Desktop apps — Google accepts any localhost callback automatically.

## 7. Paste into arawn.toml

```toml
# ~/.arawn/arawn.toml

# One Google OAuth client shared across Gmail, Calendar, Drive.
[integrations.google]
client_id = "955517163683-xxxxxxxxxxxxxxxxxxxxxxxx.apps.googleusercontent.com"
client_secret = "GOCSPX-xxxxxxxxxxxxxxxxxxxxxxxx"
```

If you'd rather use isolated OAuth clients per service, use `[integrations.gmail]`, `[integrations.calendar]`, `[integrations.drive]` instead. The shared `[integrations.google]` block is the recommended default.

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
2. Sign in (use the Google account you added as a test user).
3. Click **Allow** — accept the unverified-app warning via **Advanced → Go to `<app name>`**.
4. The browser shows a success page; close the tab.
5. The TUI shows `ℹ [integration] connected: <service>`.

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

If something failed, see [debug OAuth failures](./debug-oauth-failures.md) for the common symptom-cause-fix table.

## What's next

- Bind a feed to a workstream and watch the agent build a knowledge graph from your inbox: [your first workstream](../tutorials/first-workstream.md).
- See every Gmail/Calendar/Drive tool the agent has: [integrations reference](../reference/integrations.md).
- Encrypted token storage and scope rationale: [security model explanation](../explanation/permission-model.md).

# Getting Started

This walkthrough takes you from `git clone` to a working chat session.
Estimated time: ten minutes if you already have a Rust toolchain and an LLM
provider account.

## Prerequisites

- **Rust toolchain** — `rustup` with a stable Rust ≥ 1.80 (`rustup default stable`).
- **An LLM provider** — pick one:
  - **Groq** (free tier, fast) — sign up at [console.groq.com](https://console.groq.com),
    create an API key.
  - **Ollama Cloud** (paid for some models) — sign up at
    [ollama.com](https://ollama.com), create an API key. Lets you reach large
    open models without local GPU.
  - **Local Ollama** — install [ollama](https://ollama.com), `ollama pull
    llama3.1:8b`. No API key needed.
  - **OpenAI**, **Anthropic**, or anything OpenAI-compatible (vLLM,
    LM Studio, Together, Fireworks, Mistral, ...) — works the same way.

## 1. Build

```sh
git clone <repo-url> arawn
cd arawn
cargo build --release
```

The binary lands at `target/release/arawn`. Either add it to your `PATH` or
invoke it by full path in the steps below.

## 2. Configure

Arawn reads `~/.arawn/arawn.toml`. Pick the example matching your provider:

### Option A: Groq

```toml
# ~/.arawn/arawn.toml

[llm.default]
provider = "groq"
model = "openai/gpt-oss-120b"
api_key_env = "GROQ_API_KEY"

[engine]
llm = "default"

[server]
host = "127.0.0.1"
port = 3100

[storage]
data_dir = "~/.arawn"
```

```sh
export GROQ_API_KEY=gsk_your_key_here
```

Or skip the env var entirely and put the key directly in the config:

```toml
[llm.default]
provider = "groq"
model = "openai/gpt-oss-120b"
api_key = "gsk_your_key_here"
```

`api_key` (direct value in TOML) takes precedence over `api_key_env` (env var name) when both are set.

### Option B: Ollama Cloud

```toml
# ~/.arawn/arawn.toml

[llm.default]
provider = "https://ollama.com/v1"
model = "gemma4:31b-cloud"
api_key_env = "OLLAMA_API_KEY"

[engine]
llm = "default"

[server]
host = "127.0.0.1"
port = 3100

[storage]
data_dir = "~/.arawn"
```

```sh
export OLLAMA_API_KEY=your_key_here
```

> Available cloud models depend on your account. `gemma4:31b-cloud`,
> `qwen3-coder:480b-cloud`, `gpt-oss:120b-cloud`, and `deepseek-v3.1:671b-cloud`
> are commonly available; some models (e.g. `deepseek-v4-pro:cloud`) require
> a subscription. If a model is unavailable, arawn's startup warmup will tell
> you immediately — see [Troubleshooting](#troubleshooting) below.

### Option C: Local Ollama

```toml
[llm.default]
provider = "ollama"
model = "llama3.1:8b"
api_key_env = ""    # local Ollama needs no auth

[engine]
llm = "default"
```

No env var needed. Make sure `ollama serve` is running.

## 3. Start the server

```sh
arawn serve
```

Watch for these lines:

```
INFO arawn: LLM client pool ready ... engine_model=...
INFO arawn: LLM warmup OK name=default provider=... model=...
```

If you see `ERROR LLM warmup failed`, jump to [Troubleshooting](#troubleshooting).

Leave this terminal open. The server holds your session state.

## 4. Open the TUI

In a second terminal:

```sh
arawn tui
```

You'll see an input area at the bottom of the screen. Type a message and
press `Enter`.

## 5. First message

Try something concrete:

```
What's in this directory?
```

The agent will use its `shell` tool to run `ls`, then summarize. You should
see streaming text appear, then the tool call indicator, then the result.

You're set. From here:

- Read about [the available tools](./intro.md#whats-included) (work in progress).
- Try a more involved prompt: `Find any TODO comments in this repo and group them by file`.
- Press `Ctrl+C` in the TUI to quit; the server keeps running.

## 6. Connect integrations

Arawn integrates with Google (Gmail / Calendar / Drive), Slack, and Atlassian (Jira / Confluence). For each one, you have to:

1. Create an OAuth app in the provider's developer console.
2. Paste the `client_id` / `client_secret` into `~/.arawn/arawn.toml`.
3. Run `/connect <service>` in the TUI, approve the consent screen in your browser.

Why so hands-on: every provider requires apps to be registered before they'll hand out access to user data. Arawn doesn't ship a pre-registered shared app today (see ARAWN-I-0037 for the long-term plan).

Once connected, tokens are stored encrypted under `~/.arawn/tokens/` and refreshed silently. Tokens never leave your machine.

### Google — Gmail, Calendar, and Drive (one app for all three)

The same Google Cloud project covers all three integrations. Set it up once.

**Time:** ~10 minutes.

#### 1. Create / pick a Google Cloud project

Go to <https://console.cloud.google.com/>. Create a new project (any name) or pick an existing one. Note the **project number** in the dashboard — you'll use it in URLs below.

#### 2. Enable the APIs you want

You only need to enable the APIs for the services you'll use. Replace `<PROJECT>` in the URLs with your project number:

- Gmail: `https://console.cloud.google.com/apis/library/gmail.googleapis.com?project=<PROJECT>`
- Calendar: `https://console.cloud.google.com/apis/library/calendar-json.googleapis.com?project=<PROJECT>`
- Drive: `https://console.cloud.google.com/apis/library/drive.googleapis.com?project=<PROJECT>`

Click **Enable** on each. If the button says "Manage", it's already enabled.

> ⚠️ The OAuth scope picker (next step) only shows scopes for **enabled APIs**. If a scope you expect doesn't appear, double-check you enabled the API first.

#### 3. Configure the OAuth consent screen

Left nav → **Google Auth Platform → Branding** (the menu was renamed from "OAuth consent screen" in late 2024). If you don't see "Google Auth Platform", direct URL: `https://console.cloud.google.com/auth/branding?project=<PROJECT>`.

- **User type:** External (unless you're inside a Google Workspace org and only want it for that org's users).
- **App name:** anything ("arawn-personal" works).
- **User support email:** your email.
- **Developer contact:** your email.

Save.

> Your app will be in **Testing mode** by default — Google will warn anyone who connects "this app is unverified". That's fine for personal use; you're capped at 100 test users (i.e. yourself + anyone else you explicitly add). Verification is only needed if you want to ship arawn to strangers.

#### 4. Add OAuth scopes

Left nav → **Google Auth Platform → Data Access**. Direct URL: `https://console.cloud.google.com/auth/scopes?project=<PROJECT>`.

Click **Add or Remove Scopes**. The picker filters by enabled APIs. If a scope doesn't show up, scroll to the bottom of the panel and use **"Manually add scopes"** — paste the URL and click "Add to Table".

Scopes to add (only add the ones for services you'll use):

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

#### 5. Add yourself as a test user

Left nav → **Google Auth Platform → Audience**. Under **Test users**, click **+ Add Users** and enter the Google account you'll be connecting. Without this, the consent screen will refuse access.

#### 6. Create the OAuth client

Left nav → **APIs & Services → Credentials**. Direct URL: `https://console.cloud.google.com/apis/credentials?project=<PROJECT>`.

**Create Credentials → OAuth client ID:**

- **Application type:** Desktop app.
- **Name:** anything ("arawn desktop" works).

Click Create. Copy the **Client ID** and **Client secret** — that's what you put in arawn.

> No redirect URI configuration needed for Desktop apps — Google accepts any localhost callback automatically.

#### 7. Paste into arawn.toml

```toml
# ~/.arawn/arawn.toml

# One Google OAuth client shared across Gmail, Calendar, Drive.
[integrations.google]
client_id = "955517163683-xxxxxxxxxxxxxxxxxxxxxxxx.apps.googleusercontent.com"
client_secret = "GOCSPX-xxxxxxxxxxxxxxxxxxxxxxxx"
```

If you'd rather use isolated OAuth clients per service, use `[integrations.gmail]`, `[integrations.calendar]`, `[integrations.drive]` instead. The shared `[integrations.google]` block is the recommended default.

#### 8. Restart the server and connect

```sh
# In the terminal running arawn serve, Ctrl+C, then:
arawn serve
```

In the TUI:

```
/connect gmail
/connect google_calendar
/connect google_drive
```

Each `/connect`:

1. Opens your browser to Google's consent screen.
2. You sign in (use the Google account you added as a test user).
3. Click "Allow" — accept the unverified-app warning by clicking "Advanced → Go to `<app name>`".
4. Browser shows a success page; you can close the tab.
5. TUI shows `ℹ [integration] connected: <service>`.

Run `/integrations` in the TUI to confirm all three show as connected.

### Slack

**Time:** ~10 minutes.

#### 1. Create a Slack app

Go to <https://api.slack.com/apps> → **Create New App → From scratch**.

- **Name:** anything ("arawn-personal" works).
- **Workspace:** the workspace you want arawn to operate in. (Multi-workspace support is on the roadmap as ARAWN-I-0034.)

#### 2. Add OAuth scopes

In the app's settings, **OAuth & Permissions → Scopes**.

Add these **Bot Token Scopes**:

```
channels:history
channels:read
chat:write
chat:write.public
files:read
groups:history
groups:read
im:history
im:read
im:write
mpim:history
mpim:read
mpim:write
users:read
users:read.email
```

Add these **User Token Scopes** (yes, both — Slack's dual-token model means some tools need user-level access to private channels you've joined):

```
channels:history
channels:read
groups:history
groups:read
im:history
im:read
mpim:history
mpim:read
search:read
```

#### 3. Set the redirect URI

In **OAuth & Permissions → Redirect URLs**, click **Add New Redirect URL** and enter exactly:

```
http://localhost:8080/oauth/callback
```

> ⚠️ Use `localhost`, not `127.0.0.1`. Slack does string comparison and rejects `127.0.0.1` even though it resolves to the same address. The port is fixed at 8080 because Slack's allowlist is exact-match (no wildcards) — make sure 8080 is free on your machine when you `/connect`.

Save URLs.

#### 4. Install the app to your workspace

In the app's settings, **Install App → Install to `<Workspace>`**. Slack will show a consent screen with the scopes you asked for. Approve.

After install, you can copy the bot/user tokens from this page if you want — but arawn doesn't use them directly. Arawn does its own OAuth dance via `/connect slack` to get its own tokens.

#### 5. Get the client_id / client_secret

In the app's settings, **Basic Information → App Credentials**. Copy:

- **Client ID** (looks like `2130966322213.11049839823699`)
- **Client Secret** (32-char hex string)

#### 6. Paste into arawn.toml

```toml
[integrations.slack]
client_id = "2130966322213.11049839823699"
client_secret = "262f1cf7e5773131e68c7b61df992a1b"
```

#### 7. Restart and connect

```sh
arawn serve   # restart
```

In TUI:

```
/connect slack
```

Browser opens, you re-approve (this time it's arawn's own OAuth dance, not the install flow). After success, run `/integrations` to confirm.

> If you change scopes later, you must **re-install the app** in step 4 *and* run `/disconnect slack` then `/connect slack` again. Slack's tokens are scope-locked at issue time.

### Atlassian — Jira and Confluence

**Time:** ~10 minutes.

#### 1. Create the OAuth 2.0 (3LO) integration

Go to <https://developer.atlassian.com/console/myapps/> → **Create → OAuth 2.0 integration**.

- **Name:** "arawn-personal" works.

#### 2. Add APIs and scopes

In the app's settings, **Permissions** tab. For each API you want to use, click **Add** then **Configure**.

**Jira API** — add scopes:

```
read:jira-user
read:jira-work
write:jira-work
```

**Confluence API** — add scopes:

```
read:confluence-content.all
write:confluence-content
read:confluence-space.summary
```

> Atlassian also has **classic** and **granular** scope versions. The above are the classic scopes which arawn uses. If a scope above doesn't appear, look in the "Classic scopes" section.

#### 3. Set the callback URL

**Authorization** tab → **Callback URL**:

```
http://localhost:8080/oauth/callback
```

Same fixed-port-8080 rule as Slack.

Save.

#### 4. Get the client_id / client_secret

**Settings** tab → **Authentication details**. Copy:

- **Client ID**
- **Secret**

#### 5. Paste into arawn.toml

```toml
[integrations.atlassian]
client_id = "your-client-id"
client_secret = "your-client-secret"
```

#### 6. Restart and connect

```sh
arawn serve
```

In TUI:

```
/connect atlassian
```

Atlassian's consent screen will ask which **site** (your Jira/Confluence cloud instance) to grant access to. Pick the one you want. Arawn discovers the underlying `cloud_id` automatically after consent.

### Verifying integrations work

Run `/integrations` in the TUI — every connected service should show:

```
gmail            connected   (5 tools)
google_calendar  connected   (3 tools)
google_drive     connected   (7 tools)
slack            connected   (6 tools)
atlassian        connected   (11 tools)
```

Then try a real prompt for each:

| Service | Test prompt |
|---|---|
| Gmail | `list the last 5 emails in my inbox` |
| Calendar | `what's on my calendar this week?` |
| Drive | `list the files in my Drive root` |
| Slack | `list my Slack channels` |
| Atlassian | `show me my open Jira issues` |

### Common integration errors

| Error | Cause | Fix |
|---|---|---|
| `Error 400: redirect_uri_mismatch` | Provider's allowed redirect URI doesn't match what arawn requested | Re-check the redirect URI in the provider console matches the value in the doc above (Slack/Atlassian: exactly `http://localhost:8080/oauth/callback`; Google Desktop apps: any localhost). |
| `Error 403: access_denied` | You clicked Deny, or your account isn't in the test-users list | Add yourself as a test user in the consent screen settings. |
| `[integration] error: insufficient_scope` | You connected with one scope set; the agent is trying a tool that needs another | Add the missing scope in the provider console, run `/disconnect <svc>` then `/connect <svc>`. |
| `[integration] error: invalid_grant` | Stored refresh token expired or was revoked | `/disconnect <svc>` then `/connect <svc>` to re-auth. |
| `Connection error: failed to reach <host>` | API not enabled in the provider console, or network issue | For Google, confirm the relevant API is enabled in the **Library** (Gmail / Calendar / Drive each need their own enable click). |
| `[integration] connected` but tools error with permission issues | Token cache vs scope mismatch — common after adding new scopes | Revoke at the provider's permissions page (e.g. <https://myaccount.google.com/permissions>) then `/connect <svc>` fresh. |
| `Address already in use (port 8080)` during Slack/Atlassian connect | Something else is listening on 8080 | Stop the conflicting process, or wait a few seconds and retry — the bound socket may still be in TIME_WAIT. |

## 7. Continual data feeds (optional)

Once an integration is connected, you can have arawn mirror a slice of
its state locally — a Slack channel, a Gmail label, a Drive folder.
The agent then answers questions about that data by reading the local
mirror instead of round-tripping to the provider every time.

The simplest path: connecting Gmail, Drive, Calendar, Slack, or
Atlassian auto-creates a personal feed (your inbox, your recent Drive
files, your upcoming calendar, your `@me` mentions, your assigned Jira
issues). No setup needed beyond `/connect`.

For everything else, use `/watch`:

```
/watch slack/channel-archive design
/watch jira/project-tracker ENG
/watch drive/folder-sync Reports/2026
/watch gmail/sender-filter alerts@oncall.example.com
```

`/feeds` lists what's running. `/unwatch <feed_id>` removes one.
`/watch list <template>` shows what's available for that template
(channels, folders, projects).

Data lands under `~/.arawn/data/<provider>/<template>/<feed_id>/`. See
the [Continual Data Feeds](./feeds/index.md) reference for the full
story, including the [template catalog](./feeds/template-catalog.md)
and [agent read patterns](./feeds/agent-read-patterns.md).

## 8. Workstream palaces (optional)

Feeds give you raw content. A **workstream palace** is the curated
layer on top — a typed knowledge graph the agent builds about one
specific thing you track (a project, a person, a campaign). Bind a
feed to a workstream and the extractor turns each new projection row
into typed entities (decisions, conventions, facts, ...) with tags
drawn from a per-workstream ontology.

Quick start:

```
/workstream create work --description "Pat's day job — Acme platform team"
```

The agent walks you through proposing an initial tag ontology (5–12
tags), confirms with you, and creates the workstream. Then bind a
feed:

```
/workstream switch work
/workstream bind work fixture-work-gmail   # or any feed_id from /feeds
```

Extraction starts on the next feed run (or immediately for already-
mirrored rows via the backfill loop). Query the resulting palace
with the [signal_* tools](./palaces/agent-read-patterns.md):

```
signal_search "what did we decide about postgres?"
signal_query  { entity_type: "decision", since: "2026-04-01T00:00:00Z" }
signal_timeline { limit: 10 }
```

Periodically the steward proposes maintenance — ontology growth via
the [tag-promoter](./palaces/steward.md#extract-suggest-add), new
relations, dust summaries of cold material. Review with
`workstream_refine`, commit with `workstream_apply <id>`, undo with
`workstream_rollback <id>`.

Full story: [Workstream Palaces](./palaces/index.md). Read the
[index](./palaces/index.md) for the three-layer mental model, then
[extraction](./palaces/extraction.md) and [steward](./palaces/steward.md)
for the working pieces.

## CLI one-shot mode

Once the server is running, you can also send single prompts without the TUI:

```sh
arawn "draft a one-line commit message for the diff in this repo"
```

This streams a single response and exits. Useful for shell scripts.

## Troubleshooting

### "ERROR LLM warmup failed" at server startup

Arawn probes your configured model on startup. If it fails, the server log
shows the upstream error body. Common cases:

| Error body contains | Cause | Fix |
|---|---|---|
| `HTTP 401` / `HTTP 403` | API key missing or invalid | Verify `api_key_env` matches the var you exported, and that the key itself is correct. |
| `HTTP 403: subscription required` | Model needs a paid plan you don't have | Pick a different model or upgrade. |
| `HTTP 404: model … not found` | Model name typo or unavailable on this provider | Check the provider's model list. |
| `Connection refused` | Provider URL wrong, or local Ollama not running | Check `provider` URL; `ollama serve` if local. |

The server keeps running after a failed warmup — your *next* message will
trigger a fresh warmup attempt and the engine will surface the same error
to the TUI if it still fails.

### "embedding model unavailable — memory system will use FTS only"

The memory system uses sentence embeddings for semantic search. If the
embedder model isn't available locally, arawn falls back to keyword/FTS
search — it still works, just less smart.

To fix: install the model file at the path arawn expects (default
`~/.arawn/models/all-MiniLM-L6-v2/model.onnx`). The model isn't bundled to
keep the binary small. (We'll automate this in a future release.)

### TUI shows "connection refused" or hangs

The TUI talks to the server over WebSocket. Make sure `arawn serve` is
running in another terminal and listening on the same port (default 3100).
If you changed `[server].port` in `arawn.toml`, point the TUI at it:

```sh
arawn tui --url ws://127.0.0.1:<your-port>/ws
```

### "no API key set for ENVVAR"

You declared `api_key_env = "FOO"` in `arawn.toml` but didn't `export FOO=…`
in the shell where you ran `arawn serve`. Either export it, or set
`api_key_env = ""` if your provider doesn't need a key (local Ollama).

## Next steps

- Resume an old session: `arawn --list-sessions`, then `arawn --session <uuid>`.
- See what tools the agent has: in the TUI, type `/help` (some commands
  are work-in-progress — see backlog T-0195).
- Build a scheduled workflow: see the workflow docs (work in progress —
  see backlog T-0198).

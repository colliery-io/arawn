# Configure LLM Backends

This guide walks through configuring LLM providers in Arawn, including API key management, named profiles, and agent-specific backend assignment.

## Prerequisites

- Arawn installed and initialized
- An API key for at least one provider (or a local Ollama installation)
- Access to `~/.config/arawn/config.toml` (or your project-level `.arawn/config.toml`)

## Set up a single backend

Open your config file and add an `[llm]` section with the backend and model:

```toml
[llm]
backend = "anthropic"
model = "claude-sonnet-4-20250514"
max_context_tokens = 200000
```

This sets the global default. Every agent uses this backend unless overridden.

## Configure each provider

### Anthropic

```toml
[llm]
backend = "anthropic"
model = "claude-sonnet-4-20250514"
max_context_tokens = 200000
```

Set your API key:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

Other models: `claude-3-haiku-20240307` (fast, cheap), `claude-opus-4-20250514` (complex reasoning).

### OpenAI

```toml
[llm]
backend = "openai"
model = "gpt-4o"
max_context_tokens = 128000
```

```bash
export OPENAI_API_KEY="sk-..."
```

Other models: `gpt-4o-mini` (cost-effective), `gpt-4-turbo` (high capability).

### Groq

```toml
[llm]
backend = "groq"
model = "llama-3.1-8b-instant"
max_context_tokens = 32768
```

```bash
export GROQ_API_KEY="gsk_..."
```

Other models: `llama-3.3-70b-versatile` (general purpose), `mixtral-8x7b-32768` (long context).

### Ollama (local)

```toml
[llm]
backend = "ollama"
base_url = "http://localhost:11434"
model = "llama3"
```

No API key is required by default. If your Ollama instance requires authentication, set `OLLAMA_API_KEY`. To point at a remote Ollama server, change `base_url`:

```toml
[llm]
backend = "ollama"
base_url = "http://gpu-server.local:11434"
model = "llama3:70b"
```

### Custom (OpenAI-compatible endpoint)

Any endpoint that speaks the OpenAI API protocol:

```toml
[llm]
backend = "custom"
base_url = "https://my-endpoint.example.com/v1"
model = "my-model"
max_context_tokens = 32768
```

```bash
export LLM_API_KEY="your-key-here"
```

### Claude OAuth

Use a Claude MAX subscription instead of an API key. No key is needed — authentication happens through a browser-based OAuth PKCE flow:

```toml
[llm]
backend = "claude-oauth"
model = "claude-sonnet-4-20250514"
max_context_tokens = 200000
```

Run the login flow:

```bash
arawn auth login
```

Your browser opens, you authorize Arawn, and the token is stored automatically.

> **Limitation:** The `claude-oauth` backend is tightly coupled to the Claude Code
> execution framework. It supports **conversation only** — tool use is not available.
> This means it cannot be used for agentic workflows (tool calling, file operations,
> shell commands, etc.). It is still useful for pure conversation tasks where you
> provide sufficient context in the prompt, but for full agent capabilities use the
> `anthropic` backend with an API key instead.
>
> **Warning:** Using the Claude OAuth backend outside of Claude Code may violate
> Anthropic's Terms of Service. The Arawn project assumes no responsibility for
> account restrictions or bans resulting from its use. Proceed at your own risk.

## Create named profiles

Named profiles let you configure multiple backends and assign them to different agents. Define them as `[llm.<profile-name>]` sections:

```toml
# Global default (used when no profile is specified)
[llm]
backend = "anthropic"
model = "claude-sonnet-4-20250514"
max_context_tokens = 200000

# A profile named "fast" for quick tasks
[llm.fast]
backend = "groq"
model = "llama-3.3-70b-versatile"
max_context_tokens = 32768

# A profile named "claude" for complex work
[llm.claude]
backend = "anthropic"
model = "claude-opus-4-20250514"
max_context_tokens = 200000

# A profile named "local" for offline use
[llm.local]
backend = "ollama"
base_url = "http://localhost:11434"
model = "llama3"
```

## Assign profiles to agents

Reference named profiles from agent configuration using the profile name:

```toml
[agent.default]
llm = "claude"

[agent.summarizer]
llm = "fast"

[agent.researcher]
llm = "claude"
```

### Resolution order

When Arawn resolves which LLM to use for a given agent, it checks in this order:

1. **Agent-specific** -- `[agent.<name>].llm` points to a named profile
2. **Agent default** -- `[agent.default].llm` points to a named profile
3. **Global default** -- the `[llm]` section itself

For example, with the config above, the `summarizer` agent uses the `fast` profile (Groq), while an unnamed agent falls through to `agent.default` and uses the `claude` profile.

## Manage secrets securely

Avoid putting API keys directly in your config file. Arawn resolves keys using a priority chain:

| Priority | Source | Method |
|----------|--------|--------|
| 1 (highest) | Age-encrypted store | `arawn secrets set <name>` |
| 2 | System keyring | `arawn secrets set <backend>` (legacy) |
| 3 | Environment variable | `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc. |
| 4 (lowest) | Config file | Plaintext `api_key` field (not recommended) |

The `--api-key` CLI flag on `arawn start` overrides all of the above.

### Store a key in the encrypted secret store

Use the built-in command:

```bash
arawn secrets set anthropic
```

You will be prompted for the key. It is encrypted with `age` and stored at `~/.config/arawn/secrets/`.

### Store a key in the system keyring (legacy)

```bash
arawn secrets set anthropic
```

You will be prompted for the key. It is stored in your OS keychain (macOS Keychain, Linux Secret Service, or Windows Credential Manager).

### Use environment variables

Set the appropriate variable for your backend:

| Backend | Variable |
|---------|----------|
| Anthropic | `ANTHROPIC_API_KEY` |
| OpenAI | `OPENAI_API_KEY` |
| Groq | `GROQ_API_KEY` |
| Ollama | `OLLAMA_API_KEY` |
| Custom | `LLM_API_KEY` |

For persistent use, add the export to your shell profile (`~/.zshrc`, `~/.bashrc`).

## Configure retry behavior

Control how Arawn handles transient failures from LLM providers:

```toml
[llm]
backend = "anthropic"
model = "claude-sonnet-4-20250514"
max_context_tokens = 200000
retry_max = 3
retry_backoff_ms = 1000
```

- **`retry_max`** -- Maximum number of retry attempts for failed requests.
- **`retry_backoff_ms`** -- Delay in milliseconds between retries (applied with exponential backoff).

These options work on any backend and can be set per-profile:

```toml
[llm.fast]
backend = "groq"
model = "llama-3.1-8b-instant"
retry_max = 5
retry_backoff_ms = 500
```

## Switch backends at runtime

### Via CLI flag on `arawn start`

The `--backend` flag is available on the `arawn start` command to override the configured backend when launching the server:

```bash
arawn start --backend anthropic
arawn start --backend ollama
```

This overrides the `[llm].backend` value from your config file for that server session.

## Verify your configuration

After setting up a backend, confirm it works:

```bash
arawn ask "hello"
```

You should see a response from the configured model. If authentication fails:

1. Confirm the key is set: `echo $ANTHROPIC_API_KEY`
2. Confirm the model name is valid and available on your plan
3. For Ollama, confirm the server is running: `ollama list`
4. Check Arawn logs for detailed error messages: `arawn logs -f`

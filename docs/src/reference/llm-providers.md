# LLM providers

*Reference. Provider-specific notes for the LLMs arawn can talk to via OpenAI-compatible APIs.*

Arawn talks to LLMs through an OpenAI-compatible HTTP interface. Most providers Just Work; this page documents the few quirks and model-selection guidance you need when picking one.

## Groq

Groq's API runs strict server-side validation on the model's tool-call output before returning it as structured `tool_calls`. When a Llama/Qwen/gpt-oss model emits a tool call in its NATIVE family format instead of the OpenAI structured shape, Groq returns an HTTP 502 with:

```json
{
  "error": {
    "message": "Failed to call a function. Please adjust your prompt.",
    "type": "backend_error",
    "failed_generation": "<function=foo>{\"x\":1}</function>"
  }
}
```

Arawn surfaces the `failed_generation` content in its error messages (truncated to 2 KB), and `RetryClient` automatically resamples on this error — most retries succeed because model output is non-deterministic enough to land on the structured path the second time. You don't need to do anything special; it just works.

### Model stability table

When the failure is deterministic (the model **always** emits its native format on a given prompt), retry can't recover. Pick the model accordingly.

| Model | Stability with retry | Recommendation |
|---|---|---|
| `qwen/qwen3-32b` | Best. Occasional `<tool_call>` wrap; retry recovers reliably. | **Default for tool-call-heavy work on Groq.** |
| `openai/gpt-oss-120b` | Sporadic empty content (harmony channels). Usable but slower. | OK for casual sessions; not for high-throughput. |
| `llama-3.3-70b-versatile` | Retry-dependent; deterministic XML emission on some prompts. | Workable but expect failures. |
| `openai/gpt-oss-20b` | **Avoid.** Deterministically leaks `<\|channel\|>` tokens into tool names. Retry can't recover. | Not recommended. |

Findings adapted from a sister project (muninn) that hardened its Groq integration through prolonged UAT against tool-heavy workloads.

### What does NOT help

- **Lowering `temperature` to 0.** The format issue is output-bias, not sampling jitter. Models still emit their native format at `T=0`; the only effect is making the failure deterministic per-prompt.
- **Pinning `tool_choice` to a specific tool.** Doesn't change the model's preferred output template. (Arawn doesn't use `tool_choice` today anyway — the agent picks from the offered list freely.)
- **Increasing `max_tokens`.** Failures aren't truncation; the model emits a complete-but-wrong-format tool call.

The two real levers are **retry** (works, on by default) and **model choice** (works, configure via `[llm.<name>].model`).

## Anthropic

Direct API client (not OpenAI-compatible). Configure via `provider = "anthropic"` and `ANTHROPIC_API_KEY`. Returns `tool_use_failed` on malformed tool-call output — also retried automatically.

## Ollama (local + cloud)

OpenAI-compatible. For local, point `base_url` at your daemon (e.g. `http://localhost:11434/v1`). For Ollama Cloud, use `https://ollama.com/v1` and the model names that cloud surfaces (e.g. `gemma4:31b-cloud`).

## OpenAI

OpenAI-compatible (it's the reference impl). Configure via `provider = "openai"` and `OPENAI_API_KEY`. No special quirks.

## See also

- [Configuration schema](./config-schema.md) — `[llm.*]` entries and the field reference.
- [Environment variables](./env-vars.md) — `GROQ_API_KEY`, `ANTHROPIC_API_KEY`, etc.
- [Recover from LLM warmup failure](../how-to/recover-from-llm-warmup-failure.md) — diagnosing the boot-time probe error.

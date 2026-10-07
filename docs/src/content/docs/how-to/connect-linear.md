---
title: "Connect Linear"
---
*How-to. ~3 minutes. Linear takes a personal API key: no OAuth app, no browser flow.*

By the end of this guide arawn holds one Linear personal API key, encrypted at rest, and the `linear` integration is connected.

## 1. Create a personal API key

<!-- VERIFY: 2026-10-07 — linear.app Settings > Account > Security & access. -->

In Linear, go to **Settings → Account → Security & access** (<https://linear.app/settings/account/security>). Under **Personal API keys**, create a key named `arawn`. Give it full access, or at least **Read** and **Write**, and access to the teams arawn may see. Copy the key; it starts with `lin_api_`.

## 2. Give the key to arawn

```sh
arawn setup linear
```

Paste the key at the hidden prompt. arawn checks it with Linear's `viewer` query, then saves it encrypted in `<data_dir>/integrations/linear/`. Nothing is written to `arawn.toml`. A running server uses the key from its next call: there is no restart and no `arawn connect` step.

Check it:

```sh
arawn doctor    # integration:linear PASS
```

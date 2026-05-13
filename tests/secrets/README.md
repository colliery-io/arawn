# Test secrets

API keys for UAT and other test harnesses live in this directory as
sops-encrypted YAML. They are committed to git in ciphertext form;
each authorized developer can decrypt locally using their own AGE
private key. There is **no shared symmetric key** floating between
humans — onboarding is "you generate a keypair locally, you send me
your pubkey, I add it to the recipient list."

## One-time per developer setup

1. Install the tooling.

   macOS:
   ```sh
   brew install sops age
   ```

   Linux (Ubuntu/Debian):
   ```sh
   sudo apt install age
   # grab the latest sops release from https://github.com/getsops/sops/releases
   ```

2. Generate your AGE keypair.

   ```sh
   mkdir -p ~/.config/sops/age
   age-keygen -o ~/.config/sops/age/keys.txt
   ```

   The file contains both keys. Take the line that starts with
   `# public key: age1...` — that's the part you share.

3. Open a PR adding your pubkey to `.sops.yaml` at the repo root:

   ```yaml
   age:
     - age1existing0pubkey0from0maintainer  # existing recipient
     - age1your0new0pubkey0here              # you
   ```

   Get it reviewed + merged. The maintainer (or any current recipient)
   then runs `sops updatekeys tests/secrets/*.enc.yaml` and commits.

4. Once your pubkey is in the recipient list and the ciphertext has
   been refreshed, point sops at your private key and you're done:

   ```sh
   export SOPS_AGE_KEY_FILE=~/.config/sops/age/keys.txt
   ```

   Add that to your shell rc file so it persists.

## Editing a secret

```sh
sops edit tests/secrets/uat.enc.yaml
```

Opens the plaintext in `$EDITOR`; the ciphertext is rewritten on save.
Commit the resulting diff — sops's per-value encryption keeps the
file structure intact so PR review shows exactly which value changed.

## Rotating an API key

1. Rotate at the vendor (e.g. groq dashboard).
2. `sops edit tests/secrets/uat.enc.yaml`, paste the new value.
3. Commit + push. Other developers pull and decrypt with their
   existing AGE key — no human-to-human key exchange needed.

## Adding a new recipient (onboarding)

The dev being onboarded does steps 2–3 of "one-time per developer
setup" above. Any current recipient (maintainer or otherwise) merges
the PR and refreshes the ciphertext:

```sh
sops updatekeys tests/secrets/uat.enc.yaml
git add tests/secrets/uat.enc.yaml
git commit -m "secrets: add <dev> as recipient"
git push
```

`sops updatekeys` re-encrypts the data key to the new recipient list
without touching the underlying secret values.

## Removing a recipient

Delete the line from `.sops.yaml`. Run `sops updatekeys` and commit.
**Rotate any secret that recipient could have read** — they may have
already pulled the previous ciphertext to a local clone, and removing
them from the recipient list doesn't retroactively block that.

## Using a secret from the test harness

The angreal UAT task auto-detects `tests/secrets/uat.enc.yaml` and
wraps the cargo invocation in `sops exec-env`, so every key in the
file is exported as an environment variable to the test process.
Nothing manual to do beyond having `SOPS_AGE_KEY_FILE` set.

Manual invocation if you need it:

```sh
sops exec-env tests/secrets/uat.enc.yaml -- cargo test -p arawn-tests --test uat -- --ignored
```

## Schema

`tests/secrets/uat.enc.yaml` (plaintext form, before encryption):

```yaml
# Keys for UAT scenarios. Name them after the env var the harness
# will read — the angreal task injects every key here verbatim into
# the subprocess environment.
OLLAMA_API_KEY: "sk-..."
GROQ_API_KEY: "gsk-..."
ANTHROPIC_API_KEY: "sk-ant-..."
```

The `encrypted_regex` in `.sops.yaml` matches `*_KEY` / `*_TOKEN` /
`*_SECRET` / `*_PASSWORD` — non-matching values (like metadata
comments) stay plaintext so diffs are readable.

## Bootstrapping the first file

This repo doesn't ship a pre-encrypted `uat.enc.yaml` because there's
no first recipient pubkey baked in. To bootstrap from scratch:

```sh
# After your pubkey is in .sops.yaml:
cat > /tmp/uat.yaml <<'EOF'
OLLAMA_API_KEY: "your-key-here"
EOF
sops --encrypt /tmp/uat.yaml > tests/secrets/uat.enc.yaml
shred -u /tmp/uat.yaml         # or just rm — never commit the plaintext
git add tests/secrets/uat.enc.yaml
git commit -m "secrets: initial UAT key bundle"
```

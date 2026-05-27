# How-to guides

*Task-oriented. Recipes for solving specific problems. Assumes basic familiarity with arawn.*

Each how-to has one concrete goal and concrete steps. They're not for teaching; they're for getting something done. If you're new, work through the [tutorials](../tutorials/index.md) first.

## Connecting integrations

- **[Connect Google (Gmail, Calendar, Drive)](./connect-google.md)** — one Google Cloud project, shared OAuth client across three APIs.
- **[Connect Slack](./connect-slack.md)** — bot + user dual-token, fixed redirect URI on port 8080.
- **[Connect Atlassian (Jira + Confluence)](./connect-atlassian.md)** — 3LO OAuth, cloud-id auto-discovery.
- **[Connect GitHub](./connect-github.md)** — GitHub App model (not OAuth), App ID + slug + private key.

## Working with feeds and lenses

- **[Create a feed](./create-a-feed.md)** — `/watch` syntax and per-template parameters.
- **[Bind a lens to a feed](./bind-a-lens-to-a-feed.md)** — direct feed bind plus `github:repo:` and `github:org:` URI schemes.
- **[Curate a lens](./curate-a-lens.md)** — refine / apply / rollback flow over steward proposals.
- **[Read feeds with the agent](./read-feeds-with-the-agent.md)** — prompt patterns that get useful answers out of mirrored data.

## Configuration and runtime

- **[Lock down permissions](./lock-down-permissions.md)** — three named setups (paranoid, hands-off CI, strict review).
- **[Author a workflow by hand](./author-a-workflow-by-hand.md)** — JSON spec walkthrough for editing without the agent.

## Troubleshooting

- **[Debug OAuth failures](./debug-oauth-failures.md)** — every common error, with diagnosis steps.
- **[Recover from LLM warmup failure](./recover-from-llm-warmup-failure.md)** — provider-side symptoms and fixes.

## When how-tos aren't what you need

- If you need step-by-step learning from zero, see **[Tutorials](../tutorials/index.md)**.
- If you need to look something up, see **[Reference](../reference/index.md)**.
- If you want background and rationale, see **[Explanation](../explanation/index.md)**.

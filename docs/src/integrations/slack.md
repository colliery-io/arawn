# Slack

The Slack integration is **read/write**, not webhook-only. The agent can browse channel history, post, react, and DM users. Six tools land when configured: `slack_channels_list`, `slack_channel_history`, `slack_post_message`, `slack_search`, `slack_users_list`, `slack_open_dm`.

For setup, see the [Connecting integrations](../getting-started.md#slack) section of Getting Started.

After `/connect slack` succeeds, try:

```
list my Slack channels
```

> Multi-workspace support is on the roadmap as ARAWN-I-0034. Today, one Slack app is bound to one workspace.

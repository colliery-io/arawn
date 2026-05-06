# Atlassian — Jira and Confluence

One Atlassian Cloud OAuth app covers both Jira and Confluence. Eleven tools land when configured: 5 Jira tools (`jira_search_issues`, `jira_get_issue`, `jira_create_issue`, `jira_update_issue`, `jira_add_comment`), 5 Confluence tools (`confluence_search`, `confluence_get_page`, `confluence_create_page`, `confluence_update_page`, `confluence_list_spaces`), plus `atlassian_list_resources` for cloud-instance discovery.

For setup, see the [Connecting integrations](../getting-started.md#atlassian--jira-and-confluence) section of Getting Started.

After `/connect atlassian` succeeds, try:

```
show me my open Jira issues
```

## Notes

- **Cloud ID auto-discovery.** Atlassian's API requires a `cloud_id` per request. Arawn calls `/oauth/token/accessible-resources` after consent and caches the first cloud_id you've granted access to. If you have multiple Atlassian sites, the consent screen lets you pick.
- **Classic vs granular scopes.** Arawn uses Atlassian's classic scopes (`read:jira-work`, `write:jira-work`, etc.). If a scope above doesn't appear in the developer console's picker, look in the "Classic scopes" section.
- **Permission prompts on writes.** `jira_create_issue` / `jira_update_issue` / `jira_add_comment` / `confluence_create_page` / `confluence_update_page` ask for permission in the default permission mode.

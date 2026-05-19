# Arawn Docs Completeness Audit

*Phase A audit deliverable for ARAWN-I-0051. Returned inline by the Completeness agent on 2026-05-18; `/tmp` write was denied so this is the canonical copy.*

## Executive coverage table

| # | Category | Covered | Partial | Missing | Total |
|---|---|---:|---:|---:|---:|
| 1 | Slash commands | 4 | 5 | 16 | 25 |
| 2 | CLI flags & subcommands | 5 | 5 | 9 | 19 |
| 3 | Agent tools (engine + workflow + integrations) | 7 | 11 | 56 | 74 |
| 4 | Config keys (arawn.toml) | 9 | 8 | 20 | 37 |
| 5 | Environment variables | 4 | 0 | 14 | 18 |
| 6 | Integration providers + tools | 5 | 1 | 0 | 6 |
| 7 | Feed templates | 12 | 0 | 5 | 17 |
| 8 | Workstream features | 4 | 7 | 7 | 18 |
| 9 | Ceremonies / todos | 0 | 0 | 28 | 28 |
| 10 | Plugins / MCP / skills / sub-agents | 0 | 2 | 14 | 16 |
| 11 | Permission model | 9 | 1 | 4 | 14 |
| 12 | Data directory layout | 7 | 1 | 12 | 20 |
| | **TOTAL** | **66** | **41** | **185** | **292** |

---

## 1. Slash commands

Source: `crates/arawn-tui/src/command.rs` `register_builtins()` (line 72) + `execute_command` dispatcher (line 605).

Coverage: **4 covered / 5 partial / 16 missing** (25 items).

| Item | Status | Doc citation | Note |
|---|---|---|---|
| `/help` | MISSING | — | command.rs:74 |
| `/clear` | MISSING | — | command.rs:79 |
| `/plan` | partial | docs/src/security.md (plan mode table) | Mode mentioned; slash command not |
| `/accept on\|off\|edits` | MISSING | — | command.rs:90; sets permission mode at runtime |
| `/workstream create\|list\|switch` | partial | docs/src/getting-started.md (one example) | No full subcommand reference |
| `/session new\|list` | MISSING | — | command.rs:101 |
| `/promote <name>` | MISSING | — | command.rs:106 |
| `/tools` | MISSING | — | command.rs:112 |
| `/skills` | MISSING | — | command.rs:117 |
| `/plugins` | MISSING | — | command.rs:122 |
| `/agents` | MISSING | — | command.rs:127 |
| `/mcp` | MISSING | — | command.rs:132 |
| `/workflows list\|status` | partial | docs/src/workflows.md | Tools documented; slash command isn't |
| `/permissions` | partial | docs/src/security.md ("on backlog") | Doc calls it TBD; exists in code |
| `/integrations` | covered | docs/src/getting-started.md | Used in walkthroughs |
| `/connect <service>` | covered | docs/src/getting-started.md | Documented |
| `/disconnect <service>` | covered | docs/src/getting-started.md (errors table) | Cited |
| `/watch <tpl> <id> [k=v]` | covered | docs/src/feeds/index.md, getting-started.md | Documented |
| `/watch list [tpl]` | partial | docs/src/feeds/index.md | Two-step flow needs explanation |
| `/feeds` | partial | docs/src/feeds/index.md | Subcommands not documented |
| `/feeds pause <id>` | MISSING | — | command.rs:570 |
| `/feeds resume <id>` | MISSING | — | command.rs:574 |
| `/feeds rm <id> [--yes]` | partial | docs/src/feeds/index.md ("`/unwatch`") | Doc names it `/unwatch`; actual command is `/feeds rm` |
| `/feeds run <id>` | MISSING | — | command.rs:588 |
| `/remember <fact>` | partial | docs/src/memory.md ("Direct access (work-in-progress)") | Doc says stub; actually wired |
| `/memory` | partial | docs/src/memory.md | Same |
| `/forget <entity>` | partial | docs/src/memory.md | Same |
| `/today` | MISSING | — | command.rs:194 |
| `/week` | MISSING | — | command.rs:199 |
| `/retro` | MISSING | — | command.rs:204 |
| `/todo` | MISSING | — | command.rs:209 |

---

## 2. CLI flags & subcommands

Source: `crates/arawn/src/main.rs` `Cli`/`Command` structs (lines 43–111) + `plugin_cmd::run_plugin_command` (11).

Coverage: **5 covered / 5 partial / 9 missing** (19 items).

| Item | Status | Doc citation | Note |
|---|---|---|---|
| `arawn <prompt>` | covered | docs/src/getting-started.md | Cited |
| `arawn serve` | covered | docs/src/getting-started.md | Cited |
| `arawn serve --port <port>` | partial | docs/src/getting-started.md | Port only via TUI `--url` workaround |
| `arawn tui` | covered | docs/src/getting-started.md | Cited |
| `arawn tui --url <ws-url>` | covered | docs/src/getting-started.md | Cited |
| `--data-dir <path>` | partial | docs/src/getting-started.md | Env form covered; flag form not |
| `--session <uuid>` | partial | docs/src/getting-started.md ("Next steps") | One-line |
| `--list-sessions` | partial | docs/src/getting-started.md ("Next steps") | One-line |
| `arawn doctor [--json]` | MISSING | — | main.rs:91 |
| `arawn usage [--period] [--model] [--by-site] [--json]` | MISSING | — | main.rs:97 |
| `arawn plugin install <name@marketplace> [--scope]` | MISSING | — | plugin_cmd.rs:30 |
| `arawn plugin uninstall <name@marketplace>` | MISSING | — | plugin_cmd.rs:48 |
| `arawn plugin enable <name@marketplace>` | MISSING | — | plugin_cmd.rs:62 |
| `arawn plugin disable <name@marketplace>` | MISSING | — | plugin_cmd.rs:73 |
| `arawn plugin list` | MISSING | — | plugin_cmd.rs:83 |
| `arawn plugin marketplace add <org/repo\|url\|path>` | MISSING | — | plugin_cmd.rs:125 |
| `arawn plugin marketplace list` | MISSING | — | plugin_cmd.rs:147 |
| `--scope user\|project` | MISSING | — | plugin_cmd.rs:170 |
| `prompt` trailing positional | covered | docs/src/getting-started.md | Cited |

---

## 3. Agent tools

Sources: `crates/arawn-engine/src/tools/mod.rs`, `crates/arawn-workflow/src/tools.rs`, `crates/arawn-integrations/src/<svc>/`, plus `register_default_tools` in `crates/arawn/src/main.rs:2121`.

Coverage: **7 covered / 11 partial / 56 missing** (74 items).

### Core file/shell tools

| Tool | Status | Note |
|---|---|---|
| `shell` | partial | docs/src/security.md (sandbox); params/usage not |
| `file_read` | MISSING | tools/file_read.rs |
| `file_write` | MISSING | tools/file_write.rs |
| `file_edit` | MISSING | tools/file_edit.rs |
| `glob` | MISSING | tools/glob.rs |
| `grep` | MISSING | tools/grep.rs |
| `web_fetch` | MISSING | tools/web_fetch.rs |
| `web_search` | MISSING | tools/web_search.rs |
| `think` | MISSING | tools/think.rs |
| `ask_user` | MISSING | tools/ask_user.rs |
| `sleep` | MISSING | tools/sleep.rs |

### Sub-agent + background tasks

| Tool | Status | Note |
|---|---|---|
| `agent` | MISSING | tools/agent.rs |
| `task_create` / `task_update` / `task_list` / `task_get` | MISSING | tools/task_list.rs |
| `task_output` | MISSING | tools/task_output.rs |
| `task_stop` | MISSING | tools/task_stop.rs |

### Plan mode

| Tool | Status | Note |
|---|---|---|
| `enter_plan_mode` / `exit_plan_mode` | partial | docs/src/security.md (mode only) |

### Memory / signal / feed_search

| Tool | Status | Note |
|---|---|---|
| `memory_store` / `memory_search` | covered | docs/src/memory.md |
| `signal_search` / `signal_query` / `signal_timeline` | covered | docs/src/palaces/agent-read-patterns.md |
| `feed_search` | covered | docs/src/feeds/feed-search.md |

### Workstream lifecycle (10 tools)

| Tool | Status | Note |
|---|---|---|
| `workstream_new` | partial | docs/src/getting-started.md (slash example) |
| `workstream_list` | MISSING | tools/workstream.rs |
| `workstream_switch` | partial | docs/src/getting-started.md |
| `workstream_show` | MISSING | tools/workstream.rs |
| `workstream_describe` | MISSING | tools/workstream.rs |
| `workstream_bind` | partial | docs/src/getting-started.md, integrations/github.md |
| `workstream_unbind` | MISSING | tools/workstream.rs |
| `workstream_promote` | MISSING | tools/workstream.rs |
| `workstream_delete` | MISSING | tools/workstream.rs |
| `workstream_propose_ontology` | partial | getting-started.md (implicit via skill) |

### Steward (6 tools)

| Tool | Status | Note |
|---|---|---|
| `workstream_journal` / `workstream_refine` / `workstream_apply` / `workstream_rollback` / `workstream_dust` | covered | docs/src/palaces/steward.md, agent-read-patterns.md |
| `workstream_tag` | partial | agent-read-patterns.md (named, subcommands not) |

### Ceremonies / daily / weekly / retro / todo (28 tools)

All **MISSING** — see Section 9.

### Workflow management (4 tools)

| Tool | Status | Note |
|---|---|---|
| `workflow_create` / `workflow_list` / `workflow_status` / `workflow_delete` | covered | docs/src/workflows.md |

### Skill invocation

| Tool | Status | Note |
|---|---|---|
| `skill` (named-skill invoke) | MISSING | tools/skill.rs |

### Integration tools (34 tools)

| Tool | Status | Note |
|---|---|---|
| 5 × gmail_* | partial | integrations/gmail.md (named only) |
| 7 × drive_* | partial | integrations/drive.md (named only) |
| 3 × calendar_* | partial | integrations/calendar.md (named only) |
| `slack_list_channels` (code) ↔ `slack_channels_list` (doc) | partial | **Name drift** |
| `slack_history` (code) ↔ `slack_channel_history` (doc) | partial | **Name drift** |
| `slack_post` (code) ↔ `slack_post_message` (doc) | partial | **Name drift** |
| `slack_react` | MISSING | slack/tools.rs |
| `slack_users_list` / `slack_open_dm` | partial | integrations/slack.md |
| `jira_search` (code) ↔ `jira_search_issues` (doc) | partial | **Name drift** |
| 4 × other jira_* | partial | integrations/atlassian.md |
| `jira_transition_issue` | MISSING | atlassian/jira.rs |
| 5 × confluence_* | partial | integrations/atlassian.md |
| `atlassian_list_resources` | MISSING | Doc mentions it; no implementation found |

---

## 4. Config keys (arawn.toml)

Source: `crates/arawn/src/config.rs`; MCP config in `crates/arawn-mcp/src/config.rs`; permissions in `crates/arawn-engine/src/permissions/config.rs`.

Coverage: **9 covered / 8 partial / 20 missing** (37 keys).

| Key | Status | Note |
|---|---|---|
| `[llm.<name>]` table | covered | docs/src/getting-started.md |
| `[llm.*].provider` / `.model` / `.api_key` / `.api_key_env` / `.context_window` / `.max_tokens` | covered | getting-started.md |
| `[llm.*].base_url` | MISSING | config.rs:21 |
| `[llm.*].tool_use` | MISSING | config.rs:28 |
| `[llm.*].vision` | MISSING | config.rs:32 |
| `[engine].llm` / `.max_iterations` | covered | getting-started.md |
| `[engine].max_result_size` | MISSING | config.rs:85 |
| `[engine].tool_timeout_secs` | MISSING | config.rs:91 |
| `[compactor].llm` / `.compaction_threshold` / `.keep_recent` | partial | mentioned in `generate_default_toml` comments |
| `[extraction].llm` | MISSING | config.rs:151 |
| `[server].host` / `.port` | partial | getting-started.md |
| `[storage].data_dir` | covered | getting-started.md |
| `[prompts].token_budget` | MISSING | config.rs:202 |
| `[sandbox].network_tools` | partial | docs/src/security.md (mentioned but list not enumerated) |
| `[integrations.slack].client_id/secret` | partial | getting-started.md |
| `[integrations.google].client_id/secret` | covered | getting-started.md |
| `[integrations.gmail/calendar/drive].client_id/secret` | partial | getting-started.md (fallback) |
| `[integrations.atlassian].client_id/secret` | partial | getting-started.md |
| `[integrations.github].app_id/app_slug/private_key_path` | covered | integrations/github.md |
| `[routing.hints].lightweight/medium/heavy` | MISSING | config.rs:429 |
| `[routing.providers].local/remote` | MISSING | config.rs:413 |
| `[ceremonies.<kind>].enabled/schedule/timezone/model` | MISSING | config.rs:368–389 |
| `[permissions]` (allow/deny/ask + permission_mode) | covered | docs/src/security.md |
| `[[mcp.servers]]` | MISSING | arawn-mcp/src/config.rs |

---

## 5. Environment variables

Source: grep `env::var` across `crates/`.

Coverage: **4 covered / 0 partial / 14 missing** (18 vars).

| Var | Status | Note |
|---|---|---|
| `ARAWN_DATA_DIR` | covered | getting-started.md |
| `GROQ_API_KEY` | covered | getting-started.md |
| `GROQ_MODEL` | partial | mentioned only in `generate_default_toml` comment |
| `ANTHROPIC_API_KEY` | partial | sample TOML only |
| `OPENAI_API_KEY` | MISSING | arawn-llm/src/openai_compat.rs:75 |
| `OLLAMA_API_KEY` | covered | getting-started.md |
| `ARAWN_GMAIL_CLIENT_ID` / `_SECRET` | MISSING | main.rs:1062–1063 |
| `ARAWN_GOOGLE_CLIENT_ID` / `_SECRET` | MISSING | main.rs:1069–1070 (fallback) |
| `ARAWN_GCAL_CLIENT_ID` / `_SECRET` | MISSING | main.rs:1113–1114 |
| `ARAWN_GDRIVE_CLIENT_ID` / `_SECRET` | MISSING | main.rs:1161–1162 |
| `ARAWN_SLACK_CLIENT_ID` / `_SECRET` | MISSING | main.rs:1364–1365 |
| `ARAWN_ATLASSIAN_CLIENT_ID` / `_SECRET` | MISSING | main.rs:1222–1223 |
| `ARAWN_GITHUB_APP_ID` / `_APP_SLUG` / `_PRIVATE_KEY_PATH` | covered | integrations/github.md |
| `ARAWN_GITHUB_PRIVATE_KEY_PEM` | partial | github.md (one-line comment) |
| `ARAWN_TOOL_TIMEOUT_SECS` | MISSING | arawn-engine/src/tool_timeout.rs:27 |
| `RUST_LOG` | partial | security.md (debug logging) |
| `SHELL` | MISSING | main.rs:2258 (prompt context) |
| `TMPDIR` | MISSING | tools/shell.rs:296 (shell sandbox) |

---

## 6. Integration providers + tools

Source: `crates/arawn-integrations/src/{atlassian,calendar,drive,github,gmail,slack}/`. Feed templates in `crates/arawn-feeds/src/templates/<provider>/`.

Coverage: **5 covered / 1 partial / 0 missing** (6 providers). Per-tool coverage handled in Section 3.

| Provider | Tools | OAuth | Feed templates | Status |
|---|---|---|---|---|
| Gmail | 5 | OAuth 2.0 | inbox-archive, label-archive, sender-filter | covered (gmail.md) |
| Google Calendar | 3 | OAuth 2.0 | upcoming-archive | covered (calendar.md) |
| Google Drive | 7 | OAuth 2.0 | folder-sync, recent | covered (drive.md) |
| Slack | 6 | OAuth 2.0 bot+user dual-token | channel-archive, dm-archive, my-mentions | covered (slack.md) |
| Atlassian | 11 (6 Jira + 5 Confluence) | OAuth 2.0 3LO | project-tracker, assignee-tracker, space-archive | covered (atlassian.md) |
| GitHub | (read-only via API client; no agent tools yet) | GitHub App, not OAuth | notifications, issues-and-prs, review-queue, repo-mirror | partial — github.md says feeds "land in follow-up tasks" but actually shipped |

---

## 7. Feed templates

Source: `crates/arawn-feeds/src/templates/mod.rs::default_registry()`.

Coverage: **12 covered / 0 partial / 5 missing** (17 templates).

| Template | Status | Note |
|---|---|---|
| 12 templates: slack/* (3), gmail/* (3), calendar/upcoming-archive, drive/* (2), confluence/space-archive, jira/* (2) | covered | feeds/template-catalog.md |
| `github/notifications` | MISSING | feeds/templates/github/notifications.rs |
| `github/issues-and-prs` | MISSING | feeds/templates/github/issues_and_prs.rs |
| `github/review-queue` | MISSING | feeds/templates/github/review_queue.rs |
| `github/repo-mirror` | MISSING | feeds/templates/github/repo_mirror.rs |
| `stub/echo` | MISSING | feeds/templates/stub.rs (test fixture; arguably intentional) |

Note: docs/src/feeds/template-catalog.md opens with **"Twelve templates ship today"** but the registry registers **17** (16 real + 1 stub).

---

## 8. Workstream features

Source: `crates/arawn-core/src/workstream.rs` (model) + `crates/arawn-storage/src/workstream_store.rs` (operations).

Coverage: **4 covered / 7 partial / 7 missing** (18 items).

### Operations

| Op | Status | Note |
|---|---|---|
| `create` (with required ontology) | partial | getting-started.md (one example) |
| `find_by_name` / `get` (by UUID) | MISSING | workstream_store.rs:89,99 |
| `list` (active) | partial | implied via `workstream_list` tool |
| `list_all` (incl. archived) | MISSING | workstream_store.rs:115 |
| `update_description` | partial | getting-started.md (`--description` flag) |
| `update_identity_profile` | MISSING | workstream_store.rs:137 |
| `add_binding` | partial | palaces/index.md (mentioned generally) |
| `remove_binding` / `set_bindings` | MISSING | workstream_store.rs:167,193 |
| `soft_delete` (archive) | MISSING | workstream_store.rs:204 |
| `ensure_scratch` (idempotent) | partial | getting-started.md (scratch not explained) |
| Promote scratch → workstream | covered | getting-started.md, command.rs:106 |

### Metadata fields (`Workstream` struct)

| Field | Status | Note |
|---|---|---|
| `id` (Uuid) | covered | palaces/index.md |
| `name` (slug, validated: lowercase, ≤64 chars, `[a-z0-9_-]+`, must start letter/digit) | partial | getting-started.md (validation rules not documented) |
| `display_name` | MISSING | workstream.rs:124 |
| `description` | covered | getting-started.md, palaces/index.md |
| `root_dir` | covered | palaces/index.md |
| `bindings` | covered | palaces/index.md |
| `archived` | MISSING | workstream.rs:134 |
| `identity_profile` (Assistant / Coding) | MISSING | workstream.rs:137 (persona selector) |
| `tags_ontology` (closed list) | covered | palaces/index.md, extraction.md |

---

## 9. Ceremonies / todos

Source: `crates/arawn-ceremonies/src/{plugins,engine,runner,events,...}.rs`, `crates/arawn-engine/src/tools/{ceremony,daily,weekly,todo}.rs`.

Coverage: **0 covered / 0 partial / 28 missing** — **entire subsystem absent from `docs/src/`.**

### Daily ceremony (`0 7 * * MON-FRI` default)

`daily_run`, `daily_current`, `daily_list_items`, `daily_patch_item`, `daily_add_todo` — all MISSING.

### Weekly ceremony (`0 7 * * MON` default)

`weekly_run`, `weekly_current`, `weekly_list_items`, `weekly_list_priorities`, `weekly_confirm_priority`, `weekly_reject_priority`, `weekly_add_priority` — all MISSING.

### Retro ceremony (Friday 16:00 local default)

`retro_run`, `retro_current`, `retro_list_items`, `retro_save_diary`, `retro_patch_item` — all MISSING. Plus retro detectors (`priority-completion`, `rollover-heat`, `workstream-neglect`) — MISSING.

### Generic todos (I-0049)

`todo_create`, `todo_list`, `todo_get`, `todo_done`, `todo_undo`, `todo_patch`, `todo_archive`, `todo_search` — all MISSING.

### Persistence + UI

| Item | Status | Note |
|---|---|---|
| `/today` / `/week` / `/retro` slash → tablet modal | MISSING | command.rs:194-204 |
| `/todo` slash → modal | MISSING | command.rs:209 |
| Ceremony rollup table | MISSING | arawn-ceremonies/src/rollup.rs |
| Ceremony events table | MISSING | arawn-ceremonies/src/events.rs |
| Ceremony nightly recovery loop | MISSING | arawn-ceremonies/src/nightly.rs |
| Gather sources (calendar, attention, signals) | MISSING | plugins/gather_sources.rs |

---

## 10. Plugins / MCP / skills / sub-agents

Sources: `crates/arawn-engine/src/plugins/`, `crates/arawn/src/plugin_cmd.rs`, `crates/arawn-mcp/`, `crates/arawn-engine/src/skills/`, `crates/arawn-engine/src/tools/{agent,task_list}.rs`.

Coverage: **0 covered / 2 partial / 14 missing** (16 items).

### Plugins

| Item | Status | Note |
|---|---|---|
| Plugin manifest format (`plugin.json`) | MISSING | plugins/manifest.rs |
| Plugin user_config / env substitution | MISSING | plugins/settings.rs |
| Plugin hot-reload watcher | MISSING | plugins/runtime.rs |
| Plugin marketplace concept (3 source types: GitHub, git URL, local dir) | MISSING | plugins/marketplace.rs |
| Plugin install / enable / disable scopes (user/project) | MISSING | plugin_cmd.rs |
| Plugin components (tools, agents, skills, hooks, MCP) | MISSING | plugins/components.rs |
| Plugin `installed_plugins.json` registry | MISSING | plugins/installer.rs |
| Built-in plugins | MISSING | plugins/builtin.rs |
| Plugin source: directory / git / github | partial | docs/src/intro.md (one bullet) |

### MCP

| Item | Status | Note |
|---|---|---|
| `[[mcp.servers]]` config section | MISSING | arawn-mcp/src/config.rs |
| MCP server connection (stdio) | MISSING | arawn-mcp/src/manager.rs |
| MCP tools exposed to agent | MISSING | arawn-mcp/src/adapter.rs |
| Plugin-declared MCP servers | MISSING | plugins/manifest.rs::McpServerDef |

### Skills

| Item | Status | Note |
|---|---|---|
| Skill markdown + YAML frontmatter format | MISSING | skills/definition.rs |
| Built-in skills (`workflows`, `workstream-create`) | MISSING | skills/builtin/*.md |
| `user_invocable` skills (`/skill-name`) | MISSING | skills/loader.rs |
| `SkillTool` agent invocation | MISSING | tools/skill.rs |

### Sub-agents

| Item | Status | Note |
|---|---|---|
| `agent` tool (subagent_type, llm override) | MISSING | tools/agent.rs |
| Built-in agent types: `general-purpose`, `Explore`, `Plan` | MISSING | agent_defs.rs |
| Agent loading from `<data_dir>/agents/` (markdown w/ frontmatter) | MISSING | agent_defs.rs::load_agents_dir |
| 3-level nesting cap | MISSING | tools/agent.rs:20 |
| `task_*` background-task tools | MISSING | tools/task_list.rs, task_output.rs, task_stop.rs |

---

## 11. Permission model

Source: `crates/arawn-engine/src/permissions/`.

Coverage: **9 covered / 1 partial / 4 missing** (14 items).

| Item | Status | Note |
|---|---|---|
| Rule kinds: allow / deny / ask | covered | security.md |
| Tool pattern (exact + glob) | covered | security.md |
| Content pattern syntax `Tool(pattern)` | covered | security.md |
| Evaluation order (deny > allow > ask > mode fallback) | covered | security.md |
| Mode: `default` (read-only auto, write/shell ask) | covered | security.md |
| Mode: `accept_edits` (writes auto, shell ask) | covered | security.md |
| Mode: `bypass` (full auto) | covered | security.md |
| Mode: `plan` (side effects denied; only enter/exit_plan_mode exempt) | covered | security.md |
| `PermissionCategory`: ReadOnly / FileWrite / Shell / Other | partial | security.md (table implies, doesn't name) |
| `PermissionMode` set via `/accept` and `/plan` slash | MISSING | command.rs:84,90 |
| `PermissionResponse`: AllowOnce / AllowAlways / Deny | MISSING | permissions/checker.rs:65 |
| Session grants (cached allow-always) | MISSING | permissions/checker.rs::SessionGrants |
| Audit log + `/permissions` slash | MISSING | permissions/checker.rs::AuditEntry |
| `load_merged_permissions` / `load_permissions_from_file` | covered | security.md (implicit via `[permissions]`) |

---

## 12. Data directory layout

Source: `crates/arawn-storage/src/layout.rs::DataLayout::v1` + greps for `.join(` across crates.

Coverage: **7 covered / 1 partial / 12 missing** (20 paths).

| Path | Status | Note |
|---|---|---|
| `<data_dir>/arawn.toml` | covered | getting-started.md |
| `<data_dir>/arawn.db` | MISSING | storage/store.rs:32 |
| `<data_dir>/memory.db` (global KB) | covered | memory.md |
| `<data_dir>/memory.graph.db` | covered | memory.md |
| `<data_dir>/workstreams/<name>/memory.db` | covered | memory.md, palaces/index.md |
| `<data_dir>/workstreams/<name>/workspace/` | MISSING | storage/store.rs:77,318 |
| `<data_dir>/workstreams/scratch/` | partial | getting-started.md (implied, structure not detailed) |
| `<data_dir>/data/<provider>/<template>/<feed_id>/` | covered | feeds/index.md, feeds/template-catalog.md |
| feed `meta.json` | covered | feeds/index.md |
| `<data_dir>/projections.db` | covered | feeds/feed-search.md |
| `<data_dir>/plugins/` (install root) | MISSING | main.rs:549 |
| `<data_dir>/plugins/tools/` (V1) | MISSING | storage/layout.rs:21 |
| `<data_dir>/plugins/build/` | MISSING | storage/layout.rs:22 |
| `<data_dir>/plugins/installed_plugins.json` | MISSING | plugin_cmd.rs:85 |
| `<data_dir>/settings.json` (enable/disable + user_config) | MISSING | main.rs:552 |
| `<data_dir>/prompts/` | MISSING | storage/layout.rs:23, config.rs:569 |
| `<data_dir>/agents/` (user agents .md) | MISSING | main.rs:2149 |
| `<data_dir>/workflows.db` | covered | workflows.md |
| `<data_dir>/workflows/<name>/` (compiled dylibs) | covered | workflows.md |
| `<data_dir>/logs/server.log`, `tui.log` (daily rolling) | MISSING | main.rs:220,241 |
| `<data_dir>/integrations/<svc>/<svc>.bin` (encrypted creds) | MISSING | only github.md mentions this for github |
| `<data_dir>/tokens/` (OAuth tokens) | partial | getting-started.md (one sentence) |
| `<data_dir>/models/all-MiniLM-L6-v2/model.onnx` | covered | memory.md |
| `<data_dir>/feeds.db` (feed registry) | MISSING | arawn-feeds/src/store.rs |

---

## Biggest gaps (summary)

1. **Entire ceremonies + todos subsystem invisible** — 28 agent tools, 4 slash commands, 3 cron schedules, retro detectors framework, multi-table persistence — zero mentions in `docs/src/`.
2. **Plugins / MCP / skills / sub-agents condensed to one bullet in intro.md** — but code is mature (manifest format, marketplaces with 3 source types, hot reload, `arawn plugin` CLI subtree, two built-in skills, three built-in sub-agent types).
3. **38 of 74 agent tools undocumented** — including file/shell/grep/glob/web tools the agent reaches for constantly, plus `agent`/`task_*` sub-agent + background-task management.
4. **CLI subcommands missing:** `arawn doctor`, `arawn usage`, `arawn plugin …` (8 subcommands).
5. **4 tool-name drifts** between Slack/Jira docs and code (`slack_list_channels`, `slack_history`, `slack_post`, `jira_search`).
6. **Stale claim:** feeds/template-catalog.md says "Twelve templates ship today" but registry has 17.
7. **Workstream slug validation rules** (lowercase, ≤64 chars, charset) produce errors not documented anywhere.
8. **14 env vars missing** — most of the per-integration `ARAWN_<SVC>_CLIENT_ID/SECRET` set + `ARAWN_TOOL_TIMEOUT_SECS`.
9. **Strong coverage** of memory, feeds, palaces, permissions, individual integration setup walkthroughs. Rot concentrated in runtime-control surfaces (slash commands, CLI, plugins, ceremonies) and tool reference material.

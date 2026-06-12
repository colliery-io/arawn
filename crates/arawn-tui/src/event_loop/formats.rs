/// Render a `list_integrations` response as a markdown table the user can scan.
pub(super) fn format_integrations_list(items: &[serde_json::Value]) -> String {
    use std::fmt::Write;
    if items.is_empty() {
        return "No integrations registered. (Build with integrations to enable Gmail / Calendar / Slack.)"
            .to_string();
    }
    let mut out = String::from("**Integrations**\n\n| Service | Connected |\n|---|---|\n");
    for item in items {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let connected = item
            .get("connected")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mark = if connected { "✓" } else { "—" };
        let _ = writeln!(out, "| {name} | {mark} |");
    }
    out.push_str(
        "\nRun `/connect <service>` to authorize, `/disconnect <service>` to drop credentials.",
    );
    out
}

/// What `try_open_url` did. The TUI always prints the URL too, so even
/// `NoOpener` is a soft failure — the user can still copy/paste.
pub(super) enum OpenAttempt {
    Opened(&'static str),
    NoOpener,
    Failed(String),
}

/// Best-effort browser open. Returns immediately — doesn't block on the
/// child process (which is the whole point of `open` / `xdg-open`).
pub(super) fn try_open_url(url: &str) -> OpenAttempt {
    let opener: Option<&'static str> = if cfg!(target_os = "macos") {
        Some("open")
    } else if cfg!(target_os = "linux") {
        Some("xdg-open")
    } else if cfg!(target_os = "windows") {
        // `cmd /c start` needs an empty title arg before the URL.
        // We treat it as a special case below.
        Some("cmd")
    } else {
        None
    };
    let Some(cmd) = opener else {
        return OpenAttempt::NoOpener;
    };

    let result = if cmd == "cmd" {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
    } else {
        std::process::Command::new(cmd)
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
    };
    match result {
        Ok(_child) => OpenAttempt::Opened(cmd),
        Err(e) => OpenAttempt::Failed(e.to_string()),
    }
}

/// Render `get_permissions_status` JSON as a human-readable system message.
pub(super) fn format_permissions_status(status: &serde_json::Value) -> String {
    use std::fmt::Write;
    let mut out = String::from("**Permissions**\n\n");

    let mode = status.get("mode").and_then(|v| v.as_str()).unwrap_or("?");
    let _ = writeln!(out, "Mode: `{mode}`");

    let render_list = |label: &str, key: &str, out: &mut String| {
        if let Some(arr) = status.get(key).and_then(|v| v.as_array())
            && !arr.is_empty()
        {
            let _ = writeln!(out, "\n{label}:");
            for item in arr {
                if let Some(s) = item.as_str() {
                    let _ = writeln!(out, "  - `{s}`");
                }
            }
        }
    };
    render_list("Deny rules", "deny_rules", &mut out);
    render_list("Allow rules", "allow_rules", &mut out);
    render_list("Ask rules", "ask_rules", &mut out);

    if let Some(decisions) = status.get("recent_decisions").and_then(|v| v.as_array()) {
        if decisions.is_empty() {
            let _ = writeln!(out, "\nNo decisions recorded yet this session.");
        } else {
            let _ = writeln!(out, "\nRecent decisions (newest first):");
            // Newest at the top — the audit buffer is push_back so the last
            // entry is most recent.
            for entry in decisions.iter().rev().take(20) {
                let ts = entry
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let tool = entry
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let dec = entry
                    .get("decision")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let reason = entry.get("reason").and_then(|v| v.as_str()).unwrap_or("?");
                let _ = writeln!(out, "  {ts}  {tool:<24}  {dec:<8}  {reason}");
            }
        }
    }
    out
}

/// Render a `SystemStatus` (ARAWN-I-0068 P2-1) as a human-readable system
/// message. Thin renderer — all the aggregation already happened server-side;
/// this only formats the contract into rows the user can scan.
pub(super) fn format_system_status(status: &arawn_service::SystemStatus) -> String {
    use std::fmt::Write;
    let mark = |b: bool| if b { "✓" } else { "—" };

    let mut out = String::from("**System status**\n\n");

    // Feeds
    if status.feeds.available {
        let _ = writeln!(
            out,
            "Feeds: ✓ runtime up ({} configured)",
            status.feeds.feeds.len()
        );
        for f in &status.feeds.feeds {
            let last = f.last_run_at.as_deref().unwrap_or("never");
            let st = f.last_status.as_deref().unwrap_or("—");
            let _ = writeln!(
                out,
                "  - `{}` ({}) {} · last run {} · {}",
                f.id,
                f.template,
                if f.enabled { "enabled" } else { "paused" },
                last,
                st,
            );
        }
    } else {
        let _ = writeln!(out, "Feeds: — runtime unavailable");
    }

    // Ceremonies
    match status.ceremonies.pending_notifications {
        Some(n) if status.ceremonies.available => {
            let _ = writeln!(out, "Ceremonies: ✓ engine up · {n} pending notification(s)");
        }
        _ => {
            let _ = writeln!(
                out,
                "Ceremonies: {} engine",
                mark(status.ceremonies.available)
            );
        }
    }

    // Embedding
    let pending = status
        .embedding
        .pending
        .map(|n| n.to_string())
        .unwrap_or_else(|| "n/a".to_string());
    let _ = writeln!(
        out,
        "Embedding: model {} · {pending} pending",
        mark(status.embedding.embedder_loaded),
    );

    // Extraction
    if status.extraction.available {
        let _ = writeln!(
            out,
            "Extraction: ✓ runner up · {} cursor(s)",
            status.extraction.cursors.len()
        );
        for c in &status.extraction.cursors {
            let at = c.cursor_ts.as_deref().unwrap_or("start");
            let _ = writeln!(out, "  - {}/{} → {at}", c.lens, c.feed_type);
        }
    } else {
        let _ = writeln!(out, "Extraction: — runner unavailable");
    }

    // LLM
    if status.llm.clients.is_empty() {
        let _ = writeln!(out, "LLM: — no clients configured");
    } else {
        let _ = writeln!(out, "LLM:");
        for c in &status.llm.clients {
            let _ = writeln!(out, "  - {}: {}/{}", c.role, c.provider, c.model);
        }
    }

    out
}

/// Render a freshly-registered feed into a chat-ready system message.
pub(super) fn format_feed_registered(dto: &serde_json::Value) -> String {
    let template = dto.get("template").and_then(|v| v.as_str()).unwrap_or("?");
    let id = dto.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let cadence = dto.get("cadence").and_then(|v| v.as_str()).unwrap_or("?");
    let dir = dto.get("data_dir").and_then(|v| v.as_str()).unwrap_or("?");
    format!(
        "Registered **{template}** as `{id}`.\n\n\
         - Cadence: `{cadence}`\n\
         - Data dir: `{dir}`\n\n\
         _Will fire on the next cron tick. Run /feeds to see status._"
    )
}

/// Render the `/feeds` listing as a markdown table-ish block. Compact
/// enough to fit in the chat pane without needing a modal — the modal
/// upgrade lands in slice 2 of T-0219.
pub(super) fn format_feed_list(list: &[serde_json::Value]) -> String {
    if list.is_empty() {
        return "No feeds configured. Run /watch to register one.".into();
    }
    let mut s = String::from("**Configured feeds:**\n\n");
    for f in list {
        let id = f.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let template = f.get("template").and_then(|v| v.as_str()).unwrap_or("?");
        let cadence = f.get("cadence").and_then(|v| v.as_str()).unwrap_or("?");
        let enabled = f.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let last_run = f
            .get("last_run_at")
            .and_then(|v| v.as_str())
            .unwrap_or("(never)");
        let last_status = f.get("last_status").and_then(|v| v.as_str()).unwrap_or("-");
        let size = f
            .get("data_size_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let state = if enabled { "active" } else { "paused" };
        s.push_str(&format!(
            "- **{template}** `{id}` — `{cadence}` · {state} · last: {last_run} ({last_status}) · {} on disk\n",
            human_size(size)
        ));
    }
    s
}

pub(super) fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Render `feed_discover` results into a chat-pane block. Empty
/// `picker_supported=false` means the template's params are
/// free-form — nudge the user toward `/watch <tpl> <id> k=v` instead.
pub(super) fn format_feed_discover(dto: &serde_json::Value) -> String {
    let template = dto.get("template").and_then(|v| v.as_str()).unwrap_or("?");
    let supported = dto
        .get("picker_supported")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let rows = dto
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if !supported {
        return format!(
            "**{template}** doesn't support discovery — its params \
             are free-form (sender pattern, label name, folder path, \
             etc.).\n\n\
             Use the typed form, e.g.:\n  \
             /watch {template} <feed_id> <key>=<value>"
        );
    }
    if rows.is_empty() {
        return format!(
            "No discoverable values for **{template}**. The integration \
             may not be connected, or the workspace has none of this kind."
        );
    }
    let mut out = format!("**Pick a value for `{template}`:**\n\n");
    for row in &rows {
        let label = row.get("label").and_then(|v| v.as_str()).unwrap_or("?");
        let hint = row
            .get("hint")
            .and_then(|v| v.as_str())
            .map(|h| format!("  _({h})_"))
            .unwrap_or_default();
        // Find the first key/value pair from params and render it as
        // a copy-pasteable token.
        let params = row.get("params").cloned().unwrap_or_default();
        let kv = params
            .as_object()
            .and_then(|m| m.iter().next())
            .map(|(k, v)| {
                let val = v
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| v.to_string());
                format!("{k}={val}")
            })
            .unwrap_or_else(|| "?".into());
        out.push_str(&format!("- {label}{hint}\n  `{kv}`\n"));
    }
    out.push_str(&format!(
        "\n_To register: `/watch {template} <feed_id> <key>=<value>`._"
    ));
    out
}

/// Static help for `/watch list` with no template — points the user
/// at the canonical list and shows the discovery shortcut.
pub(super) fn format_known_templates() -> String {
    "**Available feed templates:**\n\n\
     - slack/channel-archive · slack/dm-archive · slack/my-mentions\n\
     - calendar/upcoming-archive\n\
     - gmail/inbox-archive · gmail/sender-filter · gmail/label-archive\n\
     - drive/folder-sync · drive/recent\n\
     - confluence/space-archive\n\
     - jira/project-tracker · jira/assignee-tracker\n\n\
     Run `/watch list <template>` to pick a value when the template \
     supports discovery (slack/channel-archive, jira/project-tracker, \
     confluence/space-archive). Use the typed form `/watch <template> \
     <feed_id> key=value` for the rest."
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_service::{
        CeremoniesStatus, EmbeddingStatus, ExtractionCursor, ExtractionStatus, FeedStatusRow,
        FeedsStatus, LlmClientStatus, LlmStatus, SYSTEM_STATUS_VERSION, SystemStatus,
    };

    fn populated() -> SystemStatus {
        SystemStatus {
            version: SYSTEM_STATUS_VERSION,
            feeds: FeedsStatus {
                available: true,
                feeds: vec![FeedStatusRow {
                    id: "design-channel".into(),
                    template: "slack/channel-archive".into(),
                    enabled: true,
                    last_run_at: Some("2026-06-12T10:00:00Z".into()),
                    last_status: Some("ok".into()),
                }],
            },
            ceremonies: CeremoniesStatus {
                available: true,
                pending_notifications: Some(2),
            },
            embedding: EmbeddingStatus {
                embedder_loaded: true,
                pending: Some(5),
            },
            extraction: ExtractionStatus {
                available: true,
                cursors: vec![ExtractionCursor {
                    lens: "pat".into(),
                    feed_type: "slack_messages".into(),
                    cursor_ts: Some("2026-06-11T09:00:00Z".into()),
                }],
            },
            llm: LlmStatus {
                clients: vec![LlmClientStatus {
                    role: "engine".into(),
                    provider: "groq".into(),
                    model: "llama-3.3-70b".into(),
                }],
                engine_reachable: None,
            },
        }
    }

    #[test]
    fn format_system_status_renders_every_subsystem() {
        let out = format_system_status(&populated());
        // One scannable line per subsystem, with the key live numbers.
        assert!(out.contains("Feeds: ✓ runtime up (1 configured)"), "{out}");
        assert!(out.contains("design-channel"), "{out}");
        assert!(out.contains("Ceremonies: ✓ engine up · 2 pending"), "{out}");
        assert!(out.contains("Embedding: model ✓ · 5 pending"), "{out}");
        assert!(out.contains("Extraction: ✓ runner up · 1 cursor"), "{out}");
        assert!(out.contains("pat/slack_messages"), "{out}");
        assert!(out.contains("engine: groq/llama-3.3-70b"), "{out}");
    }

    #[test]
    fn format_system_status_marks_absent_subsystems() {
        let mut s = populated();
        s.feeds.available = false;
        s.feeds.feeds.clear();
        s.ceremonies = CeremoniesStatus {
            available: false,
            pending_notifications: None,
        };
        s.embedding = EmbeddingStatus {
            embedder_loaded: false,
            pending: None,
        };
        s.extraction = ExtractionStatus {
            available: false,
            cursors: vec![],
        };
        let out = format_system_status(&s);
        assert!(out.contains("Feeds: — runtime unavailable"), "{out}");
        assert!(out.contains("Ceremonies: — engine"), "{out}");
        assert!(out.contains("Embedding: model — · n/a pending"), "{out}");
        assert!(out.contains("Extraction: — runner unavailable"), "{out}");
    }
}

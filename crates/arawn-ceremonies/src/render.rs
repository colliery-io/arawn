//! Markdown renderer for ceremony tablets.
//!
//! The TUI's `/today`, `/week`, and `/retro` commands fetch a
//! `TabletDto + Vec<ItemDto>` over WS-RPC and render the result
//! to markdown for display. The renderer lives here (rather than
//! in `arawn-tui`) so the markdown format is the canonical
//! representation — same string the user sees in the terminal, in
//! a future web view, and in an exported file.
//!
//! Stable layout per retro tablet:
//!
//! ```markdown
//! # Retro for {iso_week}
//!
//! _Generated 2026-05-15T16:00:00Z · status: open_
//!
//! ## What happened
//!
//! - {body[0]}  [^cite-{citation_id}]
//! - {body[1]}  [^cite-{citation_id}]
//!
//! ## Patterns
//!
//! - {body[0]}  [^cite-{citation_id}]
//!
//! ## Your reflection
//!
//! {diary body, verbatim}
//!
//! ---
//!
//! [^cite-sig-1]: signal id `sig-1`
//! [^cite-pattern-…]: detected pattern row
//! ```
//!
//! The TUI command on receipt of `ceremonies.get_retro_current`
//! also calls `ceremonies.list_items` to get the items, and
//! `ceremonies.get_diary` (future RPC) for the diary body — until
//! then the diary body is fetched alongside the tablet by reading
//! `ceremony_diary` directly. T-0290 leaves that fetch + the
//! slash-command wiring to the binary integration; the renderer
//! shipped here is the contract.

use crate::service::{ItemDto, PriorityDto, TabletDto};

/// What the renderer needs to draw a retro tablet. The TUI
/// assembles this from the three RPC calls (`get_retro_current`,
/// `list_items`, and the diary row).
#[derive(Debug, Clone)]
pub struct RetroView {
    pub tablet: TabletDto,
    pub items: Vec<ItemDto>,
    /// User-written diary body. `None` when the user has not yet
    /// written one.
    pub diary: Option<String>,
}

/// Render a retro tablet to markdown.
pub fn render_retro(view: &RetroView) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Retro for {}\n\n", view.tablet.period_key));
    out.push_str(&format!(
        "_Generated {} · status: {}_\n\n",
        view.tablet.generated_at, view.tablet.status
    ));

    let what_happened = items_in_section(&view.items, "what_happened");
    let patterns = items_in_section(&view.items, "patterns");

    out.push_str("## What happened\n\n");
    if what_happened.is_empty() {
        out.push_str("_(nothing notable in the gather payload yet — try again after a few daily tablets accumulate)_\n\n");
    } else {
        for item in &what_happened {
            render_item_bullet(&mut out, item);
        }
        out.push('\n');
    }

    out.push_str("## Patterns\n\n");
    if patterns.is_empty() {
        out.push_str(
            "_(insufficient history — detectors require a few prior weeks of rollup data)_\n\n",
        );
    } else {
        for item in &patterns {
            render_item_bullet(&mut out, item);
        }
        out.push('\n');
    }

    out.push_str("## Your reflection\n\n");
    match view.diary.as_deref() {
        Some(diary) if !diary.trim().is_empty() => {
            out.push_str(diary.trim_end());
            out.push_str("\n\n");
        }
        _ => {
            out.push_str(
                "_(write a few sentences about how the week felt; saved to diary on close)_\n\n",
            );
        }
    }

    // Footnote section: collect citation_ids and emit them once. The
    // TUI doesn't resolve the citations into source links yet — for
    // v1 we just print the bare ids so the user can grep them out.
    let mut citations: Vec<String> = view
        .items
        .iter()
        .filter_map(|i| i.citation_id.clone())
        .collect();
    citations.sort();
    citations.dedup();
    if !citations.is_empty() {
        out.push_str("---\n\n");
        for c in &citations {
            out.push_str(&format!("[^cite-{c}]: source row `{c}`\n"));
        }
    }
    out
}

fn items_in_section<'a>(items: &'a [ItemDto], section_key: &str) -> Vec<&'a ItemDto> {
    let mut filtered: Vec<&ItemDto> = items
        .iter()
        .filter(|i| i.section_key == section_key)
        .collect();
    filtered.sort_by_key(|i| i.ordinal);
    filtered
}

/// What the renderer needs to draw a daily tablet. The TUI assembles
/// this from `ceremonies.get_by_period("daily", today)` plus
/// `ceremonies.list_items`. Sections are the canonical four:
/// `calendar`, `todos`, `attention`, `alignment`.
#[derive(Debug, Clone)]
pub struct DailyView {
    pub tablet: TabletDto,
    pub items: Vec<ItemDto>,
}

/// Render a daily tablet to markdown.
pub fn render_daily(view: &DailyView) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Today — {}\n\n", view.tablet.period_key));
    out.push_str(&format!(
        "_Generated {} · status: {}_\n\n",
        view.tablet.generated_at, view.tablet.status
    ));

    let sections: [(&str, &str, &str); 4] = [
        (
            "calendar",
            "## Today's calendar",
            "_(no calendar events for today — the day is yours)_",
        ),
        (
            "todos",
            "## Carried-over todos",
            "_(no todos rolled over from yesterday)_",
        ),
        (
            "attention",
            "## New since yesterday",
            "_(no new signals captured since yesterday's tablet)_",
        ),
        (
            "alignment",
            "## This week's priorities",
            "_(no weekly priorities set — run /week on Monday to plan them)_",
        ),
    ];

    for (key, heading, placeholder) in sections {
        out.push_str(heading);
        out.push_str("\n\n");
        let rows = items_in_section(&view.items, key);
        if rows.is_empty() {
            out.push_str(placeholder);
            out.push_str("\n\n");
        } else {
            for item in &rows {
                render_item_bullet(&mut out, item);
            }
            out.push('\n');
        }
    }

    render_footnotes(
        &mut out,
        view.items.iter().filter_map(|i| i.citation_id.clone()),
    );
    out
}

/// What the renderer needs to draw a weekly tablet. The TUI assembles
/// this from `ceremonies.get_by_period("weekly", iso_week)` plus
/// `ceremonies.list_items` plus `ceremonies.list_priorities`.
#[derive(Debug, Clone)]
pub struct WeeklyView {
    pub tablet: TabletDto,
    pub items: Vec<ItemDto>,
    pub priorities: Vec<PriorityDto>,
}

/// Render a weekly tablet to markdown.
pub fn render_weekly(view: &WeeklyView) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Week of {}\n\n", view.tablet.period_key));
    let priorities_confirmed = view.tablet.priorities_confirmed_at.is_some();
    out.push_str(&format!(
        "_Generated {} · status: {} · priorities confirmed: {}_\n\n",
        view.tablet.generated_at,
        view.tablet.status,
        if priorities_confirmed { "yes" } else { "no" }
    ));

    // Priorities section uses the explicit `priorities` list rather
    // than the items section — confirmed rows and candidates render
    // with different glyphs so the user can see the state at a glance.
    out.push_str("## Priorities\n\n");
    if view.priorities.is_empty() {
        out.push_str(
            "_(no priorities yet — the weekly ceremony proposes candidates Monday morning)_\n\n",
        );
    } else {
        let mut sorted: Vec<&PriorityDto> = view.priorities.iter().collect();
        sorted.sort_by_key(|p| p.ordinal);
        for p in &sorted {
            render_priority_bullet(&mut out, p);
        }
        out.push('\n');
    }

    let rest: [(&str, &str, &str); 4] = [
        (
            "calendar_shape",
            "## Calendar shape",
            "_(no calendar summary available for this week)_",
        ),
        (
            "deadlines",
            "## Deadlines",
            "_(no upcoming deadlines surfaced)_",
        ),
        (
            "from_last_retro",
            "## From last retro",
            "_(no carry-over from last week's retro)_",
        ),
        (
            "inbound",
            "## Inbound",
            "_(no inbound items rolled over from last week)_",
        ),
    ];

    for (key, heading, placeholder) in rest {
        out.push_str(heading);
        out.push_str("\n\n");
        let rows = items_in_section(&view.items, key);
        if rows.is_empty() {
            out.push_str(placeholder);
            out.push_str("\n\n");
        } else {
            for item in &rows {
                render_item_bullet(&mut out, item);
            }
            out.push('\n');
        }
    }

    // Footnotes — pull citations from items AND priorities (both can
    // be backed by source rows the user might want to grep).
    let item_cites = view.items.iter().filter_map(|i| i.citation_id.clone());
    let prio_cites = view.priorities.iter().filter_map(|p| p.citation_id.clone());
    render_footnotes(&mut out, item_cites.chain(prio_cites));
    out
}

fn render_priority_bullet(out: &mut String, p: &PriorityDto) {
    // `confirmed` priorities get a check-mark; candidates get a
    // question mark so the open Monday-morning flow is obvious.
    let glyph = match p.source.as_str() {
        "confirmed" => "[x]",
        _ => "[ ]",
    };
    let body_text = p
        .body
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| serde_json::to_string(&p.body).unwrap_or_default());
    let cite = p
        .citation_id
        .as_ref()
        .map(|c| format!("  [^cite-{c}]"))
        .unwrap_or_default();
    if p.rationale.trim().is_empty() {
        out.push_str(&format!("- {glyph} {body_text}{cite}\n"));
    } else {
        out.push_str(&format!(
            "- {glyph} {body_text} — _{}_{cite}\n",
            p.rationale.trim()
        ));
    }
}

fn render_footnotes<I: Iterator<Item = String>>(out: &mut String, citations: I) {
    let mut cites: Vec<String> = citations.collect();
    cites.sort();
    cites.dedup();
    if !cites.is_empty() {
        out.push_str("---\n\n");
        for c in &cites {
            out.push_str(&format!("[^cite-{c}]: source row `{c}`\n"));
        }
    }
}

fn render_item_bullet(out: &mut String, item: &ItemDto) {
    let body_text = item
        .body
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| serde_json::to_string(&item.body).unwrap_or_default());
    let cite = item
        .citation_id
        .as_ref()
        .map(|c| format!("  [^cite-{c}]"))
        .unwrap_or_default();
    out.push_str(&format!("- {body_text}{cite}\n"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tablet(iso_week: &str, status: &str) -> TabletDto {
        TabletDto {
            id: format!("retro-{iso_week}"),
            kind: "retro".into(),
            period_key: iso_week.into(),
            generated_at: "2026-05-15T16:00:00Z".into(),
            status: status.into(),
            workstreams_scanned: json!([]),
            priorities_confirmed_at: None,
        }
    }

    fn item(section: &str, ordinal: i32, text: &str, citation: Option<&str>) -> ItemDto {
        ItemDto {
            id: format!("item-{section}-{ordinal}"),
            tablet_id: "retro-2026-W20".into(),
            section_key: section.into(),
            ordinal,
            kind: "pattern".into(),
            body: json!({"text": text}),
            citation_id: citation.map(|s| s.to_string()),
            done_at: None,
            created_at: "2026-05-15T16:00:30Z".into(),
        }
    }

    #[test]
    fn full_retro_renders_with_all_three_sections() {
        let view = RetroView {
            tablet: tablet("2026-W20", "open"),
            items: vec![
                item("what_happened", 0, "Shipped the doc.", Some("sig-1")),
                item("what_happened", 1, "Two deep-work blocks.", Some("event-7")),
                item(
                    "patterns",
                    0,
                    "Priority completion below 50%.",
                    Some("pat-9"),
                ),
            ],
            diary: Some("Felt productive but interrupted often.".into()),
        };
        let md = render_retro(&view);
        // Snapshot-style assertion — substantive shape rather than
        // a full insta snapshot (keeps the test self-contained).
        assert!(md.contains("# Retro for 2026-W20"));
        assert!(md.contains("status: open"));
        assert!(md.contains("## What happened"));
        assert!(md.contains("- Shipped the doc.  [^cite-sig-1]"));
        assert!(md.contains("- Two deep-work blocks.  [^cite-event-7]"));
        assert!(md.contains("## Patterns"));
        assert!(md.contains("- Priority completion below 50%.  [^cite-pat-9]"));
        assert!(md.contains("## Your reflection"));
        assert!(md.contains("Felt productive but interrupted often."));
        // Footnotes section deduplicated.
        assert!(md.contains("[^cite-sig-1]: source row `sig-1`"));
        assert!(md.contains("[^cite-event-7]: source row `event-7`"));
        assert!(md.contains("[^cite-pat-9]: source row `pat-9`"));
    }

    #[test]
    fn empty_what_happened_renders_placeholder() {
        let view = RetroView {
            tablet: tablet("2026-W20", "open"),
            items: vec![],
            diary: None,
        };
        let md = render_retro(&view);
        assert!(md.contains("nothing notable in the gather payload yet"));
    }

    #[test]
    fn empty_patterns_renders_bootstrap_message() {
        // No items in 'patterns' section but some in 'what_happened'.
        let view = RetroView {
            tablet: tablet("2026-W20", "open"),
            items: vec![item("what_happened", 0, "something", Some("c-1"))],
            diary: None,
        };
        let md = render_retro(&view);
        assert!(md.contains("## Patterns"));
        assert!(md.contains("insufficient history"));
    }

    #[test]
    fn missing_diary_renders_placeholder() {
        let view = RetroView {
            tablet: tablet("2026-W20", "open"),
            items: vec![],
            diary: None,
        };
        let md = render_retro(&view);
        assert!(md.contains("write a few sentences"));
    }

    #[test]
    fn blank_diary_renders_placeholder() {
        let view = RetroView {
            tablet: tablet("2026-W20", "reviewed"),
            items: vec![],
            diary: Some("   \n\n  ".into()),
        };
        let md = render_retro(&view);
        assert!(md.contains("write a few sentences"));
    }

    #[test]
    fn items_are_sorted_by_ordinal() {
        let view = RetroView {
            tablet: tablet("2026-W20", "open"),
            items: vec![
                item("what_happened", 2, "third", Some("c3")),
                item("what_happened", 0, "first", Some("c1")),
                item("what_happened", 1, "second", Some("c2")),
            ],
            diary: None,
        };
        let md = render_retro(&view);
        let first_idx = md.find("first").unwrap();
        let second_idx = md.find("second").unwrap();
        let third_idx = md.find("third").unwrap();
        assert!(first_idx < second_idx);
        assert!(second_idx < third_idx);
    }

    #[test]
    fn footnotes_deduplicate_repeated_citations() {
        let view = RetroView {
            tablet: tablet("2026-W20", "open"),
            items: vec![
                item("what_happened", 0, "a", Some("sig-1")),
                item("what_happened", 1, "b", Some("sig-1")), // same citation
                item("patterns", 0, "c", Some("sig-1")),
            ],
            diary: None,
        };
        let md = render_retro(&view);
        let count = md.matches("[^cite-sig-1]: source row").count();
        assert_eq!(count, 1, "duplicate citation should collapse in footnotes");
    }

    #[test]
    fn missing_citation_just_omits_marker() {
        let view = RetroView {
            tablet: tablet("2026-W20", "open"),
            items: vec![item("what_happened", 0, "user-added", None)],
            diary: None,
        };
        let md = render_retro(&view);
        assert!(md.contains("- user-added\n"));
        assert!(!md.contains("[^cite-"));
    }

    fn daily_tablet(date: &str, status: &str) -> TabletDto {
        TabletDto {
            id: format!("daily-{date}"),
            kind: "daily".into(),
            period_key: date.into(),
            generated_at: "2026-05-16T07:00:00Z".into(),
            status: status.into(),
            workstreams_scanned: json!([]),
            priorities_confirmed_at: None,
        }
    }

    fn weekly_tablet(iso_week: &str, status: &str, confirmed: Option<&str>) -> TabletDto {
        TabletDto {
            id: format!("weekly-{iso_week}"),
            kind: "weekly".into(),
            period_key: iso_week.into(),
            generated_at: "2026-05-11T08:00:00Z".into(),
            status: status.into(),
            workstreams_scanned: json!([]),
            priorities_confirmed_at: confirmed.map(String::from),
        }
    }

    fn priority(
        ordinal: i32,
        source: &str,
        text: &str,
        rationale: &str,
        citation: Option<&str>,
    ) -> PriorityDto {
        PriorityDto {
            id: format!("prio-{ordinal}"),
            tablet_id: "weekly-2026-W20".into(),
            body: json!({"text": text}),
            rationale: rationale.into(),
            citation_id: citation.map(String::from),
            confirmed_at: if source == "confirmed" {
                Some("2026-05-11T09:00:00Z".into())
            } else {
                None
            },
            done_at: None,
            ordinal,
            source: source.into(),
        }
    }

    #[test]
    fn daily_renders_all_four_sections_in_order() {
        let view = DailyView {
            tablet: daily_tablet("2026-05-16", "open"),
            items: vec![
                item("calendar", 0, "Standup at 14:00.", Some("evt-1")),
                item("todos", 0, "Continue plugin work.", Some("todo-1")),
                item("attention", 0, "Respond to urgent email.", Some("sig-1")),
                item("alignment", 0, "Ties to weekly priority.", Some("prio-1")),
            ],
        };
        let md = render_daily(&view);
        assert!(md.contains("# Today — 2026-05-16"));
        assert!(md.contains("status: open"));
        let cal = md.find("## Today's calendar").unwrap();
        let todos = md.find("## Carried-over todos").unwrap();
        let attn = md.find("## New since yesterday").unwrap();
        let align = md.find("## This week's priorities").unwrap();
        assert!(cal < todos);
        assert!(todos < attn);
        assert!(attn < align);
        assert!(md.contains("- Standup at 14:00.  [^cite-evt-1]"));
        assert!(md.contains("- Continue plugin work.  [^cite-todo-1]"));
        assert!(md.contains("[^cite-evt-1]: source row `evt-1`"));
    }

    #[test]
    fn daily_empty_sections_render_placeholders() {
        let view = DailyView {
            tablet: daily_tablet("2026-05-16", "open"),
            items: vec![],
        };
        let md = render_daily(&view);
        assert!(md.contains("the day is yours"));
        assert!(md.contains("no todos rolled over"));
        assert!(md.contains("no new signals"));
        assert!(md.contains("no weekly priorities set"));
    }

    #[test]
    fn daily_footnotes_deduplicate() {
        let view = DailyView {
            tablet: daily_tablet("2026-05-16", "open"),
            items: vec![
                item("calendar", 0, "a", Some("sig-1")),
                item("todos", 0, "b", Some("sig-1")),
                item("attention", 0, "c", Some("sig-1")),
            ],
        };
        let md = render_daily(&view);
        let count = md.matches("[^cite-sig-1]: source row").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn weekly_renders_priorities_and_five_sections() {
        let view = WeeklyView {
            tablet: weekly_tablet("2026-W20", "open", Some("2026-05-11T09:00:00Z")),
            items: vec![
                item("calendar_shape", 0, "7 meetings.", Some("summary-w20")),
                item("deadlines", 0, "Tax form Friday.", Some("sig-due-1")),
                item(
                    "from_last_retro",
                    0,
                    "Protect mornings.",
                    Some("diary-prev"),
                ),
                item("inbound", 0, "Vendor follow-up.", Some("wk-prev-1")),
            ],
            priorities: vec![
                priority(
                    0,
                    "confirmed",
                    "Ship OAuth refresh.",
                    "high impact",
                    Some("hot-1"),
                ),
                priority(1, "candidate", "Plan Q3 review.", "", None),
            ],
        };
        let md = render_weekly(&view);
        assert!(md.contains("# Week of 2026-W20"));
        assert!(md.contains("priorities confirmed: yes"));
        let prios = md.find("## Priorities").unwrap();
        let cal = md.find("## Calendar shape").unwrap();
        let dl = md.find("## Deadlines").unwrap();
        let flr = md.find("## From last retro").unwrap();
        let inb = md.find("## Inbound").unwrap();
        assert!(prios < cal);
        assert!(cal < dl);
        assert!(dl < flr);
        assert!(flr < inb);
        // Confirmed glyph differs from candidate glyph.
        assert!(md.contains("- [x] Ship OAuth refresh."));
        assert!(md.contains("- [ ] Plan Q3 review."));
        // Rationale renders for confirmed entries.
        assert!(md.contains("_high impact_"));
        // Priority citation participates in footnotes.
        assert!(md.contains("[^cite-hot-1]: source row `hot-1`"));
    }

    #[test]
    fn weekly_unconfirmed_status_shows_no() {
        let view = WeeklyView {
            tablet: weekly_tablet("2026-W20", "open", None),
            items: vec![],
            priorities: vec![],
        };
        let md = render_weekly(&view);
        assert!(md.contains("priorities confirmed: no"));
    }

    #[test]
    fn weekly_empty_sections_render_placeholders() {
        let view = WeeklyView {
            tablet: weekly_tablet("2026-W20", "open", None),
            items: vec![],
            priorities: vec![],
        };
        let md = render_weekly(&view);
        assert!(md.contains("no priorities yet"));
        assert!(md.contains("no calendar summary"));
        assert!(md.contains("no upcoming deadlines"));
        assert!(md.contains("no carry-over from last week"));
        assert!(md.contains("no inbound items"));
    }

    #[test]
    fn weekly_footnotes_dedupe_across_items_and_priorities() {
        let view = WeeklyView {
            tablet: weekly_tablet("2026-W20", "open", None),
            items: vec![item("deadlines", 0, "x", Some("shared-1"))],
            priorities: vec![priority(0, "confirmed", "y", "r", Some("shared-1"))],
        };
        let md = render_weekly(&view);
        let count = md.matches("[^cite-shared-1]: source row").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn body_falls_back_to_raw_json_when_text_missing() {
        let view = RetroView {
            tablet: tablet("2026-W20", "open"),
            items: vec![ItemDto {
                id: "x".into(),
                tablet_id: "retro-2026-W20".into(),
                section_key: "what_happened".into(),
                ordinal: 0,
                kind: "freeform".into(),
                body: json!({"shape": "not text"}),
                citation_id: Some("c-1".into()),
                done_at: None,
                created_at: "2026-05-15T16:00:30Z".into(),
            }],
            diary: None,
        };
        let md = render_retro(&view);
        assert!(md.contains("{\"shape\":\"not text\"}"));
    }
}

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

use crate::service::{ItemDto, TabletDto};

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
        out.push_str("_(insufficient history — detectors require a few prior weeks of rollup data)_\n\n");
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
            out.push_str("_(write a few sentences about how the week felt; saved to diary on close)_\n\n");
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
    let mut filtered: Vec<&ItemDto> = items.iter().filter(|i| i.section_key == section_key).collect();
    filtered.sort_by_key(|i| i.ordinal);
    filtered
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
                item("patterns", 0, "Priority completion below 50%.", Some("pat-9")),
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

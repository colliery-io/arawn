//! Interactive overlay for the `/watch` slash command with no args
//! (ARAWN-I-0058). Two stages:
//!
//! 1. **Pick template** — a selectable list of every registered feed
//!    template with a one-line description.
//! 2. **Fill form** — one field per the template's `param_schema()` (fetched
//!    via the `feed_schema` RPC by the event loop), plus a required `feed_id`
//!    field and an optional, pre-filled cadence field under an "advanced"
//!    separator.
//!
//! Mirrors `todo_modal.rs`'s state-machine pattern: this module is pure UI
//! state + render. It performs no I/O — `handle_key` returns a
//! [`WatchOutcome`] the event loop routes to the `feed_schema` /
//! `feed_register` RPCs.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use serde_json::{Value, json};

use arawn_service::{FeedParamKindDto, FeedParamSpecDto};

use crate::theme;

/// Sentinel keys for the two synthetic fields that aren't template params.
const FEED_ID_KEY: &str = "__feed_id__";
const CADENCE_KEY: &str = "__cadence__";

/// One template the user can pick in stage 1.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateChoice {
    pub name: String,
    pub description: String,
}

/// Which stage of the flow the modal is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchStage {
    PickTemplate,
    FillForm,
}

/// A single editable form row. Holds the spec it renders from and the current
/// raw edit buffer (for `Bool` the buffer is `"true"`/`"false"`; for `List`
/// it's the space-separated source text).
#[derive(Debug, Clone)]
pub struct FieldState {
    pub spec: FeedParamSpecDto,
    pub value: String,
}

impl FieldState {
    fn from_spec(spec: FeedParamSpecDto) -> Self {
        let value = match &spec.default {
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect::<Vec<_>>()
                .join(" "),
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };
        Self { spec, value }
    }

    fn synthetic(key: &str, label: &str, required: bool, value: String, help: &str) -> Self {
        Self {
            spec: FeedParamSpecDto {
                key: key.to_string(),
                label: label.to_string(),
                kind: FeedParamKindDto::Text,
                required,
                default: None,
                help: help.to_string(),
            },
            value,
        }
    }

    fn is_bool(&self) -> bool {
        matches!(self.spec.kind, FeedParamKindDto::Bool)
    }

    fn enum_values(&self) -> Option<&[String]> {
        match &self.spec.kind {
            FeedParamKindDto::Enum(v) => Some(v),
            _ => None,
        }
    }
}

/// State for the `/watch` registration modal.
pub struct WatchModalState {
    pub stage: WatchStage,
    // --- stage 1 ---
    pub templates: Vec<TemplateChoice>,
    pub template_index: usize,
    // --- stage 2 ---
    /// The chosen template name (set on entering the form).
    pub template: String,
    /// Default cadence reported by `feed_schema`, shown as a hint.
    pub default_cadence: String,
    /// Ordered focusable rows: `feed_id`, then each param, then `cadence`.
    pub fields: Vec<FieldState>,
    pub focus: usize,
    pub last_error: Option<String>,
}

/// What the event loop should do after a key press.
#[derive(Debug, Clone, PartialEq)]
pub enum WatchOutcome {
    None,
    /// Stage 1 selection — the event loop fetches this template's schema via
    /// `feed_schema` and calls [`WatchModalState::enter_form`].
    TemplatePicked(String),
    /// A valid form submission — send to `feed_register`.
    Submit {
        template: String,
        feed_id: String,
        params: Value,
        cadence: Option<String>,
    },
    /// Close the overlay with no side effects.
    Cancel,
}

impl WatchModalState {
    /// Open at stage 1 with the given template list.
    pub fn new(mut templates: Vec<TemplateChoice>) -> Self {
        templates.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            stage: WatchStage::PickTemplate,
            templates,
            template_index: 0,
            template: String::new(),
            default_cadence: String::new(),
            fields: Vec::new(),
            focus: 0,
            last_error: None,
        }
    }

    /// Transition into the form once the schema has been fetched. Builds the
    /// focusable field list: `feed_id` first, then one field per param (in
    /// schema order), then the cadence field last.
    pub fn enter_form(
        &mut self,
        template: &str,
        params: Vec<FeedParamSpecDto>,
        default_cadence: &str,
    ) {
        let mut fields = Vec::with_capacity(params.len() + 2);
        fields.push(FieldState::synthetic(
            FEED_ID_KEY,
            "Feed ID",
            true,
            String::new(),
            "A short name you choose for this feed (shown in /feeds).",
        ));
        fields.extend(params.into_iter().map(FieldState::from_spec));
        fields.push(FieldState::synthetic(
            CADENCE_KEY,
            "Cadence (advanced)",
            false,
            default_cadence.to_string(),
            "Cron schedule. Leave as-is for the template default; ≥15-minute floor.",
        ));

        self.template = template.to_string();
        self.default_cadence = default_cadence.to_string();
        self.fields = fields;
        self.focus = 0;
        self.last_error = None;
        self.stage = WatchStage::FillForm;
    }

    fn focused_field(&mut self) -> Option<&mut FieldState> {
        self.fields.get_mut(self.focus)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> WatchOutcome {
        match self.stage {
            WatchStage::PickTemplate => self.handle_pick_key(key),
            WatchStage::FillForm => self.handle_form_key(key),
        }
    }

    fn handle_pick_key(&mut self, key: KeyEvent) -> WatchOutcome {
        match key.code {
            KeyCode::Esc => WatchOutcome::Cancel,
            KeyCode::Up => {
                self.template_index = self.template_index.saturating_sub(1);
                WatchOutcome::None
            }
            KeyCode::Down => {
                if self.template_index + 1 < self.templates.len() {
                    self.template_index += 1;
                }
                WatchOutcome::None
            }
            KeyCode::Enter => match self.templates.get(self.template_index) {
                Some(t) => WatchOutcome::TemplatePicked(t.name.clone()),
                None => WatchOutcome::None,
            },
            _ => WatchOutcome::None,
        }
    }

    fn handle_form_key(&mut self, key: KeyEvent) -> WatchOutcome {
        // Ctrl-C / Esc always cancel.
        if key.code == KeyCode::Esc {
            return WatchOutcome::Cancel;
        }
        self.last_error = None;
        match key.code {
            KeyCode::Tab | KeyCode::Down => {
                if self.focus + 1 < self.fields.len() {
                    self.focus += 1;
                }
                WatchOutcome::None
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.focus = self.focus.saturating_sub(1);
                WatchOutcome::None
            }
            KeyCode::Enter => self.build_submit(),
            KeyCode::Left | KeyCode::Right => {
                self.cycle_focused(key.code == KeyCode::Right);
                WatchOutcome::None
            }
            KeyCode::Char(' ') => {
                // Space toggles a bool/enum; otherwise it's literal input
                // (paths, list globs, etc.).
                if let Some(f) = self.focused_field() {
                    if f.is_bool() || f.enum_values().is_some() {
                        self.cycle_focused(true);
                    } else {
                        f.value.push(' ');
                    }
                }
                WatchOutcome::None
            }
            KeyCode::Backspace => {
                if let Some(f) = self.focused_field()
                    && !f.is_bool()
                    && f.enum_values().is_none()
                {
                    f.value.pop();
                }
                WatchOutcome::None
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(f) = self.focused_field()
                    && !f.is_bool()
                    && f.enum_values().is_none()
                {
                    f.value.push(c);
                }
                WatchOutcome::None
            }
            _ => WatchOutcome::None,
        }
    }

    /// Advance a bool toggle or enum selector on the focused field.
    fn cycle_focused(&mut self, forward: bool) {
        let Some(f) = self.fields.get_mut(self.focus) else {
            return;
        };
        if f.is_bool() {
            let on = f.value == "true";
            f.value = (!on).to_string();
        } else if let Some(values) = f.enum_values().map(<[String]>::to_vec) {
            if values.is_empty() {
                return;
            }
            let cur = values.iter().position(|v| *v == f.value).unwrap_or(0);
            let next = if forward {
                (cur + 1) % values.len()
            } else {
                (cur + values.len() - 1) % values.len()
            };
            f.value = values[next].clone();
        }
    }

    /// Validate + coerce all fields into a `feed_register` payload, or return
    /// `WatchOutcome::None` after setting `last_error`.
    fn build_submit(&mut self) -> WatchOutcome {
        let mut feed_id = String::new();
        let mut cadence: Option<String> = None;
        let mut params = serde_json::Map::new();

        for f in &self.fields {
            let raw = f.value.trim();
            match f.spec.key.as_str() {
                FEED_ID_KEY => {
                    if raw.is_empty() {
                        self.last_error = Some("Feed ID is required.".into());
                        return WatchOutcome::None;
                    }
                    feed_id = raw.to_string();
                }
                CADENCE_KEY => {
                    // Only override when the user changed it from the default.
                    if !raw.is_empty() && raw != self.default_cadence {
                        cadence = Some(raw.to_string());
                    }
                }
                key => {
                    match coerce_param(&f.spec, raw) {
                        Ok(Some(v)) => {
                            params.insert(key.to_string(), v);
                        }
                        Ok(None) => {
                            // Empty optional — omit so the template default applies.
                            if f.spec.required {
                                self.last_error =
                                    Some(format!("{} is required.", f.spec.label));
                                return WatchOutcome::None;
                            }
                        }
                        Err(e) => {
                            self.last_error = Some(format!("{}: {e}", f.spec.label));
                            return WatchOutcome::None;
                        }
                    }
                }
            }
        }

        WatchOutcome::Submit {
            template: self.template.clone(),
            feed_id,
            params: Value::Object(params),
            cadence,
        }
    }
}

/// Coerce a raw field value into JSON per its kind. `Ok(None)` means "empty,
/// omit it"; `Err` is a validation message.
fn coerce_param(spec: &FeedParamSpecDto, raw: &str) -> Result<Option<Value>, String> {
    match &spec.kind {
        FeedParamKindDto::Bool => {
            // Bool always has a concrete value (toggle).
            Ok(Some(json!(raw == "true")))
        }
        _ if raw.is_empty() => Ok(None),
        FeedParamKindDto::Int => raw
            .parse::<i64>()
            .map(|n| Some(json!(n)))
            .map_err(|_| format!("'{raw}' is not a whole number")),
        FeedParamKindDto::List => {
            let items: Vec<Value> = raw
                .split([',', ' '])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| json!(s))
                .collect();
            Ok(Some(Value::Array(items)))
        }
        FeedParamKindDto::Since => crate::command::parse_since(raw).map(|iso| Some(json!(iso))),
        FeedParamKindDto::Enum(allowed) => {
            if allowed.iter().any(|a| a == raw) {
                Ok(Some(json!(raw)))
            } else {
                Err(format!("'{raw}' is not one of: {}", allowed.join(", ")))
            }
        }
        // Text / Path
        _ => Ok(Some(json!(raw))),
    }
}

pub fn render_watch_modal(state: &WatchModalState, frame: &mut Frame) {
    let area = frame.area();
    let modal_width = (area.width * 70 / 100)
        .max(44)
        .min(area.width.saturating_sub(4));
    let modal_height = area.height.saturating_sub(4).clamp(10, 26);
    let rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, rect);
    let title = match state.stage {
        WatchStage::PickTemplate => " Watch — pick a feed ",
        WatchStage::FillForm => " Watch — configure feed ",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let lines = match state.stage {
        WatchStage::PickTemplate => render_pick_lines(state),
        WatchStage::FillForm => render_form_lines(state),
    };
    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

fn render_pick_lines(state: &WatchModalState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (i, t) in state.templates.iter().enumerate() {
        let focused = i == state.template_index;
        let indicator = if focused { "▸ " } else { "  " };
        let name_style = if focused {
            Style::default()
                .fg(theme::TEXT)
                .bg(theme::SURFACE0)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::SUBTEXT1)
        };
        lines.push(Line::from(vec![
            Span::styled(indicator, Style::default().fg(Color::Yellow)),
            Span::styled(t.name.clone(), name_style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("    {}", t.description),
            Style::default().fg(theme::OVERLAY1),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " ↑↓ move · Enter select · Esc cancel",
        Style::default()
            .fg(theme::OVERLAY1)
            .add_modifier(Modifier::ITALIC),
    )));
    lines
}

fn render_form_lines(state: &WatchModalState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("Template: {}", state.template),
        Style::default().fg(theme::SUBTEXT0),
    )));
    lines.push(Line::from(""));

    let cadence_idx = state.fields.len().saturating_sub(1);
    for (i, f) in state.fields.iter().enumerate() {
        // "— advanced —" separator just before the cadence row.
        if i == cadence_idx {
            lines.push(Line::from(Span::styled(
                "— advanced —",
                Style::default().fg(theme::OVERLAY1),
            )));
        }

        let focused = i == state.focus;
        let indicator = if focused { "▸ " } else { "  " };
        let req = if f.spec.required { "*" } else { "" };
        let shown = render_field_value(f);
        let value_style = if focused {
            Style::default()
                .fg(theme::TEXT)
                .bg(theme::SURFACE0)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::SUBTEXT1)
        };
        lines.push(Line::from(vec![
            Span::styled(indicator, Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{}{}: ", f.spec.label, req),
                Style::default().fg(theme::SUBTEXT0),
            ),
            Span::styled(shown, value_style),
        ]));
        if focused && !f.spec.help.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("    {}", f.spec.help),
                Style::default().fg(theme::OVERLAY1),
            )));
        }
    }

    lines.push(Line::from(""));
    if let Some(ref err) = state.last_error {
        lines.push(Line::from(Span::styled(
            format!("error: {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    lines.push(Line::from(Span::styled(
        " Tab/↑↓ move · ←/→/space toggle · Enter register · Esc cancel",
        Style::default()
            .fg(theme::OVERLAY1)
            .add_modifier(Modifier::ITALIC),
    )));
    lines
}

/// How a field's current value reads on screen (with a caret on text fields).
fn render_field_value(f: &FieldState) -> String {
    match &f.spec.kind {
        FeedParamKindDto::Bool => {
            if f.value == "true" {
                "[on]".into()
            } else {
                "[off]".into()
            }
        }
        FeedParamKindDto::Enum(_) => format!("< {} >", f.value),
        _ if f.value.is_empty() => "_".into(),
        _ => format!("{}_", f.value),
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn typ(state: &mut WatchModalState, s: &str) {
        for c in s.chars() {
            state.handle_key(key(KeyCode::Char(c)));
        }
    }

    fn spec(key: &str, kind: FeedParamKindDto, required: bool, default: Option<Value>) -> FeedParamSpecDto {
        FeedParamSpecDto {
            key: key.into(),
            label: key.into(),
            kind,
            required,
            default,
            help: String::new(),
        }
    }

    fn fs_form() -> WatchModalState {
        let mut s = WatchModalState::new(vec![TemplateChoice {
            name: "filesystem/folder".into(),
            description: "watch a folder".into(),
        }]);
        s.enter_form(
            "filesystem/folder",
            vec![
                spec("root", FeedParamKindDto::Path, true, None),
                spec("recursive", FeedParamKindDto::Bool, false, Some(json!(true))),
                spec(
                    "include",
                    FeedParamKindDto::List,
                    false,
                    Some(json!(["**/*"])),
                ),
            ],
            "*/15 * * * *",
        );
        s
    }

    #[test]
    fn pick_stage_navigates_and_selects() {
        let mut s = WatchModalState::new(vec![
            TemplateChoice { name: "a/one".into(), description: String::new() },
            TemplateChoice { name: "b/two".into(), description: String::new() },
        ]);
        assert_eq!(s.handle_key(key(KeyCode::Down)), WatchOutcome::None);
        assert_eq!(s.template_index, 1);
        assert_eq!(
            s.handle_key(key(KeyCode::Enter)),
            WatchOutcome::TemplatePicked("b/two".into())
        );
    }

    #[test]
    fn enter_form_seeds_feed_id_params_and_cadence() {
        let s = fs_form();
        // feed_id first, root/recursive/include, cadence last = 5 rows.
        assert_eq!(s.fields.len(), 5);
        assert_eq!(s.fields[0].spec.key, FEED_ID_KEY);
        assert_eq!(s.fields[1].spec.key, "root");
        assert_eq!(s.fields[4].spec.key, CADENCE_KEY);
        // Defaults pre-filled.
        assert_eq!(s.fields[2].value, "true"); // recursive
        assert_eq!(s.fields[3].value, "**/*"); // include
        assert_eq!(s.fields[4].value, "*/15 * * * *"); // cadence
    }

    #[test]
    fn submit_blocked_until_required_filled() {
        let mut s = fs_form();
        // feed_id empty, root empty -> error on feed_id first.
        assert_eq!(s.handle_key(key(KeyCode::Enter)), WatchOutcome::None);
        assert!(s.last_error.as_deref().unwrap().contains("Feed ID"));

        // Fill feed_id, still missing root.
        typ(&mut s, "mynotes");
        assert_eq!(s.handle_key(key(KeyCode::Enter)), WatchOutcome::None);
        assert!(s.last_error.as_deref().unwrap().contains("root"));
    }

    #[test]
    fn submit_payload_matches_text_command_shape() {
        let mut s = fs_form();
        typ(&mut s, "mynotes"); // feed_id
        s.handle_key(key(KeyCode::Down)); // -> root
        typ(&mut s, "/Users/me/My Drive/Notes"); // spaced path, no quoting
        let outcome = s.handle_key(key(KeyCode::Enter));
        match outcome {
            WatchOutcome::Submit { template, feed_id, params, cadence } => {
                assert_eq!(template, "filesystem/folder");
                assert_eq!(feed_id, "mynotes");
                assert_eq!(params["root"], "/Users/me/My Drive/Notes");
                assert_eq!(params["recursive"], json!(true));
                assert_eq!(params["include"], json!(["**/*"]));
                // Cadence unchanged from default -> not overridden.
                assert_eq!(cadence, None);
            }
            other => panic!("expected Submit, got {other:?}"),
        }
    }

    #[test]
    fn bool_field_toggles_on_space_and_arrows() {
        let mut s = fs_form();
        s.focus = 2; // recursive (default true)
        s.handle_key(key(KeyCode::Char(' ')));
        assert_eq!(s.fields[2].value, "false");
        s.handle_key(key(KeyCode::Left));
        assert_eq!(s.fields[2].value, "true");
    }

    #[test]
    fn list_field_splits_on_whitespace_and_commas() {
        let mut s = fs_form();
        s.focus = 3; // include
        // Clear default, type new.
        for _ in 0.."**/*".len() {
            s.handle_key(key(KeyCode::Backspace));
        }
        typ(&mut s, "*.md, *.txt notes/**");
        s.focus = 0;
        typ(&mut s, "id");
        s.focus = 1;
        typ(&mut s, "/a/b/c");
        match s.handle_key(key(KeyCode::Enter)) {
            WatchOutcome::Submit { params, .. } => {
                assert_eq!(params["include"], json!(["*.md", "*.txt", "notes/**"]));
            }
            other => panic!("expected Submit, got {other:?}"),
        }
    }

    #[test]
    fn int_validation_rejects_non_numbers() {
        let mut s = WatchModalState::new(vec![]);
        s.enter_form(
            "gmail/inbox-archive",
            vec![spec("days_back", FeedParamKindDto::Int, false, Some(json!(7)))],
            "*/15 * * * *",
        );
        typ(&mut s, "id"); // feed_id
        s.focus = 1; // days_back, default "7"
        for _ in 0.."7".len() {
            s.handle_key(key(KeyCode::Backspace));
        }
        typ(&mut s, "abc");
        assert_eq!(s.handle_key(key(KeyCode::Enter)), WatchOutcome::None);
        assert!(s.last_error.as_deref().unwrap().contains("whole number"));
    }

    #[test]
    fn changed_cadence_becomes_override() {
        let mut s = fs_form();
        typ(&mut s, "id"); // feed_id
        s.focus = 1;
        typ(&mut s, "/a/b/c"); // root
        s.focus = 4; // cadence
        for _ in 0.."*/15 * * * *".len() {
            s.handle_key(key(KeyCode::Backspace));
        }
        typ(&mut s, "0 * * * *");
        match s.handle_key(key(KeyCode::Enter)) {
            WatchOutcome::Submit { cadence, .. } => {
                assert_eq!(cadence, Some("0 * * * *".into()));
            }
            other => panic!("expected Submit, got {other:?}"),
        }
    }

    /// End-to-end shape check (ARAWN-I-0058 T-E): a full modal flow — pick a
    /// template, seed the form from the schema the server would send, fill the
    /// required fields, submit — must produce a payload that deserializes into
    /// `arawn_service::FeedRegisterSpec` (the exact type `feed_register` takes).
    /// A filesystem path with spaces is typed verbatim, no quoting.
    #[test]
    fn submit_is_wire_compatible_with_feed_register() {
        use arawn_service::FeedRegisterSpec;

        // Stage 1 → pick filesystem/folder.
        let mut s = WatchModalState::new(vec![TemplateChoice {
            name: "filesystem/folder".into(),
            description: "watch a folder".into(),
        }]);
        assert_eq!(
            s.handle_key(key(KeyCode::Enter)),
            WatchOutcome::TemplatePicked("filesystem/folder".into())
        );

        // The event loop would fetch feed_schema; emulate its result.
        s.enter_form(
            "filesystem/folder",
            vec![
                spec("root", FeedParamKindDto::Path, true, None),
                spec("recursive", FeedParamKindDto::Bool, false, Some(json!(true))),
                spec("include", FeedParamKindDto::List, false, Some(json!(["**/*"]))),
            ],
            "*/15 * * * *",
        );

        typ(&mut s, "mynotes"); // feed_id
        s.focus = 1;
        typ(&mut s, "/Users/me/My Drive/Notes"); // spaced path, no quoting

        let outcome = s.handle_key(key(KeyCode::Enter));
        let WatchOutcome::Submit { template, feed_id, params, cadence } = outcome else {
            panic!("expected Submit, got {outcome:?}");
        };

        // Assemble the payload exactly as event_loop/watch.rs does, then prove
        // it round-trips through the real RPC arg type.
        let payload = json!({
            "template": template,
            "feed_id": feed_id,
            "params": params,
            "cadence": cadence,
        });
        let spec: FeedRegisterSpec =
            serde_json::from_value(payload).expect("payload is a valid FeedRegisterSpec");
        assert_eq!(spec.template, "filesystem/folder");
        assert_eq!(spec.feed_id, "mynotes");
        assert_eq!(spec.params["root"], "/Users/me/My Drive/Notes");
        assert!(spec.cadence.is_none(), "unchanged cadence is not overridden");
    }

    #[test]
    fn esc_cancels_in_both_stages() {
        let mut s = WatchModalState::new(vec![]);
        assert_eq!(s.handle_key(key(KeyCode::Esc)), WatchOutcome::Cancel);
        let mut f = fs_form();
        assert_eq!(f.handle_key(key(KeyCode::Esc)), WatchOutcome::Cancel);
    }
}

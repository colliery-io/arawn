//! Markdown to ratatui `Line`/`Span` conversion.
//!
//! Parses markdown using pulldown-cmark and produces styled ratatui text
//! suitable for rendering in the chat area.

use std::sync::LazyLock;

use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME: LazyLock<Theme> = LazyLock::new(|| {
    let ts = ThemeSet::load_defaults();
    ts.themes["base16-eighties.dark"].clone()
});

/// Parse a markdown string into styled ratatui `Line`s.
/// `max_width` constrains table column widths. Pass 0 for no constraint.
/// Parse a markdown string into styled ratatui `Line`s (no width constraint).
pub fn markdown_to_lines(text: &str) -> Vec<Line<'static>> {
    markdown_to_lines_with_width(text, 0)
}

/// Parse a markdown string into styled ratatui `Line`s.
/// `max_width` constrains table column widths. Pass 0 for no constraint.
pub fn markdown_to_lines_with_width(text: &str, max_width: usize) -> Vec<Line<'static>> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(text, opts);

    let mut renderer = MdRenderer::new(max_width);
    for event in parser {
        renderer.process(event);
    }
    renderer.finish()
}

const CODE_STYLE: Style = Style::new()
    .fg(Color::Rgb(180, 180, 180))
    .bg(Color::Rgb(30, 30, 40));

struct MdRenderer {
    lines: Vec<Line<'static>>,
    current_spans: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    current_style: Style,
    in_code_block: bool,
    code_lang: Option<String>,
    code_text: String,
    list_depth: usize,
    list_counters: Vec<Option<u64>>,
    list_item_start: bool,
    /// Table state
    in_table: bool,
    table_alignments: Vec<Alignment>,
    table_row: Vec<String>,
    table_is_header: bool,
    table_header: Vec<String>,
    table_rows: Vec<Vec<String>>,
    /// Link URL to display after text
    link_url: Option<String>,
    /// Max width for table rendering (0 = no constraint)
    max_width: usize,
}

impl MdRenderer {
    fn new(max_width: usize) -> Self {
        Self {
            lines: Vec::new(),
            current_spans: Vec::new(),
            style_stack: Vec::new(),
            current_style: Style::default(),
            in_code_block: false,
            code_lang: None,
            code_text: String::new(),
            list_depth: 0,
            list_counters: Vec::new(),
            list_item_start: false,
            in_table: false,
            table_alignments: Vec::new(),
            table_row: Vec::new(),
            table_is_header: false,
            table_header: Vec::new(),
            table_rows: Vec::new(),
            link_url: None,
            max_width,
        }
    }

    fn process(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => self.inline_code(&code),
            Event::SoftBreak | Event::HardBreak => self.line_break(),
            Event::Rule => {
                self.flush_line();
                self.lines.push(Line::from(Span::styled(
                    "───────────────────────────────",
                    Style::default().fg(Color::DarkGray),
                )));
            }
            _ => {}
        }
    }

    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                self.flush_line();
                let style = heading_style(level as u8);
                self.push_style(style);
            }
            Tag::Paragraph => {
                if self.list_depth == 0 && !self.in_table {
                    self.flush_line();
                }
            }
            Tag::CodeBlock(kind) => {
                self.flush_line();
                self.in_code_block = true;
                self.code_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        let l = lang.to_string();
                        if l.is_empty() { None } else { Some(l) }
                    }
                    _ => None,
                };
                self.code_text.clear();
            }
            Tag::Strong => {
                self.push_style(Style::default().add_modifier(Modifier::BOLD));
            }
            Tag::Emphasis => {
                self.push_style(Style::default().add_modifier(Modifier::ITALIC));
            }
            Tag::Strikethrough => {
                self.push_style(Style::default().add_modifier(Modifier::CROSSED_OUT));
            }
            Tag::Link { dest_url, .. } => {
                self.link_url = Some(dest_url.to_string());
                self.push_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::UNDERLINED),
                );
            }
            Tag::List(start) => {
                if self.list_depth == 0 {
                    self.flush_line();
                }
                self.list_depth += 1;
                self.list_counters.push(start);
            }
            Tag::Item => {
                self.flush_line();
                self.list_item_start = true;
            }
            Tag::BlockQuote(_) => {
                self.flush_line();
                self.push_style(Style::default().fg(Color::DarkGray));
            }
            Tag::Table(alignments) => {
                self.flush_line();
                self.in_table = true;
                self.table_alignments = alignments;
            }
            Tag::TableHead => {
                self.table_is_header = true;
            }
            Tag::TableRow => {
                self.table_row.clear();
            }
            Tag::TableCell => {
                // Accumulate cell text via self.text()
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                self.flush_line();
                self.pop_style();
            }
            TagEnd::Paragraph => {
                self.flush_line();
                if self.list_depth == 0 && !self.in_table {
                    self.push_blank();
                }
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                if let Some(ref lang) = self.code_lang {
                    self.lines.push(Line::from(Span::styled(
                        lang.clone(),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    )));
                }
                let highlighted = highlight_code(&self.code_text, self.code_lang.as_deref());
                self.lines.extend(highlighted);
                self.push_blank();
                self.code_lang = None;
                self.code_text.clear();
            }
            TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough => {
                self.pop_style();
            }
            TagEnd::Link => {
                // Append URL after link text if it's different from the text
                if let Some(url) = self.link_url.take() {
                    self.pop_style();
                    self.current_spans.push(Span::styled(
                        format!(" ({url})"),
                        Style::default().fg(Color::DarkGray),
                    ));
                } else {
                    self.pop_style();
                }
            }
            TagEnd::List(_) => {
                self.list_depth -= 1;
                self.list_counters.pop();
                if self.list_depth == 0 {
                    self.flush_line();
                }
            }
            TagEnd::Item => {
                self.flush_line();
            }
            TagEnd::BlockQuote(_) => {
                self.flush_line();
                self.pop_style();
            }
            TagEnd::Table => {
                self.in_table = false;
                self.emit_full_table();
                self.table_alignments.clear();
                self.table_header.clear();
                self.table_rows.clear();
                self.push_blank();
            }
            TagEnd::TableHead => {
                self.table_header = std::mem::take(&mut self.table_row);
                self.table_is_header = false;
            }
            TagEnd::TableRow => {
                if !self.table_is_header {
                    let row = std::mem::take(&mut self.table_row);
                    self.table_rows.push(row);
                }
            }
            TagEnd::TableCell => {
                let cell_text: String = self
                    .current_spans
                    .drain(..)
                    .map(|s| s.content.to_string())
                    .collect();
                self.table_row.push(cell_text);
            }
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        if self.in_code_block {
            self.code_text.push_str(text);
            return;
        }

        if self.list_item_start {
            self.list_item_start = false;
            let indent = "  ".repeat(self.list_depth.saturating_sub(1));
            let bullet = if let Some(Some(n)) = self.list_counters.last_mut() {
                let b = format!("{indent}{n}. ");
                *n += 1;
                b
            } else {
                format!("{indent}• ")
            };
            self.current_spans
                .push(Span::styled(bullet, Style::default().fg(Color::DarkGray)));
        }

        self.current_spans
            .push(Span::styled(text.to_string(), self.current_style));
    }

    fn inline_code(&mut self, code: &str) {
        let style = Style::default()
            .fg(Color::Rgb(220, 170, 110))
            .bg(Color::Rgb(40, 40, 50));
        self.current_spans
            .push(Span::styled(format!("`{code}`"), style));
    }

    fn line_break(&mut self) {
        self.flush_line();
    }

    fn flush_line(&mut self) {
        if !self.current_spans.is_empty() {
            let spans = std::mem::take(&mut self.current_spans);
            self.lines.push(Line::from(spans));
        }
    }

    /// Push a blank line, but only if the last line wasn't already blank.
    fn push_blank(&mut self) {
        let last_blank = self
            .lines
            .last()
            .is_some_and(|l| l.spans.is_empty() || l.width() == 0);
        if !last_blank {
            self.lines.push(Line::from(""));
        }
    }

    fn push_style(&mut self, style: Style) {
        self.style_stack.push(style);
        self.recompute_style();
    }

    fn pop_style(&mut self) {
        self.style_stack.pop();
        self.recompute_style();
    }

    fn recompute_style(&mut self) {
        let mut s = Style::default();
        for layer in &self.style_stack {
            s = s.patch(*layer);
        }
        self.current_style = s;
    }

    // --- Table rendering ---

    fn emit_full_table(&mut self) {
        let chrome = Style::default().fg(Color::DarkGray);
        let header_style = Style::default().add_modifier(Modifier::BOLD);

        // Calculate column widths from header + all rows
        let ncols = self.table_header.len();
        if ncols == 0 {
            return;
        }
        let mut col_widths = vec![0usize; ncols];
        for (i, cell) in self.table_header.iter().enumerate() {
            col_widths[i] = col_widths[i].max(cell.chars().count());
        }
        for row in &self.table_rows {
            for (i, cell) in row.iter().enumerate() {
                if i < ncols {
                    col_widths[i] = col_widths[i].max(cell.chars().count());
                }
            }
        }

        // Cap column widths to fit within max_width
        // Each column takes: 1 (│) + 1 (space) + width + 1 (space) = width + 3
        // Plus trailing │ = total = sum(widths) + 3*ncols + 1
        if self.max_width > 0 {
            let chrome_overhead = 3 * ncols + 1;
            let available = self.max_width.saturating_sub(chrome_overhead);
            let total_width: usize = col_widths.iter().sum();

            if total_width > available {
                // Distribute available width proportionally, with a minimum of 4 per column
                let min_col = 4usize;
                let min_total = min_col * ncols;

                if available <= min_total {
                    // Terminal too narrow — give each column the minimum
                    for w in &mut col_widths {
                        *w = min_col;
                    }
                } else {
                    // Scale proportionally
                    let scale = available as f64 / total_width as f64;
                    for w in &mut col_widths {
                        *w = ((*w as f64 * scale) as usize).max(min_col);
                    }
                    // Adjust rounding errors
                    let scaled_total: usize = col_widths.iter().sum();
                    if scaled_total > available {
                        // Trim the widest column
                        if let Some(widest) = col_widths.iter_mut().max() {
                            *widest = widest.saturating_sub(scaled_total - available);
                        }
                    }
                }
            }
        }

        // Emit header row
        self.emit_padded_row(
            &self.table_header.clone(),
            &col_widths,
            header_style,
            chrome,
        );

        // Emit separator
        let sep_parts: Vec<String> = col_widths.iter().map(|w| "─".repeat(w + 2)).collect();
        let sep = format!("├{}┤", sep_parts.join("┼"));
        self.lines.push(Line::from(Span::styled(sep, chrome)));

        // Emit data rows with horizontal separators between them
        let row_sep_parts: Vec<String> = col_widths.iter().map(|w| "─".repeat(w + 2)).collect();
        let row_sep = format!("├{}┤", row_sep_parts.join("┼"));
        let rows = self.table_rows.clone();
        for (i, row) in rows.iter().enumerate() {
            self.emit_padded_row(row, &col_widths, Style::default(), chrome);
            if i < rows.len() - 1 {
                self.lines.push(Line::from(Span::styled(row_sep.clone(), chrome)));
            }
        }
    }

    fn emit_padded_row(
        &mut self,
        row: &[String],
        col_widths: &[usize],
        cell_style: Style,
        chrome_style: Style,
    ) {
        // Word-wrap each cell into lines that fit the column width
        let wrapped_cells: Vec<Vec<String>> = col_widths
            .iter()
            .enumerate()
            .map(|(i, width)| {
                let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                wrap_text(cell, *width)
            })
            .collect();

        // How many visual rows this table row needs
        let max_lines = wrapped_cells.iter().map(|c| c.len()).max().unwrap_or(1);

        for line_idx in 0..max_lines {
            let mut spans = vec![Span::styled("│", chrome_style)];
            for (col, width) in col_widths.iter().enumerate() {
                let text = wrapped_cells
                    .get(col)
                    .and_then(|lines| lines.get(line_idx))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let padded = format!(" {text:<width$} ", width = width);
                spans.push(Span::styled(padded, cell_style));
                spans.push(Span::styled("│", chrome_style));
            }
            self.lines.push(Line::from(spans));
        }
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        self.flush_line();
        // Trim trailing blank lines
        while self
            .lines
            .last()
            .is_some_and(|l| l.spans.is_empty() || l.width() == 0)
        {
            self.lines.pop();
        }
        self.lines
    }
}

/// Syntax-highlight a code block, returning one Line per source line.
/// Falls back to plain CODE_STYLE if the language isn't recognized.
fn highlight_code(code: &str, lang: Option<&str>) -> Vec<Line<'static>> {
    use syntect::easy::HighlightLines;
    use syntect::highlighting::FontStyle;

    let syntax = lang
        .and_then(|l| SYNTAX_SET.find_syntax_by_token(l))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, &THEME);
    let bg = Color::Rgb(30, 30, 40);

    code.lines()
        .map(|line| {
            match highlighter.highlight_line(line, &SYNTAX_SET) {
                Ok(ranges) => {
                    let mut spans: Vec<Span<'static>> =
                        vec![Span::styled(" ", Style::default().bg(bg))];
                    for (style, text) in ranges {
                        let fg =
                            Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                        let mut ratatui_style = Style::default().fg(fg).bg(bg);
                        if style.font_style.contains(FontStyle::BOLD) {
                            ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                        }
                        if style.font_style.contains(FontStyle::ITALIC) {
                            ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                        }
                        spans.push(Span::styled(text.to_string(), ratatui_style));
                    }
                    Line::from(spans)
                }
                Err(_) => {
                    // Fallback: plain styled
                    Line::from(Span::styled(format!(" {line}"), CODE_STYLE))
                }
            }
        })
        .collect()
}

fn heading_style(level: u8) -> Style {
    let color = match level {
        1 => Color::Cyan,
        2 => Color::Blue,
        3 => Color::Magenta,
        _ => Color::White,
    };
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

/// Word-wrap text to fit within a given width. Breaks on word boundaries
/// where possible, hard-breaks long words that exceed the width.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_len = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        if current_len == 0 {
            // First word on line
            if word_len <= width {
                current_line.push_str(word);
                current_len = word_len;
            } else {
                // Hard-break long word
                let mut chars = word.chars();
                while current_len < word_len {
                    let chunk: String = chars.by_ref().take(width).collect();
                    let chunk_len = chunk.chars().count();
                    if chunk_len == 0 {
                        break;
                    }
                    if !current_line.is_empty() {
                        lines.push(current_line);
                    }
                    current_line = chunk;
                    current_len = chunk_len;
                    if current_len >= width {
                        lines.push(current_line);
                        current_line = String::new();
                        current_len = 0;
                    }
                }
            }
        } else if current_len + 1 + word_len <= width {
            // Fits on current line with a space
            current_line.push(' ');
            current_line.push_str(word);
            current_len += 1 + word_len;
        } else {
            // Doesn't fit — start new line
            lines.push(current_line);
            if word_len <= width {
                current_line = word.to_string();
                current_len = word_len;
            } else {
                // Hard-break long word
                current_line = String::new();
                current_len = 0;
                let mut chars = word.chars();
                let total = word_len;
                let mut consumed = 0;
                while consumed < total {
                    let chunk: String = chars.by_ref().take(width).collect();
                    let chunk_len = chunk.chars().count();
                    if chunk_len == 0 {
                        break;
                    }
                    consumed += chunk_len;
                    if consumed < total || chunk_len == width {
                        lines.push(chunk);
                    } else {
                        current_line = chunk;
                        current_len = chunk_len;
                    }
                }
            }
        }
    }

    if !current_line.is_empty() || lines.is_empty() {
        lines.push(current_line);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans_text(lines: &[Line]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn plain_text() {
        let lines = markdown_to_lines("Hello world");
        let text = spans_text(&lines);
        assert!(text.contains("Hello world"));
    }

    #[test]
    fn heading_levels() {
        let lines = markdown_to_lines("# H1\n## H2\n### H3");
        let text = spans_text(&lines);
        assert!(text.contains("H1"));
        assert!(text.contains("H2"));
        assert!(text.contains("H3"));
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Cyan));
        assert_eq!(lines[1].spans[0].style.fg, Some(Color::Blue));
        assert_eq!(lines[2].spans[0].style.fg, Some(Color::Magenta));
    }

    #[test]
    fn bold_and_italic() {
        let lines = markdown_to_lines("This is **bold** and *italic* text");
        let bold_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.as_ref() == "bold")
            .unwrap();
        assert!(bold_span.style.add_modifier.contains(Modifier::BOLD));
        let italic_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.as_ref() == "italic")
            .unwrap();
        assert!(italic_span.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn inline_code() {
        let lines = markdown_to_lines("Use `cargo test` to run");
        let code_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("cargo test"))
            .unwrap();
        assert_eq!(code_span.style.fg, Some(Color::Rgb(220, 170, 110)));
    }

    #[test]
    fn fenced_code_block() {
        let lines = markdown_to_lines("```rust\nfn main() {}\n```");
        let text = spans_text(&lines);
        assert!(text.contains("rust"));
        assert!(text.contains("fn"));
        assert!(text.contains("main"));
        // Should have syntax-highlighted spans (multiple spans per code line)
        let code_line = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains("fn")))
            .unwrap();
        assert!(
            code_line.spans.len() > 1,
            "code should have multiple highlighted spans"
        );
    }

    #[test]
    fn unordered_list() {
        let lines = markdown_to_lines("- item one\n- item two\n- item three");
        let text = spans_text(&lines);
        assert!(text.contains("•"));
        assert!(text.contains("item one"));
        assert!(text.contains("item three"));
    }

    #[test]
    fn ordered_list() {
        let lines = markdown_to_lines("1. first\n2. second\n3. third");
        let text = spans_text(&lines);
        assert!(text.contains("1."));
        assert!(text.contains("first"));
    }

    #[test]
    fn table_renders_aligned() {
        let lines = markdown_to_lines(
            "| Name  | Age | City          |\n|-------|-----|---------------|\n| Alice | 30  | New York      |\n| Bob   | 25  | San Francisco |",
        );
        let text = spans_text(&lines);
        assert!(text.contains("Name"));
        assert!(text.contains("Alice"));
        assert!(text.contains("San Francisco"));
        assert!(text.contains("│"));
        // Check alignment: all rows should have same width
        let table_lines: Vec<&str> = text.lines().filter(|l| l.contains("│")).collect();
        assert!(
            table_lines.len() >= 3,
            "should have header + separator + 2 data rows"
        );
        let widths: Vec<usize> = table_lines.iter().map(|l| l.len()).collect();
        assert!(
            widths.windows(2).all(|w| w[0] == w[1]),
            "all table rows should be same width, got widths: {widths:?}\n{text}"
        );
    }

    #[test]
    fn link_shows_url() {
        let lines = markdown_to_lines("Check [the docs](https://example.com) here");
        let text = spans_text(&lines);
        assert!(text.contains("the docs"));
        assert!(text.contains("https://example.com"));
    }

    #[test]
    fn no_double_blank_lines() {
        let lines = markdown_to_lines("# Heading\n\nParagraph one.\n\nParagraph two.");
        let mut consecutive_blanks = 0;
        for line in &lines {
            if line.spans.is_empty() || line.width() == 0 {
                consecutive_blanks += 1;
                assert!(
                    consecutive_blanks <= 1,
                    "should not have consecutive blank lines"
                );
            } else {
                consecutive_blanks = 0;
            }
        }
    }

    #[test]
    fn no_trailing_blanks() {
        let lines = markdown_to_lines("Hello world\n\n");
        let last = lines.last().unwrap();
        assert!(last.width() > 0, "should not end with blank line");
    }
}

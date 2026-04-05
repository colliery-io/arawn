use ratatui::style::{Color, Modifier};

/// Render a TestBackend buffer to a deterministic string for snapshot comparison.
/// Text only — no style information. Good for layout verification.
#[cfg(test)]
pub fn buffer_to_snapshot(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String {
    let buf = terminal.backend().buffer();
    let mut lines = Vec::new();

    for y in 0..buf.area.height {
        let mut line = String::new();
        for x in 0..buf.area.width {
            if let Some(cell) = buf.cell((x, y)) {
                line.push_str(cell.symbol());
            }
        }
        let trimmed = line.trim_end();
        lines.push(trimmed.to_string());
    }

    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

/// Render a TestBackend buffer with inline style annotations.
/// Style changes are marked with `«fg:Color,bg:Color,mod»` tags.
/// Only emits a tag when the style changes from the previous cell.
/// This captures colors and modifiers alongside the text content.
#[cfg(test)]
pub fn buffer_to_styled_snapshot(
    terminal: &ratatui::Terminal<ratatui::backend::TestBackend>,
) -> String {
    let buf = terminal.backend().buffer();
    let mut lines = Vec::new();

    for y in 0..buf.area.height {
        let mut line = String::new();
        let mut prev_style: Option<(Color, Color, Modifier)> = None;

        for x in 0..buf.area.width {
            if let Some(cell) = buf.cell((x, y)) {
                let fg = cell.fg;
                let bg = cell.bg;
                let mods = cell.modifier;
                let current = (fg, bg, mods);

                if prev_style != Some(current) {
                    let tag = format_style_tag(fg, bg, mods);
                    if !tag.is_empty() {
                        line.push_str(&format!("«{tag}»"));
                    }
                    prev_style = Some(current);
                }

                line.push_str(cell.symbol());
            }
        }

        let trimmed = line.trim_end();
        lines.push(trimmed.to_string());
    }

    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

#[cfg(test)]
fn format_style_tag(fg: Color, bg: Color, mods: Modifier) -> String {
    let mut parts = Vec::new();

    match fg {
        Color::Reset => {}
        Color::White => parts.push("fg:White".to_string()),
        Color::Black => parts.push("fg:Black".to_string()),
        Color::Red => parts.push("fg:Red".to_string()),
        Color::Green => parts.push("fg:Green".to_string()),
        Color::Yellow => parts.push("fg:Yellow".to_string()),
        Color::Blue => parts.push("fg:Blue".to_string()),
        Color::Magenta => parts.push("fg:Magenta".to_string()),
        Color::Cyan => parts.push("fg:Cyan".to_string()),
        Color::DarkGray => parts.push("fg:DarkGray".to_string()),
        Color::Gray => parts.push("fg:Gray".to_string()),
        _ => parts.push(format!("fg:{fg:?}")),
    }

    match bg {
        Color::Reset => {}
        Color::DarkGray => parts.push("bg:DarkGray".to_string()),
        Color::White => parts.push("bg:White".to_string()),
        _ => parts.push(format!("bg:{bg:?}")),
    }

    if mods.contains(Modifier::BOLD) {
        parts.push("bold".to_string());
    }
    if mods.contains(Modifier::ITALIC) {
        parts.push("italic".to_string());
    }
    if mods.contains(Modifier::UNDERLINED) {
        parts.push("underline".to_string());
    }

    parts.join(",")
}

//! Display-width measurement helpers.
//!
//! Cell columns in a terminal — not bytes, not chars. CJK ideographs and
//! many emoji occupy 2 cells; combining marks and ZWJ sequences occupy 0.
//! Anything that drives rendering geometry (cursor position, fill counts,
//! truncation caps, alignment padding) must measure in cells.

use unicode_width::UnicodeWidthStr;

/// Display width (cells) of `s` in a fixed-width terminal.
pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Truncate `s` to fit within `max` display cells, appending `…` if truncated.
/// `…` is 1 cell; if `max == 0` returns an empty string.
pub fn truncate_display(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if display_width(s) <= max {
        return s.to_string();
    }
    let budget = max.saturating_sub(1).max(1);
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = UnicodeWidthStr::width(ch.to_string().as_str());
        if w + cw > budget {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}

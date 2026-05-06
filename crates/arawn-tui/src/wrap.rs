//! Word-wrap styled `Line`s into the chat area's exact visual width.
//!
//! Why we own this instead of leaning on `Paragraph::wrap`: the chat scroll
//! math needs to know exactly how many wrapped lines content takes so it
//! can render the right window. Letting ratatui wrap internally — while we
//! estimate wrap counts externally — produces a mismatch that drops
//! content off the bottom of the chat area when the estimate disagrees
//! with ratatui's actual wrap. Wrapping ourselves makes the count exact.
//!
//! Also splits embedded `\n` in spans (streaming text from LLMs has them)
//! into separate lines — `Span` is conceptually a single visual line and
//! ratatui doesn't split spans on newlines.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Wrap input lines to `width`. Returns concrete pre-wrapped lines that
/// the chat renderer can slice + blit without further wrapping.
///
/// Preserves span styles. Where styles change mid-word, we keep the
/// run-of-same-style as a single span — wrapping happens at whitespace
/// boundaries between styled runs.
///
/// Returns `'static` lines (owned strings) so the caller can build them
/// from borrowed app state and still pass them onward.
pub fn wrap_lines<'a>(input: Vec<Line<'a>>, width: usize) -> Vec<Line<'static>> {
    let owned: Vec<Line<'static>> = input.into_iter().map(into_owned).collect();
    if width == 0 {
        return owned;
    }
    let mut out = Vec::with_capacity(owned.len());
    for line in owned {
        for segment in split_newlines(line) {
            wrap_one(segment, width, &mut out);
        }
    }
    out
}

/// Force every span into an owned `Cow<'static, str>` so the resulting
/// `Line<'static>` is detached from any borrowed source.
fn into_owned(line: Line<'_>) -> Line<'static> {
    let spans: Vec<Span<'static>> = line
        .spans
        .into_iter()
        .map(|s| Span::styled(s.content.into_owned(), s.style))
        .collect();
    Line::from(spans)
}

/// If any span contains `\n`, split the line into multiple lines along
/// those newlines, preserving span styles. No-op when no newlines exist.
fn split_newlines(line: Line<'static>) -> Vec<Line<'static>> {
    if !line.spans.iter().any(|s| s.content.contains('\n')) {
        return vec![line];
    }
    let mut out: Vec<Vec<Span<'static>>> = vec![Vec::new()];
    for span in line.spans {
        let style = span.style;
        let mut text = span.content.into_owned();
        loop {
            match text.find('\n') {
                Some(idx) => {
                    let before = text[..idx].to_string();
                    if !before.is_empty() {
                        out.last_mut().unwrap().push(Span::styled(before, style));
                    }
                    out.push(Vec::new());
                    text = text[idx + 1..].to_string();
                }
                None => {
                    if !text.is_empty() {
                        out.last_mut().unwrap().push(Span::styled(text, style));
                    }
                    break;
                }
            }
        }
    }
    out.into_iter().map(Line::from).collect()
}

/// Token kind: a contiguous run of whitespace or non-whitespace chars,
/// tagged with the source span's style.
struct Tok {
    text: String,
    style: Style,
    is_ws: bool,
    width: usize,
}

fn tokenize(line: &Line<'static>) -> Vec<Tok> {
    let mut toks: Vec<Tok> = Vec::new();
    for span in &line.spans {
        let style = span.style;
        let mut buf = String::new();
        let mut buf_is_ws = false;
        for ch in span.content.chars() {
            let ch_is_ws = ch.is_whitespace();
            if !buf.is_empty() && ch_is_ws != buf_is_ws {
                let width = crate::width::display_width(&buf);
                toks.push(Tok {
                    text: std::mem::take(&mut buf),
                    style,
                    is_ws: buf_is_ws,
                    width,
                });
            }
            if buf.is_empty() {
                buf_is_ws = ch_is_ws;
            }
            buf.push(ch);
        }
        if !buf.is_empty() {
            let width = crate::width::display_width(&buf);
            toks.push(Tok {
                text: buf,
                style,
                is_ws: buf_is_ws,
                width,
            });
        }
    }
    toks
}

fn wrap_one(line: Line<'static>, width: usize, out: &mut Vec<Line<'static>>) {
    let toks = tokenize(&line);
    if toks.is_empty() {
        out.push(Line::from(""));
        return;
    }

    let mut cur: Vec<Span<'static>> = Vec::new();
    let mut cur_w: usize = 0;
    // True until we've emitted any text from this logical line. Lets us
    // keep leading whitespace as intentional indent, while still dropping
    // whitespace at the start of *wrap continuations*.
    let mut on_original_line = true;

    for tok in toks {
        if tok.is_ws {
            // Keep whitespace if it fits and either we're still on the
            // original (un-wrapped) line OR we already have content on
            // the current visual line. Drop it otherwise.
            if cur_w + tok.width <= width && (cur_w > 0 || on_original_line) {
                cur.push(Span::styled(tok.text, tok.style));
                cur_w += tok.width;
            }
            // else drop
        } else if tok.width <= width {
            if cur_w + tok.width > width {
                out.push(Line::from(std::mem::take(&mut cur)));
                cur_w = 0;
                on_original_line = false;
            }
            cur.push(Span::styled(tok.text, tok.style));
            cur_w += tok.width;
        } else {
            // Word longer than the available width — hard-break it. We keep
            // already-accumulated content first, then chunk the long word
            // into width-sized pieces.
            if cur_w > 0 {
                out.push(Line::from(std::mem::take(&mut cur)));
                cur_w = 0;
                on_original_line = false;
            }
            let mut chunk = String::new();
            let mut chunk_w = 0;
            for ch in tok.text.chars() {
                if chunk_w + 1 > width {
                    cur.push(Span::styled(std::mem::take(&mut chunk), tok.style));
                    out.push(Line::from(std::mem::take(&mut cur)));
                    cur_w = 0;
                    chunk_w = 0;
                    on_original_line = false;
                }
                chunk.push(ch);
                chunk_w += 1;
            }
            if !chunk.is_empty() {
                cur.push(Span::styled(chunk, tok.style));
                cur_w = chunk_w;
            }
        }
    }
    if !cur.is_empty() {
        out.push(Line::from(cur));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Modifier};

    fn plain(s: &str) -> Line<'static> {
        Line::from(s.to_string())
    }

    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn passthrough_when_under_width() {
        let lines = vec![plain("hello world")];
        let wrapped = wrap_lines(lines, 80);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(line_text(&wrapped[0]), "hello world");
    }

    #[test]
    fn word_wraps_at_whitespace() {
        let lines = vec![plain("the quick brown fox jumps")];
        let wrapped = wrap_lines(lines, 12);
        let texts: Vec<_> = wrapped.iter().map(line_text).collect();
        // 12-wide canvas; greedy fill should produce 3 lines.
        assert!(texts.len() >= 2, "expected wrap, got {texts:?}");
        // No line exceeds width
        for line in &wrapped {
            let w: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
            assert!(w <= 12, "line over width: {:?} ({} chars)", line_text(line), w);
        }
        // Concatenation preserves words (whitespace may differ at boundaries)
        let joined = texts.join(" ").replace("  ", " ");
        assert!(joined.contains("the"));
        assert!(joined.contains("quick"));
        assert!(joined.contains("jumps"));
    }

    #[test]
    fn hard_breaks_oversize_word() {
        let lines = vec![plain("aaaaaaaaaaaa")]; // 12 chars
        let wrapped = wrap_lines(lines, 5);
        assert!(wrapped.len() >= 3, "expected 3+ lines, got {}", wrapped.len());
        for line in &wrapped {
            let w: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
            assert!(w <= 5);
        }
    }

    #[test]
    fn splits_on_embedded_newlines() {
        let line = Line::from(vec![Span::raw("hello\nworld\n!".to_string())]);
        let wrapped = wrap_lines(vec![line], 80);
        assert_eq!(wrapped.len(), 3);
        assert_eq!(line_text(&wrapped[0]), "hello");
        assert_eq!(line_text(&wrapped[1]), "world");
        assert_eq!(line_text(&wrapped[2]), "!");
    }

    #[test]
    fn preserves_span_styles_through_wrap() {
        let line = Line::from(vec![
            Span::styled(
                "hello ".to_string(),
                Style::default().fg(Color::Red),
            ),
            Span::styled(
                "world example".to_string(),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]);
        let wrapped = wrap_lines(vec![line], 8);
        // Whatever wrapping shape, every span should still carry its color.
        let mut saw_red = false;
        let mut saw_green = false;
        for line in &wrapped {
            for span in &line.spans {
                if span.style.fg == Some(Color::Red) {
                    saw_red = true;
                }
                if span.style.fg == Some(Color::Green) {
                    saw_green = true;
                }
            }
        }
        assert!(saw_red);
        assert!(saw_green);
    }

    #[test]
    fn empty_line_preserved() {
        let lines = vec![Line::from("")];
        let wrapped = wrap_lines(lines, 80);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(line_text(&wrapped[0]), "");
    }

    #[test]
    fn zero_width_is_passthrough() {
        let lines = vec![plain("anything goes")];
        let wrapped = wrap_lines(lines.clone(), 0);
        assert_eq!(wrapped.len(), lines.len());
    }
}

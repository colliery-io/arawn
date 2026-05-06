//! Markdown → Atlassian Document Format converter.
//!
//! Jira's REST API v3 requires rich-text fields (description, comment
//! body, environment, multi-line custom fields) to be passed as ADF —
//! a JSON document with explicit block / inline / mark structure.
//! Hand-building ADF in agent prompts is brittle, so we accept
//! markdown and convert it here.
//!
//! Coverage handles the cases an LLM realistically emits:
//!
//! - Paragraphs with bold (`**x**`), italic (`*x*` / `_x_`), inline code
//!   (`` `x` ``), and links (`[text](url)`).
//! - Headings level 1–6.
//! - Bullet and ordered lists (one level — nested lists flatten).
//! - Fenced code blocks with optional language hint.
//! - Block quotes.
//! - Horizontal rules.
//! - Hard line breaks.
//!
//! Anything not covered round-trips as plain text. The output is always
//! a valid ADF document, even on empty input (returns a doc with one
//! empty paragraph — which Jira accepts).

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde_json::{Value, json};

/// Convert markdown to an ADF document. Always returns
/// `{type: "doc", version: 1, content: [...]}`.
pub fn md_to_adf(md: &str) -> Value {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, opts);
    let mut blocks: Vec<Value> = Vec::new();
    let mut state = AdfBuilder::default();

    for event in parser {
        state.process(event, &mut blocks);
    }
    state.flush_pending(&mut blocks);

    if blocks.is_empty() {
        // ADF requires at least one block. Empty paragraph is the
        // canonical placeholder.
        blocks.push(json!({ "type": "paragraph", "content": [] }));
    }

    json!({
        "type": "doc",
        "version": 1,
        "content": blocks,
    })
}

#[derive(Default)]
struct AdfBuilder {
    /// Stack of inline marks active at the current parser position
    /// (`strong`, `em`, `code`, `link`). Pushed on Tag::Start, popped
    /// on TagEnd::End.
    marks: Vec<Value>,
    /// Pending inline content for the currently-open block.
    inline: Vec<Value>,
    /// What kind of block we're currently filling.
    current_block: BlockKind,
    /// For ordered/bullet lists — buffer of accumulated list items.
    list_items: Vec<Value>,
}

#[derive(Default, Debug, Clone)]
enum BlockKind {
    #[default]
    None,
    Paragraph,
    Heading(u8),
    BulletList,
    OrderedList,
    /// Inside a list item; inline goes here, the item is closed on
    /// TagEnd::Item.
    ListItem(Box<BlockKind>),
    BlockQuote,
    CodeBlock {
        language: Option<String>,
        text: String,
    },
}

impl AdfBuilder {
    fn process(&mut self, event: Event<'_>, blocks: &mut Vec<Value>) {
        match event {
            Event::Start(Tag::Paragraph) => self.start_paragraph(),
            Event::End(TagEnd::Paragraph) => self.end_paragraph(blocks),

            Event::Start(Tag::Heading { level, .. }) => self.start_heading(level),
            Event::End(TagEnd::Heading(_)) => self.end_heading(blocks),

            Event::Start(Tag::List(start_num)) => self.start_list(start_num),
            Event::End(TagEnd::List(_)) => self.end_list(blocks),

            Event::Start(Tag::Item) => self.start_item(),
            Event::End(TagEnd::Item) => self.end_item(),

            Event::Start(Tag::BlockQuote(_)) => self.start_block_quote(),
            Event::End(TagEnd::BlockQuote(_)) => self.end_block_quote(blocks),

            Event::Start(Tag::CodeBlock(kind)) => {
                self.start_code_block(match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(s) => {
                        let s = s.to_string();
                        if s.is_empty() { None } else { Some(s) }
                    }
                    _ => None,
                });
            }
            Event::End(TagEnd::CodeBlock) => self.end_code_block(blocks),

            Event::Start(Tag::Strong) => self.push_mark(json!({ "type": "strong" })),
            Event::End(TagEnd::Strong) => self.pop_mark(),

            Event::Start(Tag::Emphasis) => self.push_mark(json!({ "type": "em" })),
            Event::End(TagEnd::Emphasis) => self.pop_mark(),

            Event::Start(Tag::Strikethrough) => self.push_mark(json!({ "type": "strike" })),
            Event::End(TagEnd::Strikethrough) => self.pop_mark(),

            Event::Start(Tag::Link { dest_url, .. }) => self.push_mark(json!({
                "type": "link",
                "attrs": { "href": dest_url.to_string() },
            })),
            Event::End(TagEnd::Link) => self.pop_mark(),

            Event::Code(s) => self.push_inline_code(&s),
            Event::Text(s) => self.push_text(&s),
            Event::SoftBreak => self.push_text(" "),
            Event::HardBreak => self.push_hard_break(),

            Event::Rule => blocks.push(json!({ "type": "rule" })),

            _ => {} // ignore tables/footnotes for now
        }
    }

    // ── Block transitions ────────────────────────────────────────────

    fn start_paragraph(&mut self) {
        match &self.current_block {
            BlockKind::ListItem(_) | BlockKind::BlockQuote => {
                // Inline already routed correctly; nothing to do.
            }
            _ => self.current_block = BlockKind::Paragraph,
        }
    }

    fn end_paragraph(&mut self, blocks: &mut Vec<Value>) {
        match &self.current_block {
            BlockKind::ListItem(_) | BlockKind::BlockQuote => {}
            _ => {
                let content = std::mem::take(&mut self.inline);
                if !content.is_empty() {
                    blocks.push(json!({ "type": "paragraph", "content": content }));
                }
                self.current_block = BlockKind::None;
            }
        }
    }

    fn start_heading(&mut self, level: HeadingLevel) {
        let lvl = match level {
            HeadingLevel::H1 => 1,
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            HeadingLevel::H4 => 4,
            HeadingLevel::H5 => 5,
            HeadingLevel::H6 => 6,
        };
        self.current_block = BlockKind::Heading(lvl);
        self.inline.clear();
    }

    fn end_heading(&mut self, blocks: &mut Vec<Value>) {
        let level = match self.current_block {
            BlockKind::Heading(l) => l,
            _ => 1,
        };
        let content = std::mem::take(&mut self.inline);
        blocks.push(json!({
            "type": "heading",
            "attrs": { "level": level },
            "content": content,
        }));
        self.current_block = BlockKind::None;
    }

    fn start_list(&mut self, start_num: Option<u64>) {
        self.list_items.clear();
        self.current_block = if start_num.is_some() {
            BlockKind::OrderedList
        } else {
            BlockKind::BulletList
        };
    }

    fn end_list(&mut self, blocks: &mut Vec<Value>) {
        let items = std::mem::take(&mut self.list_items);
        let kind = match &self.current_block {
            BlockKind::OrderedList => "orderedList",
            _ => "bulletList",
        };
        blocks.push(json!({ "type": kind, "content": items }));
        self.current_block = BlockKind::None;
    }

    fn start_item(&mut self) {
        let parent = std::mem::take(&mut self.current_block);
        self.current_block = BlockKind::ListItem(Box::new(parent));
        self.inline.clear();
    }

    fn end_item(&mut self) {
        let content = std::mem::take(&mut self.inline);
        // ADF list items wrap their inline in a paragraph block.
        let paragraph = json!({ "type": "paragraph", "content": content });
        self.list_items
            .push(json!({ "type": "listItem", "content": [paragraph] }));
        if let BlockKind::ListItem(parent) = std::mem::take(&mut self.current_block) {
            self.current_block = *parent;
        }
    }

    fn start_block_quote(&mut self) {
        self.current_block = BlockKind::BlockQuote;
        self.inline.clear();
    }

    fn end_block_quote(&mut self, blocks: &mut Vec<Value>) {
        let content = std::mem::take(&mut self.inline);
        let paragraph = json!({ "type": "paragraph", "content": content });
        blocks.push(json!({ "type": "blockquote", "content": [paragraph] }));
        self.current_block = BlockKind::None;
    }

    fn start_code_block(&mut self, language: Option<String>) {
        self.current_block = BlockKind::CodeBlock {
            language,
            text: String::new(),
        };
    }

    fn end_code_block(&mut self, blocks: &mut Vec<Value>) {
        if let BlockKind::CodeBlock { language, text } = std::mem::take(&mut self.current_block) {
            let trimmed = text.trim_end_matches('\n').to_string();
            let mut block = json!({
                "type": "codeBlock",
                "content": [{ "type": "text", "text": trimmed }],
            });
            if let Some(lang) = language {
                block["attrs"] = json!({ "language": lang });
            }
            blocks.push(block);
        }
    }

    fn flush_pending(&mut self, blocks: &mut Vec<Value>) {
        if !self.inline.is_empty() {
            let content = std::mem::take(&mut self.inline);
            blocks.push(json!({ "type": "paragraph", "content": content }));
        }
    }

    // ── Inline routing ───────────────────────────────────────────────

    fn push_text(&mut self, text: &str) {
        if let BlockKind::CodeBlock { text: ref mut buf, .. } = self.current_block {
            buf.push_str(text);
            return;
        }
        self.inline.push(self.text_node(text));
    }

    fn push_inline_code(&mut self, text: &str) {
        let mut node = json!({ "type": "text", "text": text });
        let mut marks: Vec<Value> = self.marks.clone();
        marks.push(json!({ "type": "code" }));
        node["marks"] = json!(marks);
        self.inline.push(node);
    }

    fn push_hard_break(&mut self) {
        self.inline.push(json!({ "type": "hardBreak" }));
    }

    fn text_node(&self, text: &str) -> Value {
        let mut node = json!({ "type": "text", "text": text });
        if !self.marks.is_empty() {
            node["marks"] = json!(self.marks);
        }
        node
    }

    fn push_mark(&mut self, mark: Value) {
        self.marks.push(mark);
    }

    fn pop_mark(&mut self) {
        self.marks.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_produces_doc_with_empty_paragraph() {
        let adf = md_to_adf("");
        assert_eq!(adf["type"], "doc");
        assert_eq!(adf["version"], 1);
        assert_eq!(adf["content"][0]["type"], "paragraph");
    }

    #[test]
    fn plain_paragraph() {
        let adf = md_to_adf("hello world");
        assert_eq!(adf["content"][0]["type"], "paragraph");
        assert_eq!(adf["content"][0]["content"][0]["text"], "hello world");
    }

    #[test]
    fn bold_and_italic() {
        let adf = md_to_adf("this is **bold** and *italic*.");
        let content = adf["content"][0]["content"].as_array().unwrap();
        // Find the bold span
        let bold = content.iter().find(|n| n["text"] == "bold").unwrap();
        assert_eq!(bold["marks"][0]["type"], "strong");
        let italic = content.iter().find(|n| n["text"] == "italic").unwrap();
        assert_eq!(italic["marks"][0]["type"], "em");
    }

    #[test]
    fn inline_code() {
        let adf = md_to_adf("call `foo()`");
        let content = adf["content"][0]["content"].as_array().unwrap();
        let code = content.iter().find(|n| n["text"] == "foo()").unwrap();
        assert_eq!(code["marks"][0]["type"], "code");
    }

    #[test]
    fn heading_levels() {
        let adf = md_to_adf("# H1\n\n## H2\n\n### H3");
        assert_eq!(adf["content"][0]["type"], "heading");
        assert_eq!(adf["content"][0]["attrs"]["level"], 1);
        assert_eq!(adf["content"][1]["attrs"]["level"], 2);
        assert_eq!(adf["content"][2]["attrs"]["level"], 3);
    }

    #[test]
    fn bullet_list() {
        let adf = md_to_adf("- one\n- two\n- three");
        assert_eq!(adf["content"][0]["type"], "bulletList");
        let items = adf["content"][0]["content"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["type"], "listItem");
        assert_eq!(items[0]["content"][0]["type"], "paragraph");
    }

    #[test]
    fn ordered_list() {
        let adf = md_to_adf("1. first\n2. second");
        assert_eq!(adf["content"][0]["type"], "orderedList");
    }

    #[test]
    fn fenced_code_block_with_language() {
        let adf = md_to_adf("```rust\nfn main() {}\n```");
        let block = &adf["content"][0];
        assert_eq!(block["type"], "codeBlock");
        assert_eq!(block["attrs"]["language"], "rust");
        assert_eq!(block["content"][0]["text"], "fn main() {}");
    }

    #[test]
    fn link_marks() {
        let adf = md_to_adf("[arawn](https://example.com)");
        let span = &adf["content"][0]["content"][0];
        assert_eq!(span["text"], "arawn");
        assert_eq!(span["marks"][0]["type"], "link");
        assert_eq!(span["marks"][0]["attrs"]["href"], "https://example.com");
    }
}

//! Unified diff generation for file tool output.
//!
//! Produces standard unified diff format with context lines and hunk headers,
//! collapsing large unchanged regions. Output is suitable for embedding in
//! a fenced ```diff code block for TUI rendering.

use similar::{ChangeTag, TextDiff};

/// Number of context lines to show around each change.
const CONTEXT_LINES: usize = 3;

/// Generate a unified diff between `old` and `new` content for the given file path.
/// Returns a formatted diff string with hunk headers showing line numbers.
/// Unchanged regions between hunks are collapsed.
///
/// Returns `None` if the contents are identical.
pub fn unified_diff(path: &str, old: &str, new: &str) -> Option<String> {
    if old == new {
        return None;
    }

    let diff = TextDiff::from_lines(old, new);
    let mut udiff = diff.unified_diff();
    let formatted = udiff
        .context_radius(CONTEXT_LINES)
        .header(&format!("a/{path}"), &format!("b/{path}"))
        .to_string();

    if formatted.trim().is_empty() {
        None
    } else {
        Some(formatted)
    }
}

/// Format a diff as a fenced markdown code block.
pub fn diff_to_markdown(diff: &str) -> String {
    format!("```diff\n{diff}```")
}

/// Generate a creation diff (all lines added) for a new file.
/// Shows the first `max_lines` of content with line numbers.
pub fn creation_diff(path: &str, content: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let shown = lines.len().min(max_lines);

    let mut out = format!("--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{total} @@\n");
    for line in &lines[..shown] {
        out.push('+');
        out.push_str(line);
        out.push('\n');
    }
    if shown < total {
        out.push_str(&format!("\n... ({} more lines)\n", total - shown));
    }
    out
}

/// Compute a summary line: "N lines added, M lines removed"
pub fn diff_summary(old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut added = 0usize;
    let mut removed = 0usize;

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => added += 1,
            ChangeTag::Delete => removed += 1,
            ChangeTag::Equal => {}
        }
    }

    match (added, removed) {
        (0, 0) => "No changes".to_string(),
        (a, 0) => format!("{a} lines added"),
        (0, r) => format!("{r} lines removed"),
        (a, r) => format!("{a} lines added, {r} lines removed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_returns_none() {
        assert!(unified_diff("foo.rs", "hello\n", "hello\n").is_none());
    }

    #[test]
    fn simple_edit() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nline 2 modified\nline 3\n";
        let diff = unified_diff("foo.rs", old, new).unwrap();
        assert!(diff.contains("--- a/foo.rs"));
        assert!(diff.contains("+++ b/foo.rs"));
        assert!(diff.contains("@@"));
        assert!(diff.contains("-line 2"));
        assert!(diff.contains("+line 2 modified"));
    }

    #[test]
    fn context_collapses_unchanged() {
        let mut old_lines: Vec<String> = (1..=50).map(|i| format!("line {i}")).collect();
        let mut new_lines = old_lines.clone();
        // Change line 5 and line 45
        old_lines[4] = "line 5 old".to_string();
        new_lines[4] = "line 5 new".to_string();
        old_lines[44] = "line 45 old".to_string();
        new_lines[44] = "line 45 new".to_string();

        let old = old_lines.join("\n") + "\n";
        let new = new_lines.join("\n") + "\n";
        let diff = unified_diff("big.rs", &old, &new).unwrap();

        // Should have two hunks (separated by @@)
        let hunk_count = diff.matches("@@").count();
        assert!(hunk_count >= 4, "expected 2+ hunks (4+ @@ markers), got {hunk_count}");

        // Should NOT contain all 50 lines — collapsed
        assert!(
            diff.lines().count() < 30,
            "diff should be collapsed, got {} lines",
            diff.lines().count()
        );
    }

    #[test]
    fn diff_to_markdown_wraps() {
        let diff = "--- a/foo\n+++ b/foo\n@@ -1 +1 @@\n-old\n+new\n";
        let md = diff_to_markdown(diff);
        assert!(md.starts_with("```diff\n"));
        assert!(md.ends_with("```"));
    }

    #[test]
    fn creation_diff_shows_lines() {
        let content = "fn main() {\n    println!(\"hello\");\n}\n";
        let diff = creation_diff("main.rs", content, 100);
        assert!(diff.contains("+++ b/main.rs"));
        assert!(diff.contains("+fn main()"));
    }

    #[test]
    fn creation_diff_truncates() {
        let lines: Vec<String> = (1..=100).map(|i| format!("line {i}")).collect();
        let content = lines.join("\n") + "\n";
        let diff = creation_diff("big.rs", &content, 10);
        assert!(diff.contains("... (90 more lines)"));
    }

    #[test]
    fn summary_counts() {
        let old = "a\nb\nc\n";
        let new = "a\nB\nc\nd\n";
        let s = diff_summary(old, new);
        assert!(s.contains("added"), "got: {s}");
        assert!(s.contains("removed"), "got: {s}");
    }

    #[test]
    fn summary_no_changes() {
        assert_eq!(diff_summary("same\n", "same\n"), "No changes");
    }
}

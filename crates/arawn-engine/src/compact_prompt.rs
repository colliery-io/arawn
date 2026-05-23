//! Compaction prompt templates adapted from Claude Code's proven design.
//! See: claude-code/src/services/compact/prompt.ts

const NO_TOOLS_PREAMBLE: &str = "CRITICAL: Respond with TEXT ONLY. Do NOT call any tools.

- Do NOT use any tools. You already have all the context you need in the conversation above.
- Tool calls will be REJECTED and will waste your only turn — you will fail the task.
- Your entire response must be plain text: an <analysis> block followed by a <summary> block.

";

const ANALYSIS_INSTRUCTION: &str = "Before providing your final summary, wrap your analysis in <analysis> tags to organize your thoughts and ensure you've covered all necessary points. In your analysis process:

1. Chronologically analyze each message and section of the conversation. For each section thoroughly identify:
   - The user's explicit requests and intents
   - Your approach to addressing the user's requests
   - Key decisions, technical concepts and code patterns
   - Specific details like file names, full code snippets, function signatures, file edits
   - Errors encountered and how they were fixed
   - Pay special attention to specific user feedback, especially if the user told you to do something differently.
2. Double-check for technical accuracy and completeness, addressing each required element thoroughly.";

const SUMMARY_TEMPLATE: &str = "Your summary should include the following sections:

1. Primary Request and Intent: Capture all of the user's explicit requests and intents in detail
2. Key Technical Concepts: List all important technical concepts, technologies, and frameworks discussed.
3. Files and Code Sections: Enumerate specific files and code sections examined, modified, or created. Include full code snippets where applicable and a summary of why each file is important.
4. Errors and Fixes: List all errors encountered and how they were fixed. Include user feedback on errors if any.
5. Problem Solving: Document problems solved and any ongoing troubleshooting efforts.
6. All User Messages: List ALL user messages that are not tool results. These are critical for understanding the user's feedback and changing intent.
7. Pending Tasks: Outline any pending tasks that you have explicitly been asked to work on.
8. Current Work: Describe in detail precisely what was being worked on immediately before this summary request. Include file names and code snippets where applicable.
9. Optional Next Step: List the next step related to the most recent work. Include direct quotes from the most recent conversation showing exactly what task you were working on and where you left off.";

const NO_TOOLS_TRAILER: &str = "\n\nREMINDER: Do NOT call any tools. Respond with plain text only — an <analysis> block followed by a <summary> block. Tool calls will be rejected and you will fail the task.";

/// Get the full compaction prompt (summarize entire conversation).
pub fn get_compact_prompt() -> String {
    format!(
        "{NO_TOOLS_PREAMBLE}Your task is to create a detailed summary of the conversation so far, paying close attention to the user's explicit requests and your previous actions. This summary should be thorough in capturing technical details, code patterns, and architectural decisions that would be essential for continuing development work without losing context.

{ANALYSIS_INSTRUCTION}

{SUMMARY_TEMPLATE}

Please provide your summary based on the conversation so far, following this structure and ensuring precision and thoroughness in your response.{NO_TOOLS_TRAILER}"
    )
}

/// Get the partial compaction prompt (summarize only old messages, recent are kept).
pub fn get_partial_compact_prompt() -> String {
    format!(
        "{NO_TOOLS_PREAMBLE}Your task is to create a detailed summary of the RECENT portion of the conversation — the messages that follow earlier retained context. The earlier messages are being kept intact and do NOT need to be summarized. Focus your summary on what was discussed, learned, and accomplished in the recent messages only.

{ANALYSIS_INSTRUCTION}

{SUMMARY_TEMPLATE}

Please provide your summary based on the RECENT messages only (after the retained earlier context), following this structure and ensuring precision and thoroughness in your response.{NO_TOOLS_TRAILER}"
    )
}

/// Strip the `<analysis>` drafting scratchpad and extract `<summary>` content.
pub fn format_compact_summary(raw: &str) -> String {
    let mut result = raw.to_string();

    // Strip analysis section
    if let Some(start) = result.find("<analysis>")
        && let Some(end) = result.find("</analysis>")
    {
        result = format!(
            "{}{}",
            &result[..start],
            &result[end + "</analysis>".len()..]
        );
    }

    // Extract summary content
    if let Some(start) = result.find("<summary>")
        && let Some(end) = result.find("</summary>")
    {
        let content = &result[start + "<summary>".len()..end];
        result = format!("Summary:\n{}", content.trim());
    }

    // Clean up extra whitespace
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    result.trim().to_string()
}

/// Wrap a formatted summary with continuation framing for the LLM.
pub fn get_compact_user_summary_message(summary: &str, recent_preserved: bool) -> String {
    let formatted = format_compact_summary(summary);

    let mut msg = format!(
        "This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\n{formatted}"
    );

    if recent_preserved {
        msg.push_str("\n\nRecent messages are preserved verbatim.");
    }

    msg.push_str("\n\nContinue the conversation from where it left off without asking the user any further questions. Resume directly — do not acknowledge the summary, do not recap what was happening, do not preface with \"I'll continue\" or similar. Pick up the last task as if the break never happened.");

    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_prompt_contains_key_sections() {
        let prompt = get_compact_prompt();
        assert!(prompt.contains("CRITICAL: Respond with TEXT ONLY"));
        assert!(prompt.contains("<analysis>"));
        assert!(prompt.contains("Primary Request and Intent"));
        assert!(prompt.contains("Current Work"));
        assert!(prompt.contains("REMINDER: Do NOT call any tools"));
    }

    #[test]
    fn partial_prompt_mentions_recent() {
        let prompt = get_partial_compact_prompt();
        assert!(prompt.contains("RECENT portion"));
        assert!(prompt.contains("earlier messages are being kept intact"));
    }

    #[test]
    fn format_strips_analysis_extracts_summary() {
        let raw = r#"<analysis>
Some thinking here about the conversation.
</analysis>

<summary>
1. Primary Request and Intent:
   User wants to build a CLI tool.

2. Key Technical Concepts:
   - Rust
   - tokio
</summary>"#;

        let result = format_compact_summary(raw);
        assert!(!result.contains("<analysis>"));
        assert!(!result.contains("</analysis>"));
        assert!(!result.contains("<summary>"));
        assert!(!result.contains("</summary>"));
        assert!(result.contains("Primary Request and Intent"));
        assert!(result.contains("CLI tool"));
        assert!(result.starts_with("Summary:"));
    }

    #[test]
    fn format_handles_no_tags() {
        let raw = "Just plain text summary without XML tags.";
        let result = format_compact_summary(raw);
        assert_eq!(result, raw);
    }

    #[test]
    fn format_handles_analysis_only() {
        let raw = "<analysis>thinking</analysis>\nSome content after.";
        let result = format_compact_summary(raw);
        assert!(!result.contains("thinking"));
        assert!(result.contains("Some content after"));
    }

    #[test]
    fn user_summary_message_has_framing() {
        let summary = "<summary>\n1. Intent: build stuff\n</summary>";
        let msg = get_compact_user_summary_message(summary, true);
        assert!(msg.contains("continued from a previous conversation"));
        assert!(msg.contains("Recent messages are preserved"));
        assert!(msg.contains("Pick up the last task"));
    }
}

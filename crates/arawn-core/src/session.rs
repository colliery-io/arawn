use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::Message;
use crate::session_stats::SessionStats;

/// A conversation session.
/// Scratch sessions start with `workstream_id = None` and
/// `workstream_name = "scratch"`. Once promoted to a workstream,
/// the binding moves to the named workstream's KB.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    workstream_id: Option<Uuid>,
    /// Slug of the workstream this session contributes to. Memory
    /// routing in the engine reads this to pick which KB to write to /
    /// search. Defaults to `scratch`.
    workstream_name: String,
    messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub stats: SessionStats,
}

impl Session {
    /// Create a session bound to a workstream.
    pub fn new(workstream_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            workstream_id: Some(workstream_id),
            workstream_name: crate::workstream::SCRATCH_NAME.to_string(),
            messages: Vec::new(),
            created_at: Utc::now(),
            stats: SessionStats::new(),
        }
    }

    /// Create a session bound to a workstream by name. Use this on
    /// the new-session path so memory routing picks the right KB.
    pub fn new_with_workstream(workstream_id: Uuid, workstream_name: impl Into<String>) -> Self {
        let mut s = Self::new(workstream_id);
        s.workstream_name = workstream_name.into();
        s
    }

    /// Reconstruct a session from persisted parts (DB load path).
    pub fn from_parts(
        id: Uuid,
        workstream_id: Option<Uuid>,
        created_at: DateTime<Utc>,
        messages: Vec<Message>,
    ) -> Self {
        Self {
            id,
            workstream_id,
            workstream_name: crate::workstream::SCRATCH_NAME.to_string(),
            messages,
            created_at,
            stats: SessionStats::new(),
        }
    }

    /// Reconstruct a session with stats from persisted parts.
    pub fn from_parts_with_stats(
        id: Uuid,
        workstream_id: Option<Uuid>,
        created_at: DateTime<Utc>,
        messages: Vec<Message>,
        stats: SessionStats,
    ) -> Self {
        Self {
            id,
            workstream_id,
            workstream_name: crate::workstream::SCRATCH_NAME.to_string(),
            messages,
            created_at,
            stats,
        }
    }

    /// Create a scratch session (no workstream binding yet).
    pub fn scratch() -> Self {
        Self {
            id: Uuid::new_v4(),
            workstream_id: None,
            workstream_name: crate::workstream::SCRATCH_NAME.to_string(),
            messages: Vec::new(),
            created_at: Utc::now(),
            stats: SessionStats::new(),
        }
    }

    pub fn workstream_id(&self) -> Option<Uuid> {
        self.workstream_id
    }

    /// Current workstream slug for this session. Memory tools read
    /// this to pick which KB to write to / search.
    pub fn workstream_name(&self) -> &str {
        &self.workstream_name
    }

    /// Update the active workstream binding. Both the slug (used for
    /// KB routing) and the Uuid (used for session-table FK) update
    /// atomically. Called by `/workstream switch`.
    pub fn set_workstream(&mut self, name: impl Into<String>, id: Uuid) {
        self.workstream_name = name.into();
        self.workstream_id = Some(id);
    }

    /// Returns true if this is a scratch session (not yet promoted).
    pub fn is_scratch(&self) -> bool {
        self.workstream_id.is_none()
    }

    /// Promote a scratch session to a workstream. Panics if already bound.
    pub fn promote(&mut self, workstream_id: Uuid) {
        assert!(
            self.workstream_id.is_none(),
            "cannot promote: session is already bound to workstream {:?}",
            self.workstream_id.unwrap()
        );
        self.workstream_id = Some(workstream_id);
    }

    pub fn add_message(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Clear old tool results to save context space without an LLM call.
    /// Replaces content of ToolResult messages older than `keep_recent` turns
    /// for large-output tools (shell, file_read, grep, glob, web_fetch, web_search).
    /// Returns total characters cleared.
    pub fn microcompact(&mut self, keep_recent: usize) -> usize {
        const TARGETED_TOOLS: &[&str] = &[
            "shell", "Bash", "file_read", "Read", "FileRead",
            "grep", "Grep", "glob", "Glob",
            "web_fetch", "WebFetch", "web_search", "WebSearch",
            "file_write", "Write", "FileWrite",
            "file_edit", "Edit", "FileEdit",
        ];
        const STUB_THRESHOLD: usize = 100; // Don't clear results shorter than this

        if self.messages.len() <= keep_recent {
            return 0;
        }

        let cutoff = self.messages.len() - keep_recent;
        let mut chars_cleared = 0;

        // Build a map of tool_use_id → tool_name from Assistant messages
        let tool_names: std::collections::HashMap<String, String> = self
            .messages
            .iter()
            .filter_map(|msg| {
                if let Message::Assistant { tool_uses, .. } = msg {
                    Some(tool_uses.iter().map(|tu| (tu.id.clone(), tu.name.clone())))
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        for i in 0..cutoff {
            if let Message::ToolResult {
                tool_use_id,
                content,
                is_error,
            } = &self.messages[i]
            {
                // Skip small results and errors
                if content.len() < STUB_THRESHOLD || *is_error {
                    continue;
                }

                // Check if this tool is targeted
                let tool_name = tool_names
                    .get(tool_use_id)
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");

                if TARGETED_TOOLS.contains(&tool_name) {
                    let original_len = content.len();
                    chars_cleared += original_len;
                    self.messages[i] = Message::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: format!(
                            "[Previous {tool_name} result cleared — {original_len} chars]"
                        ),
                        is_error: *is_error,
                    };
                }
            }
        }

        chars_cleared
    }

    /// Replace old messages with a Summary, keeping the last `keep_recent` messages verbatim.
    /// Returns the number of messages that were summarized.
    pub fn compact(&mut self, summary_content: String, keep_recent: usize) -> usize {
        if self.messages.len() <= keep_recent {
            return 0;
        }

        let split_point = self.messages.len() - keep_recent;
        let summarized_count = split_point;
        let estimated_saved = self.messages[..split_point]
            .iter()
            .map(|m| match m {
                Message::User { content } => content.len() as u32 / 4,
                Message::Assistant { content, tool_uses } => {
                    (content.len() as u32 / 4)
                        + tool_uses
                            .iter()
                            .map(|t| t.input.to_string().len() as u32 / 4)
                            .sum::<u32>()
                }
                Message::ToolResult { content, .. } => content.len() as u32 / 4,
                Message::Summary { content, .. } => content.len() as u32 / 4,
            })
            .sum::<u32>();

        let recent: Vec<Message> = self.messages.drain(split_point..).collect();
        self.messages.clear();
        self.messages.push(Message::Summary {
            content: summary_content,
            original_count: summarized_count,
            estimated_tokens_saved: estimated_saved,
        });
        self.messages.extend(recent);

        summarized_count
    }

    /// Load messages with compaction awareness — if a Summary exists, use the
    /// last Summary as the starting point and only include messages after it.
    pub fn load_compacted(messages: Vec<Message>) -> Vec<Message> {
        // Find the index of the last Summary message
        let last_summary_idx = messages
            .iter()
            .rposition(|m| matches!(m, Message::Summary { .. }));

        match last_summary_idx {
            Some(idx) => messages[idx..].to_vec(),
            None => messages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ToolUse;
    use uuid::Uuid;

    #[test]
    fn session_bound_to_workstream() {
        let ws_id = Uuid::new_v4();
        let session = Session::new(ws_id);
        assert_eq!(session.workstream_id(), Some(ws_id));
        assert!(!session.is_scratch());
    }

    #[test]
    fn scratch_session_has_no_workstream() {
        let session = Session::scratch();
        assert!(session.is_scratch());
        assert_eq!(session.workstream_id(), None);
    }

    #[test]
    fn promote_scratch_session() {
        let mut session = Session::scratch();
        let ws_id = Uuid::new_v4();
        session.promote(ws_id);
        assert_eq!(session.workstream_id(), Some(ws_id));
        assert!(!session.is_scratch());
    }

    #[test]
    #[should_panic(expected = "cannot promote")]
    fn promote_already_bound_panics() {
        let mut session = Session::new(Uuid::new_v4());
        session.promote(Uuid::new_v4());
    }

    #[test]
    fn session_starts_with_no_messages() {
        let session = Session::new(Uuid::new_v4());
        assert!(session.messages().is_empty());
    }

    #[test]
    fn session_message_ordering_preserved() {
        let mut session = Session::new(Uuid::new_v4());
        session.add_message(Message::User {
            content: "first".into(),
        });
        session.add_message(Message::Assistant {
            content: "second".into(),
            tool_uses: vec![],
        });
        session.add_message(Message::User {
            content: "third".into(),
        });

        let msgs = session.messages();
        assert_eq!(msgs.len(), 3);
        match &msgs[0] {
            Message::User { content } => assert_eq!(content, "first"),
            _ => panic!("expected User"),
        }
        match &msgs[1] {
            Message::Assistant { content, .. } => assert_eq!(content, "second"),
            _ => panic!("expected Assistant"),
        }
        match &msgs[2] {
            Message::User { content } => assert_eq!(content, "third"),
            _ => panic!("expected User"),
        }
    }

    #[test]
    fn session_ids_are_unique() {
        let ws_id = Uuid::new_v4();
        let s1 = Session::new(ws_id);
        let s2 = Session::new(ws_id);
        assert_ne!(s1.id, s2.id);
    }

    #[test]
    fn compact_replaces_old_with_summary() {
        let mut session = Session::new(Uuid::new_v4());
        for i in 0..10 {
            session.add_message(Message::User {
                content: format!("message {i}"),
            });
        }

        let summarized = session.compact("Summary of first 7 messages".into(), 3);
        assert_eq!(summarized, 7);
        assert_eq!(session.messages().len(), 4); // 1 summary + 3 recent

        match &session.messages()[0] {
            Message::Summary {
                content,
                original_count,
                ..
            } => {
                assert!(content.contains("Summary of first 7"));
                assert_eq!(*original_count, 7);
            }
            _ => panic!("expected Summary"),
        }

        // Last 3 messages preserved
        match &session.messages()[3] {
            Message::User { content } => assert_eq!(content, "message 9"),
            _ => panic!("expected User"),
        }
    }

    #[test]
    fn compact_too_few_messages_noop() {
        let mut session = Session::new(Uuid::new_v4());
        session.add_message(Message::User {
            content: "a".into(),
        });
        session.add_message(Message::User {
            content: "b".into(),
        });

        let summarized = session.compact("summary".into(), 5);
        assert_eq!(summarized, 0);
        assert_eq!(session.messages().len(), 2); // unchanged
    }

    #[test]
    fn load_compacted_skips_before_summary() {
        let messages = vec![
            Message::User {
                content: "old 1".into(),
            },
            Message::User {
                content: "old 2".into(),
            },
            Message::Summary {
                content: "summary of old messages".into(),
                original_count: 2,
                estimated_tokens_saved: 100,
            },
            Message::User {
                content: "new 1".into(),
            },
            Message::Assistant {
                content: "new 2".into(),
                tool_uses: vec![],
            },
        ];

        let compacted = Session::load_compacted(messages);
        assert_eq!(compacted.len(), 3); // summary + 2 new
        assert!(matches!(&compacted[0], Message::Summary { .. }));
    }

    #[test]
    fn load_compacted_no_summary_returns_all() {
        let messages = vec![
            Message::User {
                content: "a".into(),
            },
            Message::User {
                content: "b".into(),
            },
        ];

        let compacted = Session::load_compacted(messages.clone());
        assert_eq!(compacted.len(), 2);
    }

    #[test]
    fn microcompact_clears_old_tool_results() {
        let mut session = Session::scratch();
        // Turn 1: user + assistant(tool) + tool_result
        session.add_message(Message::User { content: "find files".into() });
        session.add_message(Message::Assistant {
            content: "".into(),
            tool_uses: vec![ToolUse {
                id: "c1".into(),
                name: "shell".into(),
                input: serde_json::json!({"command": "find . -name '*.rs'"}),
            }],
        });
        session.add_message(Message::ToolResult {
            tool_use_id: "c1".into(),
            content: "a".repeat(500), // large result
            is_error: false,
        });
        // Turn 2: assistant response
        session.add_message(Message::Assistant {
            content: "Here are the files.".into(),
            tool_uses: vec![],
        });
        // Several more turns to push turn 1 past the cutoff
        for i in 0..8 {
            session.add_message(Message::User { content: format!("q{i}") });
            session.add_message(Message::Assistant {
                content: format!("a{i}"),
                tool_uses: vec![],
            });
        }

        let chars_cleared = session.microcompact(6);
        assert!(chars_cleared >= 500, "should clear the large shell result, cleared={chars_cleared}");

        // The old tool result should be a stub
        if let Message::ToolResult { content, .. } = &session.messages()[2] {
            assert!(content.contains("[Previous shell result cleared"), "got: {content}");
        } else {
            panic!("expected ToolResult at index 2");
        }
    }

    #[test]
    fn microcompact_preserves_recent_results() {
        let mut session = Session::scratch();
        session.add_message(Message::User { content: "read file".into() });
        session.add_message(Message::Assistant {
            content: "".into(),
            tool_uses: vec![ToolUse {
                id: "c1".into(),
                name: "file_read".into(),
                input: serde_json::json!({"path": "foo.rs"}),
            }],
        });
        session.add_message(Message::ToolResult {
            tool_use_id: "c1".into(),
            content: "x".repeat(500),
            is_error: false,
        });
        // Only 3 messages — within keep_recent=6
        let chars_cleared = session.microcompact(6);
        assert_eq!(chars_cleared, 0, "should not clear recent results");
    }

    #[test]
    fn microcompact_skips_small_results() {
        let mut session = Session::scratch();
        for i in 0..10 {
            session.add_message(Message::User { content: format!("q{i}") });
            session.add_message(Message::Assistant {
                content: "".into(),
                tool_uses: vec![ToolUse {
                    id: format!("c{i}"),
                    name: "shell".into(),
                    input: serde_json::json!({"command": "echo hi"}),
                }],
            });
            session.add_message(Message::ToolResult {
                tool_use_id: format!("c{i}"),
                content: "hi".into(), // small result
                is_error: false,
            });
        }

        let chars_cleared = session.microcompact(6);
        assert_eq!(chars_cleared, 0, "should not clear small results");
    }

    #[test]
    fn microcompact_skips_errors() {
        let mut session = Session::scratch();
        for i in 0..10 {
            session.add_message(Message::User { content: format!("q{i}") });
            session.add_message(Message::Assistant {
                content: "".into(),
                tool_uses: vec![ToolUse {
                    id: format!("c{i}"),
                    name: "shell".into(),
                    input: serde_json::json!({}),
                }],
            });
            session.add_message(Message::ToolResult {
                tool_use_id: format!("c{i}"),
                content: "x".repeat(500),
                is_error: true, // error — should not be cleared
            });
        }

        let chars_cleared = session.microcompact(6);
        assert_eq!(chars_cleared, 0, "should not clear error results");
    }

    #[test]
    fn microcompact_skips_non_targeted_tools() {
        let mut session = Session::scratch();
        for i in 0..10 {
            session.add_message(Message::User { content: format!("q{i}") });
            session.add_message(Message::Assistant {
                content: "".into(),
                tool_uses: vec![ToolUse {
                    id: format!("c{i}"),
                    name: "think".into(), // not targeted
                    input: serde_json::json!({}),
                }],
            });
            session.add_message(Message::ToolResult {
                tool_use_id: format!("c{i}"),
                content: "x".repeat(500),
                is_error: false,
            });
        }

        let chars_cleared = session.microcompact(6);
        assert_eq!(chars_cleared, 0, "should not clear think results");
    }
}

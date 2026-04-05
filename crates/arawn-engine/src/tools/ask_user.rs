use async_trait::async_trait;
use serde_json::{Value, json};

use crate::context::ToolContext;
use crate::error::EngineError;
use crate::tool::{Tool, ToolOutput};

/// Asks the user structured multiple-choice questions to gather requirements
/// or clarify ambiguity. Returns the user's selected answers.
///
/// In non-interactive contexts (e.g. one-shot CLI), returns the questions
/// as formatted text for the user to answer in their next message.
pub struct AskUserTool;

#[async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn description(&self) -> &str {
        "Ask the user 1-4 multiple-choice questions to gather requirements or clarify ambiguity. \
         Each question has a header, question text, and a list of options. \
         Set multiSelect to true if the user can pick more than one option.\n\n\
         Usage notes:\n\
         - Use this when you need the user to make a concrete choice before proceeding\n\
         - If you recommend a specific option, make that the first option and add \"(Recommended)\" at the end of the label\n\
         - Use multiSelect: true to allow multiple answers\n\
         - Don't guess when you can ask"
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 4,
                    "items": {
                        "type": "object",
                        "properties": {
                            "header": {
                                "type": "string",
                                "description": "Short header/title for the question"
                            },
                            "question": {
                                "type": "string",
                                "description": "The question text to display"
                            },
                            "options": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {
                                            "type": "string",
                                            "description": "The option text"
                                        }
                                    },
                                    "required": ["label"]
                                },
                                "description": "Available choices"
                            },
                            "multiSelect": {
                                "type": "boolean",
                                "description": "Whether the user can select multiple options (default: false)"
                            }
                        },
                        "required": ["header", "question", "options"]
                    },
                    "description": "1-4 questions to ask the user"
                }
            },
            "required": ["questions"]
        })
    }

    async fn execute(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        let questions = params
            .get("questions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| EngineError::Tool("missing 'questions' parameter".into()))?;

        if questions.is_empty() {
            return Err(EngineError::Tool(
                "at least one question is required".into(),
            ));
        }

        // Format questions as structured text for the user.
        // In a TUI/WebSocket context, the client renders these as interactive widgets.
        // In CLI mode, they appear as formatted text the user responds to.
        let mut output = String::new();

        for (i, q) in questions.iter().enumerate() {
            let header = q
                .get("header")
                .and_then(|v| v.as_str())
                .unwrap_or("Question");
            let question = q.get("question").and_then(|v| v.as_str()).unwrap_or("");
            let multi = q
                .get("multiSelect")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let options = q
                .get("options")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if i > 0 {
                output.push('\n');
            }

            output.push_str(&format!("### {header}\n"));
            output.push_str(&format!("{question}\n"));
            if multi {
                output.push_str("(select all that apply)\n");
            }
            for (j, opt) in options.iter().enumerate() {
                let label = opt.get("label").and_then(|v| v.as_str()).unwrap_or("?");
                output.push_str(&format!("  {}. {label}\n", j + 1));
            }
        }

        // Return the formatted questions as a tool result.
        // The engine should surface this to the user and collect their response.
        Ok(ToolOutput::success(output.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Workstream;
    use serde_json::json;
    use uuid::Uuid;

    fn test_ctx() -> ToolContext {
        let ws = Workstream::scratch("/tmp/test");
        ToolContext::new(&ws, Uuid::new_v4())
    }

    #[test]
    fn schema_is_valid() {
        let tool = AskUserTool;
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["questions"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("questions")));
    }

    #[test]
    fn is_read_only() {
        assert!(AskUserTool.is_read_only());
    }

    #[tokio::test]
    async fn single_question() {
        let ctx = test_ctx();
        let result = AskUserTool
            .execute(
                &ctx,
                json!({
                    "questions": [{
                        "header": "Framework",
                        "question": "Which framework?",
                        "options": [
                            {"label": "React"},
                            {"label": "Vue"}
                        ]
                    }]
                }),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("### Framework"));
        assert!(result.content.contains("Which framework?"));
        assert!(result.content.contains("1. React"));
        assert!(result.content.contains("2. Vue"));
    }

    #[tokio::test]
    async fn multi_select_shows_hint() {
        let ctx = test_ctx();
        let result = AskUserTool
            .execute(
                &ctx,
                json!({
                    "questions": [{
                        "header": "Testing",
                        "question": "Which libraries?",
                        "options": [
                            {"label": "Jest"},
                            {"label": "Vitest"}
                        ],
                        "multiSelect": true
                    }]
                }),
            )
            .await
            .unwrap();

        assert!(result.content.contains("select all that apply"));
    }

    #[tokio::test]
    async fn multiple_questions() {
        let ctx = test_ctx();
        let result = AskUserTool
            .execute(
                &ctx,
                json!({
                    "questions": [
                        {
                            "header": "Language",
                            "question": "Which language?",
                            "options": [{"label": "Rust"}, {"label": "Go"}]
                        },
                        {
                            "header": "DB",
                            "question": "Which database?",
                            "options": [{"label": "Postgres"}, {"label": "SQLite"}]
                        }
                    ]
                }),
            )
            .await
            .unwrap();

        assert!(result.content.contains("### Language"));
        assert!(result.content.contains("### DB"));
    }

    #[tokio::test]
    async fn empty_questions_errors() {
        let ctx = test_ctx();
        let result = AskUserTool.execute(&ctx, json!({"questions": []})).await;

        assert!(result.is_err());
    }
}

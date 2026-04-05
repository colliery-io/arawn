use std::path::Path;

use tracing::{debug, info};
use uuid::Uuid;

use crate::tool::ToolOutput;

/// Default maximum characters per tool result before persisting to disk.
pub const DEFAULT_MAX_RESULT_SIZE_CHARS: usize = 50_000;

/// Truncation preview size — how much of the original to keep inline.
const PREVIEW_SIZE: usize = 2_000;

/// Check if a tool output exceeds the size threshold. If so, persist the full
/// output to disk and replace with a truncated preview + file pointer.
///
/// Returns the (possibly modified) ToolOutput.
pub async fn limit_tool_result(
    output: ToolOutput,
    session_id: Uuid,
    data_dir: &Path,
    max_chars: usize,
) -> ToolOutput {
    if output.content.len() <= max_chars {
        return output;
    }

    // Persist full output to disk
    let result_id = Uuid::new_v4();
    let result_dir = data_dir
        .join("sessions")
        .join(session_id.to_string())
        .join("tool-results");

    if let Err(e) = tokio::fs::create_dir_all(&result_dir).await {
        debug!(error = %e, "failed to create tool-results dir, returning truncated");
        return truncate_output(output, max_chars, None);
    }

    let result_path = result_dir.join(format!("{result_id}.txt"));

    match tokio::fs::write(&result_path, &output.content).await {
        Ok(()) => {
            let original_len = output.content.len();
            info!(
                original_len,
                path = %result_path.display(),
                "persisted large tool result to disk"
            );
            truncate_output(output, max_chars, Some(&result_path))
        }
        Err(e) => {
            debug!(error = %e, "failed to persist tool result, returning truncated");
            truncate_output(output, max_chars, None)
        }
    }
}

fn truncate_output(
    output: ToolOutput,
    _max_chars: usize,
    persisted_path: Option<&Path>,
) -> ToolOutput {
    let preview = &output.content[..PREVIEW_SIZE.min(output.content.len())];
    let original_len = output.content.len();

    let mut content = String::with_capacity(PREVIEW_SIZE + 200);
    content.push_str(preview);
    content.push_str("\n\n... (");
    content.push_str(&format!("{original_len} chars total, truncated)"));

    if let Some(path) = persisted_path {
        content.push_str(&format!(
            "\n\nFull output saved to: {}\nUse file_read to access the complete output.",
            path.display()
        ));
    }

    ToolOutput {
        content,
        is_error: output.is_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn small_output_passes_through() {
        let output = ToolOutput::success("small result");
        let session_id = Uuid::new_v4();
        let tmp = TempDir::new().unwrap();

        let result = limit_tool_result(
            output,
            session_id,
            tmp.path(),
            DEFAULT_MAX_RESULT_SIZE_CHARS,
        )
        .await;
        assert_eq!(result.content, "small result");
        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn large_output_gets_truncated_and_persisted() {
        let big_content = "x".repeat(100_000);
        let output = ToolOutput::success(big_content.clone());
        let session_id = Uuid::new_v4();
        let tmp = TempDir::new().unwrap();

        let result = limit_tool_result(output, session_id, tmp.path(), 50_000).await;

        // Result should be truncated
        assert!(
            result.content.len() < 10_000,
            "should be much smaller than original"
        );
        assert!(result.content.contains("truncated"));
        assert!(result.content.contains("file_read"));
        assert!(result.content.contains("tool-results"));

        // File should exist on disk with full content
        let results_dir = tmp
            .path()
            .join("sessions")
            .join(session_id.to_string())
            .join("tool-results");
        let files: Vec<_> = std::fs::read_dir(&results_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(files.len(), 1);

        let persisted = std::fs::read_to_string(files[0].path()).unwrap();
        assert_eq!(persisted.len(), 100_000);
    }

    #[tokio::test]
    async fn truncated_output_contains_preview() {
        let big_content = format!("IMPORTANT_HEADER\n{}", "data\n".repeat(20_000));
        let output = ToolOutput::success(big_content);
        let session_id = Uuid::new_v4();
        let tmp = TempDir::new().unwrap();

        let result = limit_tool_result(output, session_id, tmp.path(), 50_000).await;

        // Preview should contain the beginning of the content
        assert!(result.content.contains("IMPORTANT_HEADER"));
    }

    #[tokio::test]
    async fn error_flag_preserved() {
        let big_content = "e".repeat(100_000);
        let output = ToolOutput::error(big_content);
        let session_id = Uuid::new_v4();
        let tmp = TempDir::new().unwrap();

        let result = limit_tool_result(output, session_id, tmp.path(), 50_000).await;
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn custom_threshold() {
        let content = "x".repeat(5_000);
        let output = ToolOutput::success(content);
        let session_id = Uuid::new_v4();
        let tmp = TempDir::new().unwrap();

        // With threshold of 1000, this should be truncated
        let result = limit_tool_result(output, session_id, tmp.path(), 1_000).await;
        assert!(result.content.contains("truncated"));
    }
}

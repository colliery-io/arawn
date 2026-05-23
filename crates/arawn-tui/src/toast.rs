//! I-0035 Phase 4 T-A — ephemeral 1-line toast surface.
//!
//! Toasts are short, informational messages that appear above the
//! status bar and fade after a TTL. Any `ServerNotice` handler can
//! enqueue one via [`App::post_toast`]; the renderer pops expired
//! toasts lazily on each render pass.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Default time-to-live for a toast (5 seconds — enough to read a
/// one-liner without being noisy).
pub const TOAST_TTL: Duration = Duration::from_secs(5);

/// Maximum queue depth. When full, the oldest is dropped — a runaway
/// producer can't OOM the TUI.
pub const TOAST_QUEUE_CAP: usize = 8;

/// Severity used to route a toast to a theme color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub posted_at: Instant,
    pub ttl: Duration,
}

impl Toast {
    pub fn new(message: impl Into<String>, level: ToastLevel) -> Self {
        Self {
            message: message.into(),
            level,
            posted_at: Instant::now(),
            ttl: TOAST_TTL,
        }
    }

    pub fn is_expired(&self, now: Instant) -> bool {
        now.duration_since(self.posted_at) >= self.ttl
    }
}

/// Drop every toast at the front of the queue whose TTL has elapsed.
/// Returns the queue mutated in place. Render code calls this before
/// peeking the head so an expired toast never stays visible past its
/// TTL even if no other state changed.
pub fn drop_expired(queue: &mut VecDeque<Toast>, now: Instant) {
    while let Some(front) = queue.front() {
        if front.is_expired(now) {
            queue.pop_front();
        } else {
            break;
        }
    }
}

/// Append a toast, capping at [`TOAST_QUEUE_CAP`]. Oldest is dropped
/// when full so a sustained producer can't grow the queue without
/// bound.
pub fn enqueue(queue: &mut VecDeque<Toast>, toast: Toast) {
    if queue.len() >= TOAST_QUEUE_CAP {
        queue.pop_front();
    }
    queue.push_back(toast);
}

/// T-0361 — write `text` to the system clipboard via the OSC 52
/// escape sequence. Zero deps; works in most modern terminals
/// (iTerm2, kitty, Alacritty, wezterm, recent xterm, tmux ≥ 3.3).
/// Older terminals silently no-op.
///
/// Format: `ESC ] 52 ; c ; <base64-of-text> BEL`. Some terminals
/// cap the payload size; we don't try to handle that — copying a
/// 100KB response is degenerate anyway.
pub fn write_osc52_clipboard(text: &str) {
    use std::io::Write;
    let encoded = base64_encode(text.as_bytes());
    let seq = format!("\x1b]52;c;{encoded}\x07");
    // stderr (not stdout) so the sequence reaches the terminal even
    // when stdout is being captured by ratatui's renderer. Best-
    // effort: the OS-level write itself can fail (closed terminal,
    // pipe to /dev/null) and the toast wording stays the same.
    let _ = std::io::stderr().write_all(seq.as_bytes());
}

/// Minimal RFC 4648 base64 encoder. Avoids pulling a crate dep
/// for one short string per `/copy` invocation. Pure, no padding
/// shortcuts skipped.
fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n =
            ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
        out.push(CHARS[(n & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        out.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        out.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_expired_after_ttl() {
        let mut t = Toast::new("hi", ToastLevel::Info);
        t.posted_at = Instant::now() - Duration::from_secs(10);
        t.ttl = Duration::from_secs(5);
        assert!(t.is_expired(Instant::now()));
    }

    #[test]
    fn is_not_expired_within_ttl() {
        let t = Toast::new("hi", ToastLevel::Info);
        assert!(!t.is_expired(Instant::now()));
    }

    #[test]
    fn drop_expired_pops_front_only() {
        let mut q: VecDeque<Toast> = VecDeque::new();
        let mut old = Toast::new("old", ToastLevel::Info);
        old.posted_at = Instant::now() - Duration::from_secs(10);
        q.push_back(old);
        let fresh = Toast::new("fresh", ToastLevel::Info);
        q.push_back(fresh);
        drop_expired(&mut q, Instant::now());
        assert_eq!(q.len(), 1);
        assert_eq!(q.front().unwrap().message, "fresh");
    }

    // T-0361 — base64 encoder.

    #[test]
    fn base64_encodes_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn base64_encodes_single_byte() {
        // "f" → "Zg==" (RFC 4648 §10 reference).
        assert_eq!(base64_encode(b"f"), "Zg==");
    }

    #[test]
    fn base64_encodes_two_bytes() {
        assert_eq!(base64_encode(b"fo"), "Zm8=");
    }

    #[test]
    fn base64_encodes_three_bytes() {
        assert_eq!(base64_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn base64_encodes_eight_bytes() {
        // "foobar" → "Zm9vYmFy".
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn enqueue_caps_at_max() {
        let mut q: VecDeque<Toast> = VecDeque::new();
        for i in 0..(TOAST_QUEUE_CAP + 3) {
            enqueue(&mut q, Toast::new(format!("m{i}"), ToastLevel::Info));
        }
        assert_eq!(q.len(), TOAST_QUEUE_CAP);
        // First three were dropped — front is m3.
        assert_eq!(q.front().unwrap().message, "m3");
    }
}

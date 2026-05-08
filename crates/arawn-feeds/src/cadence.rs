//! Cadence (cron expression) validation.
//!
//! I-0039 locks a 15-minute minimum cadence — no per-feed override.
//! Real-time / sub-minute needs go through a different surface
//! (webhooks / push, future initiative). 15 minutes is the smallest
//! interval that's also polite to providers' rate limits.

use std::time::Duration;

use cloacina::CronEvaluator;

use crate::error::FeedError;

/// Minimum allowed cadence. 15 minutes per I-0039 design lock.
pub const MIN_CADENCE: Duration = Duration::from_secs(15 * 60);

/// Validate a cron expression in UTC and reject anything whose minimum
/// fire-interval is below [`MIN_CADENCE`]. Computes the gap between the
/// next 5 fire-times and rejects if any pair is too close.
pub fn validate_cadence(cron_expr: &str) -> Result<(), FeedError> {
    // First pass: cron syntax + timezone validity. Lets us return a
    // useful error without computing fire-times if the expression is
    // malformed.
    CronEvaluator::validate(cron_expr, "UTC")
        .map_err(|e| FeedError::InvalidParams(format!("invalid cron expression: {e}")))?;

    let evaluator = CronEvaluator::new(cron_expr, "UTC")
        .map_err(|e| FeedError::InvalidParams(format!("invalid cron expression: {e}")))?;

    let mut t = chrono::Utc::now();
    let mut prev = None;
    for _ in 0..5 {
        let next = evaluator
            .next_execution(t)
            .map_err(|e| FeedError::InvalidParams(format!("cron next execution: {e}")))?;
        if let Some(p) = prev {
            let gap: chrono::Duration = next - p;
            let gap = gap.to_std().unwrap_or(Duration::ZERO);
            if gap < MIN_CADENCE {
                return Err(FeedError::InvalidParams(format!(
                    "cadence '{cron_expr}' fires every {}s; minimum allowed is {}s ({} minutes)",
                    gap.as_secs(),
                    MIN_CADENCE.as_secs(),
                    MIN_CADENCE.as_secs() / 60,
                )));
            }
        }
        prev = Some(next);
        t = next;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifteen_minute_cadence_is_accepted() {
        // every 15 minutes
        validate_cadence("*/15 * * * *").expect("15min should pass");
        // every hour
        validate_cadence("0 * * * *").expect("hourly should pass");
        // daily
        validate_cadence("0 6 * * *").expect("daily should pass");
    }

    #[test]
    fn sub_fifteen_minute_cadence_is_rejected() {
        // every minute
        let err = validate_cadence("* * * * *").unwrap_err();
        assert!(matches!(err, FeedError::InvalidParams(_)));
        // every 5 minutes
        let err = validate_cadence("*/5 * * * *").unwrap_err();
        assert!(matches!(err, FeedError::InvalidParams(_)));
        // every 10 minutes
        let err = validate_cadence("*/10 * * * *").unwrap_err();
        assert!(matches!(err, FeedError::InvalidParams(_)));
    }

    #[test]
    fn malformed_cron_is_rejected() {
        let err = validate_cadence("not a cron").unwrap_err();
        assert!(matches!(err, FeedError::InvalidParams(_)));
    }
}

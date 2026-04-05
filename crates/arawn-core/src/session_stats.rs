use serde::{Deserialize, Serialize};

/// Accumulated token usage and activity stats for a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SessionStats {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub turns: u32,
    pub tool_calls: u32,
}

impl SessionStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record usage from a single LLM call.
    pub fn record_turn(&mut self, input_tokens: u32, output_tokens: u32, tool_call_count: u32) {
        self.input_tokens += input_tokens as u64;
        self.output_tokens += output_tokens as u64;
        self.turns += 1;
        self.tool_calls += tool_call_count;
    }

    /// Total tokens (input + output).
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Estimate cost in USD given per-1k-token rates.
    pub fn estimated_cost_usd(&self, cost_per_1k_input: f64, cost_per_1k_output: f64) -> f64 {
        let input_cost = (self.input_tokens as f64 / 1000.0) * cost_per_1k_input;
        let output_cost = (self.output_tokens as f64 / 1000.0) * cost_per_1k_output;
        input_cost + output_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_stats_are_zero() {
        let stats = SessionStats::new();
        assert_eq!(stats.input_tokens, 0);
        assert_eq!(stats.output_tokens, 0);
        assert_eq!(stats.turns, 0);
        assert_eq!(stats.tool_calls, 0);
    }

    #[test]
    fn record_turn_accumulates() {
        let mut stats = SessionStats::new();
        stats.record_turn(100, 50, 2);
        stats.record_turn(200, 100, 1);

        assert_eq!(stats.input_tokens, 300);
        assert_eq!(stats.output_tokens, 150);
        assert_eq!(stats.turns, 2);
        assert_eq!(stats.tool_calls, 3);
        assert_eq!(stats.total_tokens(), 450);
    }

    #[test]
    fn cost_calculation() {
        let mut stats = SessionStats::new();
        stats.input_tokens = 10_000;
        stats.output_tokens = 5_000;

        // $0.01 per 1k input, $0.03 per 1k output
        let cost = stats.estimated_cost_usd(0.01, 0.03);
        // 10 * 0.01 + 5 * 0.03 = 0.10 + 0.15 = 0.25
        assert!((cost - 0.25).abs() < 0.001);
    }

    #[test]
    fn zero_rates_zero_cost() {
        let mut stats = SessionStats::new();
        stats.record_turn(1000, 500, 0);
        assert_eq!(stats.estimated_cost_usd(0.0, 0.0), 0.0);
    }
}

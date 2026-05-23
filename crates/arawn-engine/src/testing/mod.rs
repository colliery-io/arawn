//! Test harness for end-to-end engine scenarios.
//!
//! Public API:
//! - [`TestHarness`] — the ready-to-run harness; call `harness.run(input)` to
//!   exercise the full `QueryEngine` loop against a scripted mock LLM.
//! - [`TestHarnessBuilder`] — fluent builder; start with `TestHarness::builder()`.
//! - [`HarnessResult`] — the outcome of a run; exposes the final assistant
//!   text, the message history, captured tool calls, and helpers for
//!   assertions.
//!
//! The harness is consumed from the `arawn-tests` integration-test crate
//! (full_pipeline, hooks, memory_tools, permissions, skills, hot_reload,
//! workflows). Internals are split across this module's siblings:
//! `result.rs`, `builder.rs`, `harness.rs`.

mod builder;
mod harness;
mod result;

pub use builder::TestHarnessBuilder;
pub use harness::TestHarness;
pub use result::HarnessResult;

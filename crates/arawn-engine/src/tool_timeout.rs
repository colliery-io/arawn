//! Wall-clock timeout resolution for tool calls.
//!
//! Every tool execution in the engine is wrapped in a `tokio::time::timeout`
//! whose duration is resolved here. Precedence (highest wins):
//!
//! 1. The agent's per-call override — a `timeout_secs` field in the tool's
//!    JSON arguments. Extracted by [`extract_override`] before the args
//!    reach the tool implementation.
//! 2. `ARAWN_TOOL_TIMEOUT_SECS` environment variable.
//! 3. The engine config's `tool_timeout_secs` field.
//! 4. [`DEFAULT_TIMEOUT_SECS`] (120).
//!
//! There is **no hard ceiling**. The agent has more context than a static
//! config about how long a given tool call should take; trust it.

use std::time::Duration;

use serde_json::Value;

/// The name of the JSON field the agent uses to set a per-call timeout.
pub const TIMEOUT_PARAM: &str = "timeout_secs";

/// Default timeout when nothing else is configured: 120 seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Environment variable that overrides the default timeout for the whole process.
pub const TIMEOUT_ENV_VAR: &str = "ARAWN_TOOL_TIMEOUT_SECS";

/// Resolve the default timeout for a tool call when the agent did not pass an
/// override. Precedence: env var > config > [`DEFAULT_TIMEOUT_SECS`].
///
/// `config_secs` is the value read from `[engine] tool_timeout_secs` in
/// `arawn.toml`, if any.
pub fn default_timeout(config_secs: Option<u64>) -> Duration {
    if let Ok(raw) = std::env::var(TIMEOUT_ENV_VAR)
        && let Ok(secs) = raw.parse::<u64>()
        && secs > 0
    {
        return Duration::from_secs(secs);
    }
    Duration::from_secs(
        config_secs
            .filter(|s| *s > 0)
            .unwrap_or(DEFAULT_TIMEOUT_SECS),
    )
}

/// Resolve the effective timeout for a single tool call.
///
/// - `call_override`: the value the agent passed in `timeout_secs`, if any
///   ([`extract_override`] is the canonical way to obtain it).
/// - `config_secs`: the `[engine] tool_timeout_secs` value from config.
///
/// Returns `(duration, source)` where `source` is `"override"` if the agent
/// supplied one and `"default"` otherwise — useful for diagnostics.
pub fn resolve(call_override: Option<u64>, config_secs: Option<u64>) -> (Duration, &'static str) {
    match call_override {
        Some(secs) if secs > 0 => (Duration::from_secs(secs), "override"),
        _ => (default_timeout(config_secs), "default"),
    }
}

/// Strip `timeout_secs` from the tool argument object and return its value
/// if present. Mutates `args` in place so the downstream tool never sees the
/// field.
///
/// Returns:
/// - `Ok(None)` if the field is absent or the args are not an object.
/// - `Ok(Some(secs))` if a positive integer was present (and was removed).
/// - `Err(message)` if the field was present but invalid (zero, negative,
///   non-integer). The caller surfaces this as a tool error so the agent
///   learns the rule.
pub fn extract_override(args: &mut Value) -> Result<Option<u64>, String> {
    let Some(obj) = args.as_object_mut() else {
        return Ok(None);
    };
    let Some(raw) = obj.remove(TIMEOUT_PARAM) else {
        return Ok(None);
    };
    match raw {
        Value::Number(n) => {
            if let Some(secs) = n.as_u64()
                && secs > 0
            {
                Ok(Some(secs))
            } else {
                Err(format!(
                    "{TIMEOUT_PARAM} must be a positive integer; got {n}"
                ))
            }
        }
        other => Err(format!(
            "{TIMEOUT_PARAM} must be a positive integer; got {other}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Use a process-wide lock to serialise tests that mutate the env var.
    // env::set_var is unsafe-by-default in 2024 edition; the lock isolates
    // concurrent test runs within the same process.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_env<F: FnOnce()>(value: Option<&str>, f: F) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var(TIMEOUT_ENV_VAR).ok();
        match value {
            Some(v) => unsafe { std::env::set_var(TIMEOUT_ENV_VAR, v) },
            None => unsafe { std::env::remove_var(TIMEOUT_ENV_VAR) },
        }
        f();
        match prev {
            Some(v) => unsafe { std::env::set_var(TIMEOUT_ENV_VAR, v) },
            None => unsafe { std::env::remove_var(TIMEOUT_ENV_VAR) },
        }
    }

    #[test]
    fn default_when_no_env_no_config() {
        with_env(None, || {
            assert_eq!(default_timeout(None), Duration::from_secs(120));
        });
    }

    #[test]
    fn config_overrides_default() {
        with_env(None, || {
            assert_eq!(default_timeout(Some(60)), Duration::from_secs(60));
        });
    }

    #[test]
    fn env_overrides_config() {
        with_env(Some("90"), || {
            assert_eq!(default_timeout(Some(60)), Duration::from_secs(90));
        });
    }

    #[test]
    fn env_zero_falls_back() {
        with_env(Some("0"), || {
            assert_eq!(default_timeout(Some(45)), Duration::from_secs(45));
        });
    }

    #[test]
    fn env_garbage_falls_back() {
        with_env(Some("not-a-number"), || {
            assert_eq!(default_timeout(Some(45)), Duration::from_secs(45));
        });
    }

    #[test]
    fn config_zero_falls_back_to_default() {
        with_env(None, || {
            assert_eq!(default_timeout(Some(0)), Duration::from_secs(120));
        });
    }

    #[test]
    fn resolve_with_override_uses_override() {
        with_env(None, || {
            let (d, src) = resolve(Some(5), Some(60));
            assert_eq!(d, Duration::from_secs(5));
            assert_eq!(src, "override");
        });
    }

    #[test]
    fn resolve_without_override_uses_default() {
        with_env(None, || {
            let (d, src) = resolve(None, Some(60));
            assert_eq!(d, Duration::from_secs(60));
            assert_eq!(src, "default");
        });
    }

    #[test]
    fn extract_override_absent() {
        let mut args = json!({"foo": "bar"});
        assert_eq!(extract_override(&mut args).unwrap(), None);
        assert_eq!(args, json!({"foo": "bar"}));
    }

    #[test]
    fn extract_override_strips_and_returns() {
        let mut args = json!({"foo": "bar", "timeout_secs": 30});
        assert_eq!(extract_override(&mut args).unwrap(), Some(30));
        assert_eq!(args, json!({"foo": "bar"}));
    }

    #[test]
    fn extract_override_zero_is_rejected() {
        let mut args = json!({"timeout_secs": 0});
        assert!(extract_override(&mut args).is_err());
    }

    #[test]
    fn extract_override_negative_is_rejected() {
        let mut args = json!({"timeout_secs": -5});
        assert!(extract_override(&mut args).is_err());
    }

    #[test]
    fn extract_override_string_is_rejected() {
        let mut args = json!({"timeout_secs": "30"});
        assert!(extract_override(&mut args).is_err());
    }

    #[test]
    fn extract_override_non_object_args_returns_none() {
        let mut args = json!("not an object");
        assert_eq!(extract_override(&mut args).unwrap(), None);
    }
}

//! Custom assertion macros for test readability.

/// Assert that a JSON value contains the expected key-value pairs.
///
/// ```rust,ignore
/// use arawn_test_utils::assert_json_contains;
///
/// let body: serde_json::Value = serde_json::json!({"id": "123", "name": "test", "extra": true});
/// assert_json_contains!(body, {"id": "123", "name": "test"});
/// ```
#[macro_export]
macro_rules! assert_json_contains {
    ($actual:expr, { $($key:tt : $val:expr),* $(,)? }) => {{
        let actual = &$actual;
        $(
            let key = stringify!($key).trim_matches('"');
            let expected = serde_json::json!($val);
            let got = actual.get(key);
            assert!(
                got.is_some(),
                "Expected key '{}' not found in JSON.\nActual: {}",
                key,
                serde_json::to_string_pretty(actual).unwrap_or_default()
            );
            assert_eq!(
                got.unwrap(), &expected,
                "Key '{}' mismatch.\nExpected: {}\nGot: {}\nFull JSON: {}",
                key,
                expected,
                got.unwrap(),
                serde_json::to_string_pretty(actual).unwrap_or_default()
            );
        )*
    }};
}

/// Assert that an HTTP response has the expected status code.
///
/// ```rust,ignore
/// use arawn_test_utils::assert_status;
///
/// let resp = client.get("/health").send().await.unwrap();
/// assert_status!(resp, 200);
/// ```
#[macro_export]
macro_rules! assert_status {
    ($resp:expr, $status:expr) => {{
        let status = $resp.status().as_u16();
        assert_eq!(
            status, $status,
            "Expected status {}, got {}",
            $status, status
        );
    }};
}

/// Assert that a string contains a substring (with better error messages).
#[macro_export]
macro_rules! assert_contains {
    ($haystack:expr, $needle:expr) => {{
        let haystack = &$haystack;
        let needle = $needle;
        assert!(
            haystack.contains(needle),
            "Expected string to contain '{}'.\nActual: '{}'",
            needle,
            haystack
        );
    }};
}

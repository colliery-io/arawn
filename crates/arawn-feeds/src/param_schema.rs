//! Machine-readable parameter schema for feed templates.
//!
//! Every [`crate::template::FeedTemplate`] declares its accepted params as a
//! list of [`ParamSpec`] via `FeedTemplate::param_schema()` (a *required*
//! trait method — there is no default impl, so the compiler refuses any
//! template that hasn't described its config). The `/watch` modal renders a
//! form from this; future consumers (docs generation, validation cross-checks)
//! can read it too.
//!
//! This is *descriptive*. The server-side `validate()` remains the source of
//! truth for what's actually accepted — the schema should agree with it, and a
//! registry-wide test guards against drift, but the schema never replaces
//! validation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The kind of a single parameter — drives which widget the form renders and
/// how the entered value is coerced before submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "values")]
pub enum ParamKind {
    /// Free-form string (channel name, sender pattern, label, project key…).
    Text,
    /// Integer. Bounds (e.g. "1–60") live in [`ParamSpec::help`]; the server's
    /// `validate()` enforces them — the schema deliberately doesn't duplicate
    /// the range to avoid schema↔validate drift.
    Int,
    /// Boolean toggle.
    Bool,
    /// A filesystem path. Server-side it's just a string; the UI may treat it
    /// specially (file-picker affordance). Critically, a `Path` field is a
    /// single value typed verbatim — so a path with spaces needs no quoting.
    Path,
    /// A list of strings entered space/comma-separated, coerced to a JSON
    /// array (e.g. filesystem `include` / `exclude` globs).
    List,
    /// A first-run backfill point-in-time. Accepts the `parse_since` grammar:
    /// relative (`7d`, `12h`, `6w`, `6mo`), ISO date (`2026-01-01`), or RFC3339.
    /// Shared by gmail/slack/jira/drive feeds. Distinct from `Text` so the
    /// form can apply that grammar and show duration help.
    Since,
    /// A fixed set of allowed string values (rendered as a selector). No
    /// current template uses this; retained for future templates.
    Enum(Vec<String>),
}

/// One declared parameter of a feed template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamSpec {
    /// The JSON key submitted to `feed_register` (e.g. `"root"`).
    pub key: String,
    /// Human label for the form field (e.g. "Folder to watch").
    pub label: String,
    /// What kind of value this is — drives the widget + coercion.
    pub kind: ParamKind,
    /// Whether the field must be filled. Required fields carry no `default`.
    pub required: bool,
    /// Pre-filled value for optional fields; `None` for required fields.
    pub default: Option<Value>,
    /// One-line guidance shown beneath the field (purpose, ranges, examples).
    pub help: String,
}

impl ParamSpec {
    /// A required parameter (no default).
    pub fn required(key: &str, label: &str, kind: ParamKind, help: &str) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            kind,
            required: true,
            default: None,
            help: help.to_string(),
        }
    }

    /// An optional parameter with a pre-filled default.
    pub fn optional(key: &str, label: &str, kind: ParamKind, default: Value, help: &str) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            kind,
            required: false,
            default: Some(default),
            help: help.to_string(),
        }
    }

    /// An optional parameter with no pre-filled default (e.g. `since`).
    pub fn optional_no_default(key: &str, label: &str, kind: ParamKind, help: &str) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            kind,
            required: false,
            default: None,
            help: help.to_string(),
        }
    }

    /// The shared first-run backfill `since` field. Every template that
    /// supports cold-start backfill declares it identically, so this helper
    /// keeps the wording consistent across the six that use it.
    pub fn since() -> Self {
        Self::optional_no_default(
            "since",
            "Backfill from",
            ParamKind::Since,
            "First-run only: how far back to seed history. Relative (7d, 6w, 6mo), \
             a date (2026-01-01), or an RFC3339 timestamp. Ignored once the feed has run.",
        )
    }

    /// True when `default` (if present) is type-consistent with `kind`. Used by
    /// the registry-wide self-consistency test.
    pub fn default_matches_kind(&self) -> bool {
        match &self.default {
            None => true,
            Some(v) => match self.kind {
                ParamKind::Text | ParamKind::Path | ParamKind::Since => v.is_string(),
                ParamKind::Int => v.is_number(),
                ParamKind::Bool => v.is_boolean(),
                ParamKind::List => v.is_array(),
                ParamKind::Enum(ref allowed) => {
                    v.as_str().is_some_and(|s| allowed.iter().any(|a| a == s))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn constructors_set_required_and_default() {
        let req = ParamSpec::required("root", "Root", ParamKind::Path, "the folder");
        assert!(req.required);
        assert!(req.default.is_none());

        let opt = ParamSpec::optional("recursive", "Recursive", ParamKind::Bool, json!(true), "h");
        assert!(!opt.required);
        assert_eq!(opt.default, Some(json!(true)));

        let since = ParamSpec::since();
        assert_eq!(since.key, "since");
        assert!(matches!(since.kind, ParamKind::Since));
        assert!(!since.required);
        assert!(since.default.is_none());
    }

    #[test]
    fn default_type_consistency() {
        assert!(
            ParamSpec::optional("n", "N", ParamKind::Int, json!(7), "h").default_matches_kind()
        );
        assert!(
            !ParamSpec::optional("n", "N", ParamKind::Int, json!("7"), "h").default_matches_kind()
        );
        assert!(
            ParamSpec::optional("xs", "Xs", ParamKind::List, json!(["a"]), "h")
                .default_matches_kind()
        );
        assert!(
            !ParamSpec::optional("b", "B", ParamKind::Bool, json!("true"), "h")
                .default_matches_kind()
        );
        // Enum default must be one of the allowed values.
        let kind = ParamKind::Enum(vec!["a".into(), "b".into()]);
        assert!(
            ParamSpec::optional("e", "E", kind.clone(), json!("a"), "h").default_matches_kind()
        );
        assert!(!ParamSpec::optional("e", "E", kind, json!("z"), "h").default_matches_kind());
    }

    #[test]
    fn param_spec_roundtrips_json() {
        let spec = ParamSpec::optional(
            "include",
            "Include globs",
            ParamKind::List,
            json!(["**/*"]),
            "patterns",
        );
        let s = serde_json::to_string(&spec).unwrap();
        let back: ParamSpec = serde_json::from_str(&s).unwrap();
        assert_eq!(spec, back);
    }

    #[test]
    fn enum_kind_roundtrips_json() {
        let spec = ParamSpec::required(
            "mode",
            "Mode",
            ParamKind::Enum(vec!["fast".into(), "full".into()]),
            "pick one",
        );
        let s = serde_json::to_string(&spec).unwrap();
        let back: ParamSpec = serde_json::from_str(&s).unwrap();
        assert_eq!(spec, back);
    }
}

use serde::{Deserialize, Serialize};
use tracing::warn;

use super::checker::PermissionMode;
use super::rules::{PermissionRule, RuleKind};

/// Permission configuration — holds allow/deny/ask rule lists.
///
/// Loaded from settings and merged with priority (user > project > defaults).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// Rules that explicitly allow tool execution.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Rules that explicitly deny tool execution.
    #[serde(default)]
    pub deny: Vec<String>,
    /// Rules that require user confirmation before tool execution.
    #[serde(default)]
    pub ask: Vec<String>,
    /// T-0347: starting permission posture. Mirrors `/autonomy` slash
    /// command. Values: `ask` | `edits` | `full` | `plan`. Absent →
    /// `Ask` (default-safe). Invalid TOML values surface as parse
    /// errors from serde — callers should warn-and-fall-back.
    #[serde(default)]
    pub autonomy: Option<PermissionMode>,
}

impl PermissionConfig {
    /// Parse the string-based config into typed `PermissionRule` values.
    /// Malformed entries are logged as warnings and skipped.
    pub fn into_rules(&self) -> Vec<PermissionRule> {
        let mut rules = Vec::new();

        for spec in &self.deny {
            rules.push(PermissionRule::parse(RuleKind::Deny, spec));
        }
        for spec in &self.allow {
            rules.push(PermissionRule::parse(RuleKind::Allow, spec));
        }
        for spec in &self.ask {
            rules.push(PermissionRule::parse(RuleKind::Ask, spec));
        }

        rules
    }

    /// Merge two configs: `self` is higher priority (e.g., user-level),
    /// `other` is lower priority (e.g., project-level).
    ///
    /// Higher-priority deny rules are prepended so they're checked first.
    /// All rules from both sources are included.
    pub fn merge(self, other: PermissionConfig) -> PermissionConfig {
        PermissionConfig {
            allow: [self.allow, other.allow].concat(),
            deny: [self.deny, other.deny].concat(),
            ask: [self.ask, other.ask].concat(),
            // Autonomy is a single value — higher-priority (self) wins.
            autonomy: self.autonomy.or(other.autonomy),
        }
    }
}

/// Wrapper for the permissions section in the top-level config.
/// This gets embedded in ArawnConfig as an optional field.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionsSection {
    #[serde(default)]
    pub permissions: PermissionConfig,
}

/// Load permission config from a TOML file, returning defaults if the file
/// is missing or the permissions section is absent.
pub fn load_permissions_from_file(path: &std::path::Path) -> PermissionConfig {
    if !path.exists() {
        return PermissionConfig::default();
    }

    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str::<PermissionsSection>(&content) {
            Ok(section) => section.permissions,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "failed to parse permissions from config, using defaults");
                PermissionConfig::default()
            }
        },
        Err(e) => {
            warn!(path = %path.display(), error = %e, "failed to read config file, using defaults");
            PermissionConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::rules::{PermissionDecision, RuleMatcher};
    use std::io::Write;

    #[test]
    fn empty_config_produces_no_rules() {
        let config = PermissionConfig::default();
        assert!(config.into_rules().is_empty());
    }

    #[test]
    fn config_parses_rules() {
        let config = PermissionConfig {
            allow: vec!["Read".into(), "Glob".into()],
            deny: vec!["Bash(rm -rf *)".into()],
            ask: vec!["Bash".into()],
            autonomy: None,
        };
        let rules = config.into_rules();
        assert_eq!(rules.len(), 4);

        // Deny rules come first (from into_rules ordering)
        assert_eq!(rules[0].kind, RuleKind::Deny);
        assert_eq!(rules[0].tool_pattern, "Bash");

        // Then allow
        assert_eq!(rules[1].kind, RuleKind::Allow);
        assert_eq!(rules[2].kind, RuleKind::Allow);

        // Then ask
        assert_eq!(rules[3].kind, RuleKind::Ask);
    }

    #[test]
    fn merge_preserves_priority() {
        let user = PermissionConfig {
            allow: vec![],
            deny: vec!["Bash(rm *)".into()],
            ask: vec![],
            autonomy: None,
        };
        let project = PermissionConfig {
            allow: vec!["Bash".into()],
            deny: vec![],
            ask: vec![],
            autonomy: None,
        };

        let merged = user.merge(project);
        let rules = merged.into_rules();

        // User's deny should block even though project allows Bash
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Bash", "rm -rf /"),
            PermissionDecision::Denied
        );
        // Non-rm Bash should be allowed by project rule
        assert_eq!(
            RuleMatcher::evaluate(&rules, "Bash", "ls"),
            PermissionDecision::Allowed
        );
    }

    #[test]
    fn load_from_toml_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp.as_file(),
            r#"
[permissions]
allow = ["Read", "Glob", "Grep"]
deny = ["Bash(rm -rf *)"]
ask = ["Bash", "Edit"]
"#
        )
        .unwrap();

        let config = load_permissions_from_file(tmp.path());
        assert_eq!(config.allow.len(), 3);
        assert_eq!(config.deny.len(), 1);
        assert_eq!(config.ask.len(), 2);
    }

    // T-0347 — [permissions] autonomy round-trip.
    #[test]
    fn load_autonomy_from_toml() {
        for (val, expected) in [
            ("ask", PermissionMode::Ask),
            ("edits", PermissionMode::Edits),
            ("full", PermissionMode::Full),
            ("plan", PermissionMode::Plan),
        ] {
            let tmp = tempfile::NamedTempFile::new().unwrap();
            write!(
                tmp.as_file(),
                "[permissions]\nautonomy = \"{val}\"\n"
            )
            .unwrap();
            let cfg = load_permissions_from_file(tmp.path());
            assert_eq!(cfg.autonomy, Some(expected), "autonomy = {val}");
        }
    }

    #[test]
    fn load_autonomy_absent_is_none() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp.as_file(),
            "[permissions]\nallow = [\"Read\"]\n"
        )
        .unwrap();
        let cfg = load_permissions_from_file(tmp.path());
        assert_eq!(cfg.autonomy, None);
    }

    #[test]
    fn load_autonomy_invalid_value_falls_back_to_default() {
        // Bad value invalidates the whole [permissions] block via
        // serde — the loader warn+falls-back to PermissionConfig::default,
        // which means autonomy: None, no rules. Verifies the warn-and-
        // fall-back contract from the acceptance criteria.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp.as_file(),
            "[permissions]\nautonomy = \"bogus\"\nallow = [\"Read\"]\n"
        )
        .unwrap();
        let cfg = load_permissions_from_file(tmp.path());
        assert_eq!(cfg.autonomy, None);
        assert!(cfg.allow.is_empty());
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let config = load_permissions_from_file(std::path::Path::new("/nonexistent/arawn.toml"));
        assert!(config.allow.is_empty());
        assert!(config.deny.is_empty());
        assert!(config.ask.is_empty());
    }

    #[test]
    fn load_file_without_permissions_section() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp.as_file(),
            r#"
[engine]
max_iterations = 10
"#
        )
        .unwrap();

        let config = load_permissions_from_file(tmp.path());
        assert!(config.allow.is_empty());
    }

}

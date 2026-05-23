//! Hand-rolled `FeedTemplate` registry.
//!
//! At server boot we build one of these and register every template
//! the binary supports. Cloacina cron tasks look up templates here by
//! name when firing.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::FeedError;
use crate::template::FeedTemplate;

/// Maps template name (`<provider>/<name>`) → impl. Cheap to clone
/// because values are `Arc`s.
#[derive(Default, Clone)]
pub struct FeedTemplateRegistry {
    inner: HashMap<&'static str, Arc<dyn FeedTemplate>>,
}

impl FeedTemplateRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, template: Arc<dyn FeedTemplate>) {
        self.inner.insert(template.name(), template);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn FeedTemplate>> {
        self.inner.get(name).cloned()
    }

    /// Look up or return a structured error so callers don't have to
    /// reach for a free-form string each time.
    pub fn require(&self, name: &str) -> Result<Arc<dyn FeedTemplate>, FeedError> {
        self.get(name).ok_or_else(|| {
            FeedError::InvalidParams(format!("no template registered with name '{name}'"))
        })
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.inner.keys().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FeedDefaults, TemplateParams};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::path::Path;

    struct DummyTemplate(&'static str);

    #[async_trait]
    impl FeedTemplate for DummyTemplate {
        fn name(&self) -> &'static str {
            self.0
        }
        fn validate(&self, _params: &TemplateParams) -> Result<(), FeedError> {
            Ok(())
        }
        fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
            FeedDefaults {
                cadence: "*/15 * * * *".into(),
                initial_cursor: Value::Null,
            }
        }
        async fn run(
            &self,
            _ctx: &crate::template::TemplateCtx,
            _params: &TemplateParams,
            _feed_dir: &Path,
            _cursor: &Value,
        ) -> Result<crate::template::RunOutcome, FeedError> {
            unreachable!("not exercised in registry tests")
        }
    }

    #[test]
    fn register_and_lookup_round_trips() {
        let mut reg = FeedTemplateRegistry::new();
        reg.register(Arc::new(DummyTemplate("a/b")));
        reg.register(Arc::new(DummyTemplate("c/d")));
        assert!(reg.get("a/b").is_some());
        assert!(reg.get("c/d").is_some());
    }

    #[test]
    fn require_returns_invalid_params_for_unknown_name() {
        let reg = FeedTemplateRegistry::new();
        match reg.require("nope/missing") {
            Err(FeedError::InvalidParams(msg)) => assert!(msg.contains("nope/missing")),
            Err(other) => panic!("expected InvalidParams, got {other:?}"),
            Ok(_) => panic!("expected error"),
        }
    }
}

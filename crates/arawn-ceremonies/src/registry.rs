//! Process-wide registry of ceremony plugins.
//!
//! Plugins are registered at startup (typically in the binary's
//! main). The cloacina runner (T-0281) walks the registry to wire
//! one workflow per plugin; the RPC dispatcher (T-0283) routes
//! `ceremonies.*` calls by `kind`. Add a new ceremony in the future
//! and you register it here — no other plumbing touches it.

use std::collections::HashMap;
use std::sync::Arc;

use crate::CeremonyError;
use crate::plugin::Ceremony;

/// Holds the `Ceremony` plugins registered with the engine. Cheap to
/// clone; the underlying map is shared.
#[derive(Default, Clone)]
pub struct PluginRegistry {
    inner: Arc<std::sync::RwLock<HashMap<String, Arc<dyn Ceremony>>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a plugin. Errors if a plugin with the same `kind()`
    /// is already registered — the engine relies on `kind` being a
    /// unique dispatch key.
    pub fn register(&self, plugin: Arc<dyn Ceremony>) -> Result<(), CeremonyError> {
        let kind = plugin.kind().to_string();
        let mut w = self.inner.write().expect("PluginRegistry poisoned");
        if w.contains_key(&kind) {
            return Err(CeremonyError::duplicate_kind(kind));
        }
        w.insert(kind, plugin);
        Ok(())
    }

    /// Look up a plugin by `kind`. Returns `None` if no plugin is
    /// registered under that key.
    pub fn get(&self, kind: &str) -> Option<Arc<dyn Ceremony>> {
        self.inner
            .read()
            .expect("PluginRegistry poisoned")
            .get(kind)
            .cloned()
    }

    /// Snapshot of every registered plugin, in undefined order.
    /// Useful for the cron loop (which wants to iterate at startup)
    /// and for the doctor command (which wants to list registered
    /// ceremonies).
    pub fn all(&self) -> Vec<Arc<dyn Ceremony>> {
        self.inner
            .read()
            .expect("PluginRegistry poisoned")
            .values()
            .cloned()
            .collect()
    }

    /// Number of registered plugins.
    pub fn len(&self) -> usize {
        self.inner.read().expect("PluginRegistry poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{Ceremony, CronSchedule, NewItem};
    use crate::types::GatheredFacts;
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};

    struct StubCeremony {
        kind: &'static str,
    }

    #[async_trait]
    impl Ceremony for StubCeremony {
        fn kind(&self) -> &'static str {
            self.kind
        }
        fn period_key(&self, _now: DateTime<Utc>) -> String {
            "stub-period".into()
        }
        fn default_schedule(&self) -> CronSchedule {
            CronSchedule::local("0 0 * * *")
        }
        async fn gather(
            &self,
            _ctx: &dyn crate::plugin::CeremonyCtx,
        ) -> Result<GatheredFacts, CeremonyError> {
            Ok(GatheredFacts::new(serde_json::json!({})))
        }
        async fn compose(
            &self,
            _ctx: &dyn crate::plugin::CeremonyCtx,
            _facts: GatheredFacts,
        ) -> Result<Vec<NewItem>, CeremonyError> {
            Ok(Vec::new())
        }
    }

    fn stub(kind: &'static str) -> Arc<dyn Ceremony> {
        Arc::new(StubCeremony { kind })
    }

    #[test]
    fn register_and_get_by_kind() {
        let reg = PluginRegistry::new();
        reg.register(stub("daily")).unwrap();
        assert_eq!(reg.len(), 1);
        let p = reg.get("daily").expect("plugin should be present");
        assert_eq!(p.kind(), "daily");
    }

    #[test]
    fn unknown_kind_returns_none() {
        let reg = PluginRegistry::new();
        reg.register(stub("daily")).unwrap();
        assert!(reg.get("retro").is_none());
    }

    #[test]
    fn duplicate_kind_is_rejected() {
        let reg = PluginRegistry::new();
        reg.register(stub("daily")).unwrap();
        let result = reg.register(stub("daily"));
        assert!(matches!(result, Err(CeremonyError::DuplicateKind(_))));
    }

    #[test]
    fn all_returns_every_registered_plugin() {
        let reg = PluginRegistry::new();
        reg.register(stub("daily")).unwrap();
        reg.register(stub("weekly")).unwrap();
        reg.register(stub("retro")).unwrap();
        let mut kinds: Vec<&str> = reg.all().iter().map(|p| p.kind()).collect();
        kinds.sort();
        assert_eq!(kinds, vec!["daily", "retro", "weekly"]);
    }

    #[test]
    fn empty_registry() {
        let reg = PluginRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.all().is_empty());
    }

    #[test]
    fn registry_is_clone_share() {
        let reg = PluginRegistry::new();
        let cloned = reg.clone();
        reg.register(stub("daily")).unwrap();
        // Cloned share the inner Arc → sees the registration.
        assert_eq!(cloned.len(), 1);
    }
}

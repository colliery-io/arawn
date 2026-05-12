//! Workstream-scoped memory routing.
//!
//! `WorkstreamMemoryRouter` opens a fresh `MemoryManager` per workstream
//! on first access and caches it for subsequent reads. Memory tools
//! consult the active `SessionWorkstream` to pick which manager to use
//! at execute time.
//!
//! Test code passes `MemoryHandle::Fixed(Arc<MemoryManager>)` so the
//! existing fixed-manager tests continue working unchanged.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use arawn_embed::Embedder;
use arawn_memory::{MemoryError, MemoryManager};

use crate::tools::SessionWorkstream;

/// Lazy + cached map of workstream-name → `MemoryManager`.
pub struct WorkstreamMemoryRouter {
    data_dir: PathBuf,
    embedding_dims: Option<usize>,
    embedder: Option<Arc<dyn Embedder>>,
    session: SessionWorkstream,
    cache: Mutex<HashMap<String, Arc<MemoryManager>>>,
}

impl WorkstreamMemoryRouter {
    pub fn new(
        data_dir: impl Into<PathBuf>,
        embedding_dims: Option<usize>,
        embedder: Option<Arc<dyn Embedder>>,
        session: SessionWorkstream,
    ) -> Self {
        Self {
            data_dir: data_dir.into(),
            embedding_dims,
            embedder,
            session,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve the active workstream's memory manager. Opens (and
    /// caches) the KB on first touch.
    pub fn current(&self) -> Result<Arc<MemoryManager>, MemoryError> {
        let name = self.session.current();
        self.for_workstream(&name)
    }

    pub fn for_workstream(&self, name: &str) -> Result<Arc<MemoryManager>, MemoryError> {
        if let Some(existing) = self.cache.lock().unwrap().get(name).cloned() {
            return Ok(existing);
        }
        let mut mgr = MemoryManager::for_workstream(&self.data_dir, name, self.embedding_dims)?;
        if let Some(e) = self.embedder.as_ref() {
            mgr = mgr.with_embedder(Arc::clone(e));
        }
        let arc = Arc::new(mgr);
        self.cache
            .lock()
            .unwrap()
            .insert(name.to_string(), Arc::clone(&arc));
        Ok(arc)
    }
}

/// Memory tools depend on one of these. `Fixed` is for tests and
/// any caller that doesn't care about workstream routing. `Routed`
/// is the production wiring.
#[derive(Clone)]
pub enum MemoryHandle {
    Fixed(Arc<MemoryManager>),
    Routed(Arc<WorkstreamMemoryRouter>),
}

impl MemoryHandle {
    /// Resolve the active manager. For `Fixed`, always the same one;
    /// for `Routed`, the one matching the current `SessionWorkstream`.
    pub fn manager(&self) -> Result<Arc<MemoryManager>, MemoryError> {
        match self {
            MemoryHandle::Fixed(m) => Ok(Arc::clone(m)),
            MemoryHandle::Routed(r) => r.current(),
        }
    }
}

impl From<Arc<MemoryManager>> for MemoryHandle {
    fn from(m: Arc<MemoryManager>) -> Self {
        MemoryHandle::Fixed(m)
    }
}

impl From<Arc<WorkstreamMemoryRouter>> for MemoryHandle {
    fn from(r: Arc<WorkstreamMemoryRouter>) -> Self {
        MemoryHandle::Routed(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_caches_per_workstream() {
        let tmp = tempfile::tempdir().unwrap();
        let session = SessionWorkstream::scratch();
        let router = WorkstreamMemoryRouter::new(tmp.path(), None, None, session.clone());

        let m1 = router.current().unwrap();
        let m2 = router.current().unwrap();
        assert!(Arc::ptr_eq(&m1, &m2), "cache should return the same manager");

        session.set("other");
        let m3 = router.current().unwrap();
        assert!(!Arc::ptr_eq(&m1, &m3), "different workstream should get a different manager");
    }

    #[test]
    fn fixed_handle_dispatches() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = Arc::new(MemoryManager::open(tmp.path(), "scratch", None).unwrap());
        let h = MemoryHandle::Fixed(mgr.clone());
        assert!(Arc::ptr_eq(&h.manager().unwrap(), &mgr));
    }
}

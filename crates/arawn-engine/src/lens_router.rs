//! Lens-scoped memory routing.
//!
//! `LensMemoryRouter` opens a fresh `MemoryManager` per lens
//! on first access and caches it for subsequent reads. Memory tools
//! consult the active `SessionLens` to pick which manager to use
//! at execute time.
//!
//! Test code passes `MemoryHandle::Fixed(Arc<MemoryManager>)` so the
//! existing fixed-manager tests continue working unchanged.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use arawn_embed::Embedder;
use arawn_memory::{MemoryError, MemoryManager};

use crate::tools::SessionLens;

/// Lazy + cached map of lens-name → `MemoryManager`.
pub struct LensMemoryRouter {
    data_dir: PathBuf,
    embedding_dims: Option<usize>,
    embedder: Option<Arc<dyn Embedder>>,
    session: SessionLens,
    cache: Mutex<HashMap<String, Arc<MemoryManager>>>,
}

impl LensMemoryRouter {
    pub fn new(
        data_dir: impl Into<PathBuf>,
        embedding_dims: Option<usize>,
        embedder: Option<Arc<dyn Embedder>>,
        session: SessionLens,
    ) -> Self {
        Self {
            data_dir: data_dir.into(),
            embedding_dims,
            embedder,
            session,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve the active lens's memory manager. Opens (and
    /// caches) the KB on first touch.
    pub fn current(&self) -> Result<Arc<MemoryManager>, MemoryError> {
        let name = self.session.current();
        self.for_lens(&name)
    }

    /// Name of the active lens — useful for tools that need to
    /// open external per-lens stores (e.g. the steward journal).
    pub fn current_name(&self) -> String {
        self.session.current()
    }

    /// Every lens's memory manager (name → manager), for cross-lens reads
    /// (ARAWN-I-0060). Enumerates the on-disk lens KB directories under
    /// `<data_dir>/lenses/` — the dir name *is* the lens name
    /// (`lens_dir_name` returns the bare name) — and opens/caches each via
    /// [`Self::for_lens`]. Lenses with no KB yet (no writes) don't appear,
    /// which is correct: there's nothing to read. Errors opening any one lens
    /// are skipped, not fatal.
    pub fn all_lens_managers(&self) -> Vec<(String, Arc<MemoryManager>)> {
        let mut out = Vec::new();
        let dir = self.data_dir.join("lenses");
        let Ok(rd) = std::fs::read_dir(&dir) else {
            return out;
        };
        for entry in rd.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && let Some(name) = entry.file_name().to_str()
                && let Ok(mgr) = self.for_lens(name)
            {
                out.push((name.to_string(), mgr));
            }
        }
        out
    }

    pub fn for_lens(&self, name: &str) -> Result<Arc<MemoryManager>, MemoryError> {
        if let Some(existing) = self.cache.lock().unwrap().get(name).cloned() {
            return Ok(existing);
        }
        let mut mgr = MemoryManager::for_lens(&self.data_dir, name, self.embedding_dims)?;
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
/// any caller that doesn't care about lens routing. `Routed`
/// is the production wiring.
#[derive(Clone)]
pub enum MemoryHandle {
    Fixed(Arc<MemoryManager>),
    Routed(Arc<LensMemoryRouter>),
}

impl MemoryHandle {
    /// Resolve the active manager. For `Fixed`, always the same one;
    /// for `Routed`, the one matching the current `SessionLens`.
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

impl From<Arc<LensMemoryRouter>> for MemoryHandle {
    fn from(r: Arc<LensMemoryRouter>) -> Self {
        MemoryHandle::Routed(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_caches_per_lens() {
        let tmp = tempfile::tempdir().unwrap();
        let session = SessionLens::scratch();
        let router = LensMemoryRouter::new(tmp.path(), None, None, session.clone());

        let m1 = router.current().unwrap();
        let m2 = router.current().unwrap();
        assert!(
            Arc::ptr_eq(&m1, &m2),
            "cache should return the same manager"
        );

        session.set("other");
        let m3 = router.current().unwrap();
        assert!(
            !Arc::ptr_eq(&m1, &m3),
            "different lens should get a different manager"
        );
    }

    #[test]
    fn fixed_handle_dispatches() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = Arc::new(MemoryManager::open(tmp.path(), "scratch", None).unwrap());
        let h = MemoryHandle::Fixed(mgr.clone());
        assert!(Arc::ptr_eq(&h.manager().unwrap(), &mgr));
    }
}

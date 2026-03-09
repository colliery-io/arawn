//! Memory service for the domain layer.
//!
//! Wraps `arawn-memory::MemoryStore` to provide memory and note operations
//! through the domain facade. This ensures the server's REST API and the
//! agent's internal memory share the same backing store.

use std::sync::Arc;

use arawn_memory::MemoryStore;

/// Domain service for memory and note operations.
///
/// Wraps an optional `MemoryStore`. When `None`, all operations return
/// "not available" errors, allowing graceful degradation when memory
/// is not configured.
#[derive(Clone)]
pub struct MemoryService {
    store: Option<Arc<MemoryStore>>,
}

impl MemoryService {
    /// Create a new memory service.
    pub fn new(store: Option<Arc<MemoryStore>>) -> Self {
        Self { store }
    }

    /// Whether the memory store is available.
    pub fn is_enabled(&self) -> bool {
        self.store.is_some()
    }

    /// Get the underlying memory store.
    ///
    /// Returns `None` if memory is not configured.
    pub fn store(&self) -> Option<&Arc<MemoryStore>> {
        self.store.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_service_disabled() {
        let service = MemoryService::new(None);
        assert!(!service.is_enabled());
        assert!(service.store().is_none());
    }

    #[test]
    fn test_memory_service_enabled() {
        let store = Arc::new(MemoryStore::open_in_memory().unwrap());
        let service = MemoryService::new(Some(store));
        assert!(service.is_enabled());
        assert!(service.store().is_some());
    }

    #[test]
    fn test_memory_service_clone() {
        let store = Arc::new(MemoryStore::open_in_memory().unwrap());
        let service = MemoryService::new(Some(store));
        let cloned = service.clone();
        assert!(cloned.is_enabled());
        // Both point to the same store
        assert!(Arc::ptr_eq(
            service.store().unwrap(),
            cloned.store().unwrap()
        ));
    }

    #[test]
    fn test_memory_service_clone_disabled() {
        let service = MemoryService::new(None);
        let cloned = service.clone();
        assert!(!cloned.is_enabled());
    }
}

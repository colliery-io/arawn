//! `LocalService` inherent methods backing the `lenses.*` portion of
//! `ArawnService`. The trait shell in `super::mod` delegates to these.

use std::path::PathBuf;

use arawn_core::Lens;
use arawn_service::{LensInfo, ServiceError};

use super::LocalService;

impl LocalService {
    pub(super) async fn list_lenses_inner(&self) -> Result<Vec<LensInfo>, ServiceError> {
        let store = self.store.lock().unwrap();
        let lenses = store.list_lenses()?;

        Ok(lenses
            .into_iter()
            .map(|ws| LensInfo {
                id: ws.id,
                name: ws.name,
                root_dir: ws.root_dir,
                created_at: ws.created_at,
            })
            .collect())
    }

    pub(super) async fn create_lens_inner(
        &self,
        name: String,
        root_dir: PathBuf,
    ) -> Result<LensInfo, ServiceError> {
        let ws = Lens::new(&name, &root_dir);
        let store = self.store.lock().unwrap();
        store.create_lens(&ws)?;

        Ok(LensInfo {
            id: ws.id,
            name: ws.name,
            root_dir: ws.root_dir,
            created_at: ws.created_at,
        })
    }
}

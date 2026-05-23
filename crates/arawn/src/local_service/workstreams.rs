//! `LocalService` inherent methods backing the `workstreams.*` portion of
//! `ArawnService`. The trait shell in `super::mod` delegates to these.

use std::path::PathBuf;

use arawn_core::Workstream;
use arawn_service::{ServiceError, WorkstreamInfo};


use super::LocalService;

impl LocalService {
    pub(super) async fn list_workstreams_inner(&self) -> Result<Vec<WorkstreamInfo>, ServiceError> {
        let store = self.store.lock().unwrap();
        let workstreams = store.list_workstreams()?;

        Ok(workstreams
            .into_iter()
            .map(|ws| WorkstreamInfo {
                id: ws.id,
                name: ws.name,
                root_dir: ws.root_dir,
                created_at: ws.created_at,
            })
            .collect())
    }

    pub(super) async fn create_workstream_inner(
        &self,
        name: String,
        root_dir: PathBuf,
    ) -> Result<WorkstreamInfo, ServiceError> {
        let ws = Workstream::new(&name, &root_dir);
        let store = self.store.lock().unwrap();
        store.create_workstream(&ws)?;

        Ok(WorkstreamInfo {
            id: ws.id,
            name: ws.name,
            root_dir: ws.root_dir,
            created_at: ws.created_at,
        })
    }

}

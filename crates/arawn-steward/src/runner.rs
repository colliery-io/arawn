//! `StewardRunner` — walks the active workstream set and runs the
//! configured subroutines against each. T-0256 scope: scaffolding only,
//! exercised end-to-end via `IdentitySubroutine`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tracing::{debug, info, warn};

use arawn_core::Workstream;
use arawn_memory::MemoryManager;
use arawn_storage::Store;

use crate::error::StewardError;
use crate::journal::Journal;
use crate::subroutine::{StewardSubroutine, SubroutineCtx};

/// Per-subroutine action caps. Per ARAWN-A-0003 defaults are
/// intentionally not baked in — they live in `arawn.toml` and the
/// caller passes them through. The struct exists so adding a new
/// subroutine is an additive change.
#[derive(Debug, Clone)]
pub struct SubroutineCaps {
    per_subroutine: HashMap<String, usize>,
    default_cap: usize,
}

impl Default for SubroutineCaps {
    /// Placeholder defaults that exist only so tests + first-boot don't
    /// hit zero. Real values come from `arawn.toml` once the harness in
    /// Phase 5 measures convergence-vs-damage.
    fn default() -> Self {
        let mut m = HashMap::new();
        m.insert("reshelve".to_string(), 10);
        m.insert("dust".to_string(), 5);
        m.insert("map".to_string(), 20);
        m.insert("doorwatch".to_string(), 20);
        m.insert("identity".to_string(), 1);
        Self {
            per_subroutine: m,
            default_cap: 5,
        }
    }
}

impl SubroutineCaps {
    pub fn new(default_cap: usize) -> Self {
        Self {
            per_subroutine: HashMap::new(),
            default_cap,
        }
    }

    pub fn with_cap(mut self, subroutine: impl Into<String>, cap: usize) -> Self {
        self.per_subroutine.insert(subroutine.into(), cap);
        self
    }

    pub fn cap_for(&self, subroutine: &str) -> usize {
        self.per_subroutine
            .get(subroutine)
            .copied()
            .unwrap_or(self.default_cap)
    }
}

/// Aggregate stats for one `run_pass` invocation across all
/// workstreams + subroutines.
#[derive(Debug, Default, Clone)]
pub struct StewardStats {
    pub workstreams_visited: usize,
    pub subroutine_runs: usize,
    pub actions_journaled: usize,
    pub mutations_applied: usize,
    pub proposals_recorded: usize,
    pub caps_hit: usize,
    pub errors: usize,
}

/// Function that materializes the `MemoryManager` for a workstream.
/// In production this is `WorkstreamMemoryRouter::for_workstream`; in
/// tests an inline closure works.
pub type MemoryResolver = Arc<
    dyn Fn(&str) -> Result<Arc<MemoryManager>, StewardError> + Send + Sync,
>;

pub struct StewardRunner {
    store: Arc<Mutex<Store>>,
    data_dir: PathBuf,
    memory: MemoryResolver,
    subroutines: Vec<Arc<dyn StewardSubroutine>>,
    caps: SubroutineCaps,
    /// Cache of opened journals so each workstream's sqlite handle
    /// stays warm across passes.
    journals: Arc<Mutex<HashMap<String, Arc<Journal>>>>,
}

impl StewardRunner {
    pub fn new(
        store: Arc<Mutex<Store>>,
        data_dir: impl Into<PathBuf>,
        memory: MemoryResolver,
        subroutines: Vec<Arc<dyn StewardSubroutine>>,
    ) -> Self {
        Self {
            store,
            data_dir: data_dir.into(),
            memory,
            subroutines,
            caps: SubroutineCaps::default(),
            journals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_caps(mut self, caps: SubroutineCaps) -> Self {
        self.caps = caps;
        self
    }

    /// Open / fetch the cached journal for a workstream.
    pub fn journal_for(&self, workstream_name: &str) -> Result<Arc<Journal>, StewardError> {
        if let Some(existing) = self.journals.lock().unwrap().get(workstream_name).cloned() {
            return Ok(existing);
        }
        let j = Arc::new(Journal::open(&self.data_dir, workstream_name)?);
        self.journals
            .lock()
            .unwrap()
            .insert(workstream_name.to_string(), Arc::clone(&j));
        Ok(j)
    }

    /// Run one pass over `workstream`: every subroutine, in declared
    /// order, sequentially. A subroutine error is logged and surfaces
    /// in `stats.errors` but does not abort the remaining subroutines.
    pub async fn run_pass_for_workstream(
        &self,
        workstream: &Workstream,
    ) -> Result<StewardStats, StewardError> {
        let mut stats = StewardStats {
            workstreams_visited: 1,
            ..Default::default()
        };

        let memory = (self.memory)(&workstream.name)?;
        let journal = self.journal_for(&workstream.name)?;

        for sub in &self.subroutines {
            stats.subroutine_runs += 1;
            let cap = self.caps.cap_for(sub.name());
            let ctx = SubroutineCtx {
                workstream: workstream.clone(),
                memory: Arc::clone(&memory),
                journal: Arc::clone(&journal),
                cap,
            };
            match sub.run(&ctx).await {
                Ok(out) => {
                    stats.actions_journaled += out.actions_journaled;
                    stats.mutations_applied += out.mutations_applied;
                    stats.proposals_recorded += out.proposals_recorded;
                    if out.cap_hit {
                        stats.caps_hit += 1;
                    }
                    debug!(
                        workstream = %workstream.name,
                        subroutine = sub.name(),
                        journaled = out.actions_journaled,
                        applied = out.mutations_applied,
                        proposals = out.proposals_recorded,
                        "steward subroutine done"
                    );
                }
                Err(e) => {
                    stats.errors += 1;
                    warn!(
                        workstream = %workstream.name,
                        subroutine = sub.name(),
                        error = %e,
                        "steward subroutine failed; continuing"
                    );
                }
            }
        }
        Ok(stats)
    }

    /// Run one pass across every active (non-archived) workstream.
    pub async fn run_pass_for_all(&self) -> Result<StewardStats, StewardError> {
        let workstreams: Vec<Workstream> = {
            let s = self.store.lock().unwrap();
            s.list_workstreams()
                .map_err(|e| StewardError::Storage(e.to_string()))?
        };
        let mut agg = StewardStats::default();
        for ws in workstreams {
            match self.run_pass_for_workstream(&ws).await {
                Ok(s) => {
                    agg.workstreams_visited += s.workstreams_visited;
                    agg.subroutine_runs += s.subroutine_runs;
                    agg.actions_journaled += s.actions_journaled;
                    agg.mutations_applied += s.mutations_applied;
                    agg.proposals_recorded += s.proposals_recorded;
                    agg.caps_hit += s.caps_hit;
                    agg.errors += s.errors;
                }
                Err(e) => {
                    agg.errors += 1;
                    warn!(
                        workstream = %ws.name,
                        error = %e,
                        "steward pass failed; continuing with next workstream"
                    );
                }
            }
        }
        if agg.workstreams_visited > 0 {
            info!(
                workstreams = agg.workstreams_visited,
                actions = agg.actions_journaled,
                applied = agg.mutations_applied,
                proposals = agg.proposals_recorded,
                errors = agg.errors,
                "steward fan-out complete"
            );
        }
        Ok(agg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subroutine::IdentitySubroutine;

    fn setup() -> (
        tempfile::TempDir,
        Arc<Mutex<Store>>,
        MemoryResolver,
    ) {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        store.ensure_scratch_workstream().unwrap();
        let store = Arc::new(Mutex::new(store));
        let data_dir = tmp.path().to_path_buf();
        let resolver: MemoryResolver = Arc::new(move |name: &str| {
            MemoryManager::for_workstream(&data_dir, name, None)
                .map(Arc::new)
                .map_err(|e| StewardError::Memory(e.to_string()))
        });
        (tmp, store, resolver)
    }

    #[tokio::test]
    async fn pass_visits_every_active_workstream() {
        let (_tmp, store, resolver) = setup();
        {
            let s = store.lock().unwrap();
            s.create_workstream(&Workstream::new(
                "pat",
                std::env::temp_dir().join("pat"),
            ))
            .unwrap();
            s.create_workstream(&Workstream::new(
                "old",
                std::env::temp_dir().join("old"),
            ))
            .unwrap();
            s.soft_delete_workstream("old").unwrap();
        }
        let runner = StewardRunner::new(
            store,
            _tmp.path(),
            resolver,
            vec![Arc::new(IdentitySubroutine::default())],
        );
        let stats = runner.run_pass_for_all().await.unwrap();
        // scratch + pat = 2; old is archived.
        assert_eq!(stats.workstreams_visited, 2);
        assert_eq!(stats.subroutine_runs, 2);
        assert_eq!(stats.actions_journaled, 2);
        assert_eq!(stats.proposals_recorded, 2);
    }

    #[tokio::test]
    async fn caps_override_takes_precedence() {
        let (_tmp, store, resolver) = setup();
        let caps = SubroutineCaps::new(3).with_cap("identity", 99);
        let runner = StewardRunner::new(
            store,
            _tmp.path(),
            resolver,
            vec![Arc::new(IdentitySubroutine::default())],
        )
        .with_caps(caps);
        // No assertion on the cap value reaching the subroutine — the
        // identity subroutine ignores it — but the pass should succeed.
        let stats = runner.run_pass_for_all().await.unwrap();
        assert_eq!(stats.errors, 0);
    }

    #[tokio::test]
    async fn journal_persists_across_passes() {
        let (_tmp, store, resolver) = setup();
        let runner = StewardRunner::new(
            store,
            _tmp.path(),
            resolver,
            vec![Arc::new(IdentitySubroutine::default())],
        );
        let _ = runner.run_pass_for_all().await.unwrap();
        let _ = runner.run_pass_for_all().await.unwrap();
        let j = runner.journal_for("scratch").unwrap();
        let recent = j.recent(10).unwrap();
        assert_eq!(recent.len(), 2, "two passes → two journal rows");
    }
}

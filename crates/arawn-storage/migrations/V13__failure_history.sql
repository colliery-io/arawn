-- ARAWN-T-0477 (I-0068 P2-1b) — persisted background-job failure history.
--
-- Before this, a failed ceremony dispatch left no row (just a vanished
-- `warn!` line) and steward subroutine errors were dropped after bumping a
-- stat. These two append-only tables make "why is there no tablet today?"
-- and "did the steward fail last night?" answerable, and back the `/status`
-- health surface (ARAWN-T-0476). Both are pruned to a bounded row count by
-- the writers, so they can't grow without limit.

-- One row per ceremony dispatch (success, skip, or error). `outcome` is the
-- terminal state; `error` carries the failure text when outcome = 'error'.
CREATE TABLE ceremony_run_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    kind        TEXT NOT NULL,                 -- daily | weekly | retro | ...
    period_key  TEXT NOT NULL,                 -- the dispatched period
    outcome     TEXT NOT NULL                  -- ok | skipped | error
                CHECK (outcome IN ('ok', 'skipped', 'error')),
    error       TEXT,                          -- failure text when outcome = 'error'
    ran_at      TEXT NOT NULL                  -- RFC3339
);

CREATE INDEX ceremony_run_history_kind_idx ON ceremony_run_history(kind, id);

-- One row per failed steward subroutine pass. The journal stays
-- success-only; this is the error side-channel.
CREATE TABLE steward_error_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    lens_name   TEXT NOT NULL,
    subroutine  TEXT NOT NULL,
    error       TEXT NOT NULL,
    failed_at   TEXT NOT NULL                  -- RFC3339
);

CREATE INDEX steward_error_log_at_idx ON steward_error_log(id);

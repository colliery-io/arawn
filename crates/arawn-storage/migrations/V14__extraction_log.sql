-- ARAWN-T-0484 (I-0068) — per-row extraction log.
--
-- Records what the extractor decided for each (lens, projection row): the
-- run id, the terminal outcome, and the classify reason — so an operator can
-- ask "why is there (no) signal from this row?" (signal_explain), and so a
-- row can be marked `dismissed` and skipped on future passes (signal_dismiss)
-- without deleting it from the source feed.
--
-- One row per (lens_name, projection_id); the latest extraction overwrites.
-- `outcome` distinguishes "extracted empty" from "skipped" (out of scope) —
-- the determinism distinction deferred from T-0482.

CREATE TABLE extraction_log (
    lens_name      TEXT NOT NULL,
    projection_id  TEXT NOT NULL,
    run_id         TEXT NOT NULL,              -- per-pass uuid (provenance)
    outcome        TEXT NOT NULL               -- ok | empty | skipped
                   CHECK (outcome IN ('ok', 'empty', 'skipped')),
    reason         TEXT,                        -- classify rationale, when any
    dismissed      INTEGER NOT NULL DEFAULT 0,  -- 1 = don't re-extract this row
    updated_at     TEXT NOT NULL,               -- RFC3339
    PRIMARY KEY (lens_name, projection_id)
);

CREATE INDEX extraction_log_lens_idx ON extraction_log(lens_name);

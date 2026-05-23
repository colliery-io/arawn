-- ARAWN-T-0367 — runtime-mutable per-ceremony config.
--
-- Today the only inhabitant is the retro plugin's `cadence` knob
-- (weekly/biweekly/monthly) and its `cadence_anchor` (the first
-- run date for biweekly/monthly cycles). The table is shaped to
-- carry future cadence-bearing ceremonies without further
-- migrations: anything keyed by `(kind, key)` with a TEXT value.

CREATE TABLE ceremony_config (
    kind  TEXT NOT NULL,
    key   TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (kind, key)
);

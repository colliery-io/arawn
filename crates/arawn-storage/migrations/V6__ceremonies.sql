-- I-0043 ceremony engine: the full schema for the ceremony plugin
-- system (engine + retro plugin shipped together; daily + weekly
-- inherit this schema without further migrations).
--
-- One transaction owns every row written during a ceremony run, so
-- a partial failure rolls back the whole tablet. The two-write-path
-- citation contract (T-0282) is enforced in Rust; `citation_id` is
-- NULL-able here because user-write items legitimately have no
-- source.

-- Top-level tablet. One row per (kind, period_key). period_key is
-- the date for daily and the ISO week (e.g. `2026-W20`) for weekly
-- and retro.
CREATE TABLE ceremony_tablets (
    id                       TEXT PRIMARY KEY,            -- e.g. daily-2026-05-15
    kind                     TEXT NOT NULL,               -- daily | weekly | retro | ...
    period_key               TEXT NOT NULL,
    generated_at             TEXT NOT NULL,               -- RFC3339
    status                   TEXT NOT NULL,               -- open | reviewed | unreviewed | archived
    workstreams_scanned      TEXT NOT NULL,               -- JSON array of workstream names
    priorities_confirmed_at  TEXT,                        -- weekly only; null on daily/retro
    UNIQUE(kind, period_key)
);

CREATE INDEX ceremony_tablets_kind_idx     ON ceremony_tablets(kind);
CREATE INDEX ceremony_tablets_status_idx   ON ceremony_tablets(status);

-- Section headings within a tablet. Declared up-front by each
-- plugin (calendar / attention / proposals / todos / patterns /
-- what_happened / priorities / ...) so renderers know the canonical
-- order.
CREATE TABLE ceremony_sections (
    tablet_id    TEXT NOT NULL REFERENCES ceremony_tablets(id) ON DELETE CASCADE,
    section_key  TEXT NOT NULL,
    ordinal      INTEGER NOT NULL,
    title        TEXT NOT NULL,
    PRIMARY KEY (tablet_id, section_key)
);

-- One row per item displayed in a section. `citation_id` is the id
-- of the source row (signal_id / event_id / proposal_id /
-- ceremony_patterns_detected.id) the LLM cited. NULL only when the
-- item was written via the user-write path (freeform diary entries,
-- user-added todos). T-0282 enforces this in Rust at the write site;
-- the schema permits NULL so the user-write path doesn't need a
-- placeholder citation.
CREATE TABLE ceremony_items (
    id           TEXT PRIMARY KEY,
    tablet_id    TEXT NOT NULL REFERENCES ceremony_tablets(id) ON DELETE CASCADE,
    section_key  TEXT NOT NULL,
    ordinal      INTEGER NOT NULL,
    kind         TEXT NOT NULL,        -- calendar_event | attention | proposal | todo | pattern | priority | freeform
    body         TEXT NOT NULL,        -- JSON, structured per item kind
    citation_id  TEXT,                 -- nullable; set on every composed item, null on user items
    done_at      TEXT,                 -- todos only
    created_at   TEXT NOT NULL
);

CREATE INDEX ceremony_items_tablet_idx   ON ceremony_items(tablet_id);
CREATE INDEX ceremony_items_section_idx  ON ceremony_items(tablet_id, section_key, ordinal);

-- Todos that persist across daily tablets. Each daily generation
-- pulls un-done rows here and links them into the new tablet's
-- todo section via ceremony_items.
CREATE TABLE ceremony_todos_rolling (
    todo_id              TEXT PRIMARY KEY,
    body                 TEXT NOT NULL,
    origin_tablet_id     TEXT NOT NULL REFERENCES ceremony_tablets(id) ON DELETE CASCADE,
    created_at           TEXT NOT NULL,
    done_at              TEXT,
    last_seen_tablet_id  TEXT NOT NULL REFERENCES ceremony_tablets(id) ON DELETE CASCADE
);

CREATE INDEX ceremony_todos_done_idx        ON ceremony_todos_rolling(done_at);
CREATE INDEX ceremony_todos_last_seen_idx   ON ceremony_todos_rolling(last_seen_tablet_id);

-- Priorities written by the Monday weekly tablet. Unconfirmed
-- candidates land here as rows; the Monday confirm flow flips
-- confirmed_at on the kept ones and deletes the rest.
CREATE TABLE ceremony_priorities (
    id            TEXT PRIMARY KEY,
    tablet_id     TEXT NOT NULL REFERENCES ceremony_tablets(id) ON DELETE CASCADE,
    body          TEXT NOT NULL,
    rationale     TEXT NOT NULL,
    citation_id   TEXT,                  -- source row in feeds/signals/last_retro
    confirmed_at  TEXT,                  -- null until user confirms
    done_at       TEXT,
    ordinal       INTEGER NOT NULL
);

CREATE INDEX ceremony_priorities_tablet_idx     ON ceremony_priorities(tablet_id);
CREATE INDEX ceremony_priorities_confirmed_idx  ON ceremony_priorities(confirmed_at);

-- End-of-week aggregation across feeds/calendar/tablets. Generic —
-- consumed by retro's pattern detectors today; any future ceremony
-- can read it. (workstream, metric_key) is the slicing dimension;
-- value is whatever numeric form the metric needs.
CREATE TABLE ceremony_activity_rollup (
    iso_week    TEXT NOT NULL,
    workstream  TEXT NOT NULL,
    metric_key  TEXT NOT NULL,           -- emails_sent | slack_threads_participated | meetings_attended | deep_work_hours | signals_extracted_count | steward_proposals_accepted | steward_proposals_rejected | ...
    value       REAL NOT NULL,
    PRIMARY KEY (iso_week, workstream, metric_key)
);

CREATE INDEX ceremony_activity_rollup_week_idx  ON ceremony_activity_rollup(iso_week);

-- Pattern rows surface in the retro's "patterns" section. Composed
-- items that present a pattern carry the pattern row's id as their
-- citation_id; the pattern row's payload carries its own source-row
-- citations. Two-level citation chain.
CREATE TABLE ceremony_patterns_detected (
    id                   TEXT PRIMARY KEY,
    iso_week             TEXT NOT NULL,
    pattern_key          TEXT NOT NULL,
    magnitude            REAL NOT NULL,
    payload              TEXT NOT NULL,           -- JSON: cited source rows + comparison window
    surfaced_in_retro    BOOLEAN NOT NULL DEFAULT 0
);

CREATE INDEX ceremony_patterns_week_idx  ON ceremony_patterns_detected(iso_week);
CREATE INDEX ceremony_patterns_key_idx   ON ceremony_patterns_detected(pattern_key);

-- User diary content. One row per retro tablet (PK by tablet_id);
-- absence of a row means the user didn't write a diary for that
-- week. word_count is computed at write time.
CREATE TABLE ceremony_diary (
    tablet_id   TEXT PRIMARY KEY REFERENCES ceremony_tablets(id) ON DELETE CASCADE,
    body        TEXT NOT NULL,
    written_at  TEXT NOT NULL,
    word_count  INTEGER NOT NULL
);

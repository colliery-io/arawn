-- I-0040 phase 4: per-workstream extractor cursors.
-- One row per (workstream, feed_type). Tracks the highest source_ts
-- the extractor has processed for that workstream's view of that
-- feed_type. Reactive trigger: when feed dispatch writes new
-- projection rows, the extractor advances per-workstream cursors.

CREATE TABLE extractor_cursors (
    workstream_name    TEXT NOT NULL,
    feed_type          TEXT NOT NULL,
    last_source_ts     TEXT NOT NULL DEFAULT '',   -- RFC3339; empty = never run
    last_processed_at  TEXT NOT NULL,
    PRIMARY KEY (workstream_name, feed_type)
);

CREATE INDEX extractor_cursors_workstream_idx
    ON extractor_cursors(workstream_name);

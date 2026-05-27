-- I-0040 phase 4: per-lens extractor cursors.
-- One row per (lens, feed_type). Tracks the highest source_ts
-- the extractor has processed for that lens's view of that
-- feed_type. Reactive trigger: when feed dispatch writes new
-- projection rows, the extractor advances per-lens cursors.

CREATE TABLE extractor_cursors (
    lens_name    TEXT NOT NULL,
    feed_type          TEXT NOT NULL,
    last_source_ts     TEXT NOT NULL DEFAULT '',   -- RFC3339; empty = never run
    last_processed_at  TEXT NOT NULL,
    PRIMARY KEY (lens_name, feed_type)
);

CREATE INDEX extractor_cursors_lens_idx
    ON extractor_cursors(lens_name);

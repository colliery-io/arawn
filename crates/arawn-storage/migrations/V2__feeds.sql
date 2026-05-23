-- I-0039: continual data feeds. One row per configured feed.
-- cloacina cron schedules are derived from this table at boot + on /watch.
CREATE TABLE feeds (
    id          TEXT PRIMARY KEY,         -- stable slug, e.g. "slack-design-archive"
    template    TEXT NOT NULL,            -- "<provider>/<template>" registry key
    params      TEXT NOT NULL,            -- JSON payload (template-specific shape)
    cadence     TEXT NOT NULL,            -- cron expression (UTC)
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX feeds_template_idx ON feeds(template);
CREATE INDEX feeds_enabled_idx ON feeds(enabled);

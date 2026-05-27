-- I-0049 / T-0309 — generic todos table.
--
-- Single source of truth for done_at across user todos, weekly
-- priorities, and rollover todos. `kind` is a free-form text
-- discriminant: 'user' for chat-driven reminders, 'weekly_priority'
-- for confirmed weekly priorities, 'rollover' for daily todos that
-- carry across days. Future kinds (linear, github, jira, ...) drop
-- in without schema changes.
--
-- `attrs` carries kind-specific JSON payload — tablet_id, ordinal,
-- confirmed_at, citation_id, origin_tablet_id, last_seen_tablet_id,
-- etc. Not indexed; selection happens by kind + lens + done_at.
--
-- Backfill from ceremony_priorities + ceremony_todos_rolling ships
-- in T-0311 (separate migration). Schema cutover that turns ceremony
-- tables into FK pointers ships in T-0312.

CREATE TABLE todos (
    id           TEXT PRIMARY KEY,
    body         TEXT NOT NULL,
    rationale    TEXT,
    kind         TEXT NOT NULL,
    lens   TEXT,
    created_at   TEXT NOT NULL,
    due_at       TEXT,
    done_at      TEXT,
    archived_at  TEXT,
    attrs        TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX todos_kind_idx        ON todos(kind);
CREATE INDEX todos_done_idx        ON todos(done_at);
CREATE INDEX todos_lens_idx  ON todos(lens);

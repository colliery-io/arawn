-- I-0049 / T-0312 — ceremony schema cutover.
--
-- Make `todos` the canonical source of body/rationale/done_at across
-- the ceremony surface. After this migration:
--
-- * `ceremony_priorities` is a thin index — (tablet_id, ordinal,
--   confirmed_at) + `todo_id` FK to todos. body/rationale/citation_id/
--   done_at have all moved to the linked todo row (citation_id sits
--   in `todos.attrs.citation_id`).
--
-- * `ceremony_todos_rolling` retires as a table and re-appears as a
--   read-only view over `todos WHERE kind='rollover'`. The view
--   preserves the original column shape (todo_id/body/origin_tablet_id/
--   created_at/done_at/last_seen_tablet_id) so SELECT-shaped readers
--   in retro_detectors continue to work unchanged. INSERTs into
--   `ceremony_todos_rolling` no longer compile in SQLite (views
--   aren't writable here) — the plugin/service layer has been
--   rewritten in this commit to route writes through TodoService.
--
-- * `ceremony_items.todo_id` is an optional FK so user-todo items can
--   link back to their canonical todo (kind='todo' or kind='priority').
--
-- The recreate-and-rename dance for ceremony_priorities is needed
-- because SQLite can't tighten a column to NOT NULL after the fact.
-- Indexes are re-created against the new table.

----------------------------------------------------------------------
-- ceremony_priorities — recreate without the duplicated columns
----------------------------------------------------------------------

CREATE TABLE ceremony_priorities_new (
    id            TEXT PRIMARY KEY,
    tablet_id     TEXT NOT NULL REFERENCES ceremony_tablets(id) ON DELETE CASCADE,
    todo_id       TEXT NOT NULL REFERENCES todos(id) ON DELETE CASCADE,
    confirmed_at  TEXT,
    ordinal       INTEGER NOT NULL
);

-- Backfill copies every existing row. T-0311's V8 already inserted a
-- matching `wp:<id>` todo row per priority; the new row points at it.
INSERT INTO ceremony_priorities_new (id, tablet_id, todo_id, confirmed_at, ordinal)
SELECT id, tablet_id, 'wp:' || id, confirmed_at, ordinal
FROM ceremony_priorities;

DROP TABLE ceremony_priorities;
ALTER TABLE ceremony_priorities_new RENAME TO ceremony_priorities;

CREATE INDEX ceremony_priorities_tablet_idx     ON ceremony_priorities(tablet_id);
CREATE INDEX ceremony_priorities_confirmed_idx  ON ceremony_priorities(confirmed_at);
CREATE INDEX ceremony_priorities_todo_idx       ON ceremony_priorities(todo_id);

----------------------------------------------------------------------
-- ceremony_todos_rolling — drop, replace with view
----------------------------------------------------------------------

DROP INDEX IF EXISTS ceremony_todos_done_idx;
DROP INDEX IF EXISTS ceremony_todos_last_seen_idx;
DROP TABLE ceremony_todos_rolling;

CREATE VIEW ceremony_todos_rolling AS
SELECT
    id                                           AS todo_id,
    body                                         AS body,
    json_extract(attrs, '$.origin_tablet_id')    AS origin_tablet_id,
    created_at                                   AS created_at,
    done_at                                      AS done_at,
    json_extract(attrs, '$.last_seen_tablet_id') AS last_seen_tablet_id
FROM todos
WHERE kind = 'rollover' AND archived_at IS NULL;

----------------------------------------------------------------------
-- ceremony_items.todo_id — optional FK back to the canonical todo
----------------------------------------------------------------------

ALTER TABLE ceremony_items
    ADD COLUMN todo_id TEXT REFERENCES todos(id) ON DELETE SET NULL;

-- Persist Session.workstream_name alongside workstream_id.
-- The id is the FK; the name is what memory routing reads at runtime
-- and what /workstream switch sets. Storing both means session
-- resumption can immediately re-establish the active workstream
-- without a JOIN.

ALTER TABLE sessions ADD COLUMN workstream_name TEXT NOT NULL DEFAULT 'scratch';

-- Backfill: rows with workstream_id NULL stay 'scratch' (already the
-- default). Rows with workstream_id pull the name from workstreams.
UPDATE sessions
   SET workstream_name = (
       SELECT name FROM workstreams WHERE workstreams.id = sessions.workstream_id
   )
 WHERE workstream_id IS NOT NULL
   AND EXISTS (SELECT 1 FROM workstreams WHERE workstreams.id = sessions.workstream_id);

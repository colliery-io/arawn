-- Persist Session.lens_name alongside lens_id.
-- The id is the FK; the name is what memory routing reads at runtime
-- and what /lens switch sets. Storing both means session
-- resumption can immediately re-establish the active lens
-- without a JOIN.

ALTER TABLE sessions ADD COLUMN lens_name TEXT NOT NULL DEFAULT 'scratch';

-- Backfill: rows with lens_id NULL stay 'scratch' (already the
-- default). Rows with lens_id pull the name from lenses.
UPDATE sessions
   SET lens_name = (
       SELECT name FROM lenses WHERE lenses.id = sessions.lens_id
   )
 WHERE lens_id IS NOT NULL
   AND EXISTS (SELECT 1 FROM lenses WHERE lenses.id = sessions.lens_id);

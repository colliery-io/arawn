-- ARAWN-T-0248: extend the workstream registry.
--
-- V1's `workstreams` table had only (id, name, root_dir, created_at).
-- This migration adds the columns the Phase 3 registry needs:
--   display_name, description, bindings (JSON array), archived, updated_at.
--
-- The `scratch` row is inserted at runtime by WorkstreamRegistry::ensure_scratch
-- so the root_dir picks up the current $HOME instead of being baked in.

ALTER TABLE workstreams ADD COLUMN display_name TEXT NOT NULL DEFAULT '';
ALTER TABLE workstreams ADD COLUMN description  TEXT NOT NULL DEFAULT '';
ALTER TABLE workstreams ADD COLUMN bindings     TEXT NOT NULL DEFAULT '[]';
ALTER TABLE workstreams ADD COLUMN archived     INTEGER NOT NULL DEFAULT 0;
ALTER TABLE workstreams ADD COLUMN updated_at   TEXT NOT NULL DEFAULT '';

-- Backfill: existing rows get display_name = name and updated_at = created_at.
UPDATE workstreams SET display_name = name WHERE display_name = '';
UPDATE workstreams SET updated_at = created_at WHERE updated_at = '';

-- `name` is the primary addressing key for the user; enforce uniqueness.
CREATE UNIQUE INDEX workstreams_name_uidx ON workstreams(name);
CREATE INDEX workstreams_archived_idx ON workstreams(archived);
CREATE INDEX workstreams_updated_at_idx ON workstreams(updated_at);

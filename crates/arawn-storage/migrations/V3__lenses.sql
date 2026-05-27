-- ARAWN-T-0248: extend the lens registry.
--
-- V1's `lenses` table had only (id, name, root_dir, created_at).
-- This migration adds the columns the Phase 3 registry needs:
--   display_name, description, bindings (JSON array), archived, updated_at.
--
-- The `scratch` row is inserted at runtime by LensRegistry::ensure_scratch
-- so the root_dir picks up the current $HOME instead of being baked in.

ALTER TABLE lenses ADD COLUMN display_name TEXT NOT NULL DEFAULT '';
ALTER TABLE lenses ADD COLUMN description  TEXT NOT NULL DEFAULT '';
ALTER TABLE lenses ADD COLUMN bindings     TEXT NOT NULL DEFAULT '[]';
ALTER TABLE lenses ADD COLUMN archived     INTEGER NOT NULL DEFAULT 0;
ALTER TABLE lenses ADD COLUMN updated_at   TEXT NOT NULL DEFAULT '';

-- Backfill: existing rows get display_name = name and updated_at = created_at.
UPDATE lenses SET display_name = name WHERE display_name = '';
UPDATE lenses SET updated_at = created_at WHERE updated_at = '';

-- `name` is the primary addressing key for the user; enforce uniqueness.
CREATE UNIQUE INDEX lenses_name_uidx ON lenses(name);
CREATE INDEX lenses_archived_idx ON lenses(archived);
CREATE INDEX lenses_updated_at_idx ON lenses(updated_at);

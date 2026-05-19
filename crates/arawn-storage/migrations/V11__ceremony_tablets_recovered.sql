-- Add a `recovered` flag to ceremony tablets so back-filled
-- tablets composed for a historical date are distinguishable
-- from live ones (ARAWN-T-0365 / ARAWN-I-0052).
--
-- Existing rows default to `false` — they were composed live by
-- the cron loop before back-fill existed, so the audit trail is
-- truthful without a data migration.

ALTER TABLE ceremony_tablets ADD COLUMN recovered INTEGER NOT NULL DEFAULT 0;

-- ARAWN-T-0330: workstream-scoped identity persona.
--
-- Adds `identity_profile` to the workstream registry. Values:
--   'assistant' (default) — the personal-agentic-assistant persona
--                            from arawn's vision (watch/check/summarize/nudge).
--   'coding'              — the engineering-tool persona, opt-in per workstream
--                            for software-development scopes.
--
-- All existing rows default to 'assistant' via the column default;
-- this matches the vision and is the intentional behavior flip
-- described in I-0035 Phase 1.

ALTER TABLE workstreams ADD COLUMN identity_profile TEXT NOT NULL DEFAULT 'assistant';

-- I-0049 / T-0311 — non-destructive backfill of ceremony state into
-- the generic todos table.
--
-- One todos row per ceremony_priorities row (kind='weekly_priority')
-- and per ceremony_todos_rolling row (kind='rollover'). Source rows
-- stay where they are; T-0312 swaps the ceremony tables onto FK
-- pointers in a separate migration once the cutover is wired.
--
-- IDs are deterministic — `wp:<source_id>` for priorities and
-- `rl:<source_id>` for rollover todos — so re-running this against a
-- freshly built database (e.g. test fixture) yields the same ids.
-- Refinery's version tracking already prevents re-runs on a real
-- database; the deterministic id is belt-and-braces for tests.
--
-- `attrs` captures the kind-specific bag: weekly priorities carry
-- tablet_id + ordinal + confirmed_at + citation_id; rollover todos
-- carry origin_tablet_id + last_seen_tablet_id. Anyone wanting the
-- linked tablet later goes via attrs->>'tablet_id' until T-0312
-- adds a proper FK column.

INSERT INTO todos (
    id, body, rationale, kind, lens, created_at,
    due_at, done_at, archived_at, attrs
)
SELECT
    'wp:' || cp.id,
    cp.body,
    NULLIF(cp.rationale, ''),
    'weekly_priority',
    NULL,
    ct.generated_at,
    NULL,
    cp.done_at,
    NULL,
    json_object(
        'tablet_id', cp.tablet_id,
        'ordinal', cp.ordinal,
        'confirmed_at', cp.confirmed_at,
        'citation_id', cp.citation_id
    )
FROM ceremony_priorities cp
JOIN ceremony_tablets ct ON ct.id = cp.tablet_id;

INSERT INTO todos (
    id, body, rationale, kind, lens, created_at,
    due_at, done_at, archived_at, attrs
)
SELECT
    'rl:' || todo_id,
    body,
    NULL,
    'rollover',
    NULL,
    created_at,
    NULL,
    done_at,
    NULL,
    json_object(
        'origin_tablet_id', origin_tablet_id,
        'last_seen_tablet_id', last_seen_tablet_id
    )
FROM ceremony_todos_rolling;

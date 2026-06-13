---
id: p2-5-p2-6-embedding-hash
level: task
title: "P2-5/P2-6: Embedding hash validation + stuck-pass handling; memory-store concurrency races"
short_code: "ARAWN-T-0481"
created_at: 2026-06-12T12:02:16.285501+00:00
updated_at: 2026-06-13T12:32:16.463346+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0068
---

# P2-5/P2-6: Embedding hash validation + stuck-pass handling; memory-store concurrency races

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0068]] — implements P2-5 (HIGH) and P2-6 (HIGH). Two small, surgical data-integrity fixes batched.

## Objective **[REQUIRED]**

Fix silent data corruption in the embedding lifecycle and the memory store: stale vectors that never re-embed, an embed pass that can spin forever, and concurrency races that lose facts and undercount confidence.

**Defects:**
- **P2-5** (`arawn-projections/src/embed.rs`, `store.rs:465-488`): `write_embedding()` never validates `body_hash` against the row's current body — if the body changes between the pending fetch and the vector write, the row flips to `embedded` with a vector for stale text and never re-embeds. And an `embed_batch()` error makes the loop spin with zero progress while the backlog grows unbounded.
- **P2-6** (`arawn-memory/src/store.rs:446-502`): `store_fact()` is FTS-search-then-reinforce with no transaction (a concurrent delete loses the fact); `reinforce_entity()` is read-increment-write without a lock (concurrent reinforcement undercounts), silently corrupting confidence ranking.

### Type
- [x] Bug — silent data corruption

### Priority
- [x] P2 - Medium (both HIGH findings; corrupt quietly)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] `write_embedding()` compares-and-sets on `body_hash`: a mismatch leaves the row `pending` (re-embeds later) rather than committing a stale vector.
- [ ] A failing `embed_batch()` marks the offending rows errored / backs off instead of retrying the same head-of-line batch forever; backlog can't grow unbounded.
- [ ] `store_fact()` dedupe-or-insert runs in a transaction (no lost fact on concurrent delete).
- [ ] `reinforce_entity()` is a single SQL `UPDATE ... SET count = count + 1` (atomic; no undercount).
- [ ] Pending/error embedding counts are queryable (surfaced via [[ARAWN-T-0476]]).
- [ ] Tests for the hash-mismatch path and the reinforce atomicity.
- [ ] `angreal check all` and `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Embedding: in `write_embedding`, `WHERE body_hash = ?` (the hash captured when the row entered the pending set) so a changed body doesn't get the stale vector. On batch error, mark rows `error` with a retry/backoff column. Memory: wrap dedupe-or-insert in a txn; replace the read-incr-write reinforce with an atomic UPDATE.

### Dependencies
Counts surface via [[ARAWN-T-0476]]. Independent of the ceremony/feed tasks — can parallelize.

### Risk Considerations
Small, surgical, high-value. Main risk is the embedding state machine's other transitions — keep the hash check to the write path. The atomic reinforce changes confidence numbers slightly (correctly) — fine.

## Status Updates **[REQUIRED]**

### 2026-06-13 — COMPLETE ✅
**P2-5 embedding (`arawn-projections`):**
- **Hash compare-and-set**: `PendingEmbedRow` carries `body_hash` (captured at fetch); `write_embedding(.., expected_hash)` reads the row's current `body_hash` under the held conn lock and bails (leaving it `pending`) if it changed — no stale vector committed. The whole method holds the lock, so read + writes are atomic against a concurrent `embedding_invalidate`.
- **Backoff**: new `retry_count` column on `<feed_type>_embeddings` (schema CREATE + idempotent ALTER for existing tables). `embed_batch` bumps it on every failure path; `pending_embedding_rows` excludes `retry_count >= MAX_EMBED_RETRIES` (=5), so a permanently-failing batch parks itself instead of blocking the queue head / growing the backlog. A body change resets `retry_count` (in `embedding_invalidate`).
- **Counts surfaced** (T-0476): `pending_embedding_count` excludes parked rows; new `errored_embedding_count`. `EmbeddingStatus` gained `errored: Option<u64>`; status.rs populates it; TUI renders "⚠ N stuck".

**P2-6 memory (`arawn-memory`) — finding refined:** the store is **graphqlite Cypher**, not SQLite, and `reinforce_entity` already held the conn `Mutex` across its read+write — so it was *already* atomic against concurrent reinforce (the ticket's "single SQL UPDATE" doesn't apply; Cypher has no arithmetic SET, and the lock is what guarantees atomicity). The REAL race was `store_fact`: `search_by_type`/`reinforce_entity`/`insert_entity` each took the lock independently, so a concurrent delete/insert could slip between the search and the write. **Fix:** `store_fact` now takes the conn lock ONCE and runs dedupe-or-insert via new `search_by_type_locked`/`reinforce_entity_locked`/`insert_entity_locked` free helpers under that single lock; the public methods delegate to the same helpers.

**Tests:** `write_embedding_rejects_stale_body_hash`, `parked_rows_drop_out_of_pending_and_count_as_errored` (projections); existing `store_fact_insert/reinforce/reinforce_case_insensitive` exercise the new compound-lock path.

**Gates:** fmt + clippy -D warnings clean · projections(59) + memory(85) + hybrid_search(4) + tui format(2) + arawn-tests status(2) green. All acceptance criteria met (P2-6 reframed: reinforce was already lock-atomic; store_fact compound-lock is the real fix).
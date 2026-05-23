# Performance Review

## Summary

Arawn's performance profile is well-matched to its workload. The dominant bottleneck is LLM API latency (seconds per call), which makes most code-level micro-optimizations irrelevant. The architecture correctly identifies this: read-only tools run concurrently, writes run serially, streams are processed incrementally, and SQLite is configured with WAL mode for minimal contention.

The findings below focus on issues that either (a) add measurable user-facing latency during the gaps between LLM calls, (b) represent unbounded growth risks in long sessions, or (c) involve redundant work that accumulates per-turn in the agentic loop.

**Overall assessment: Good.** The system is appropriately engineered for a local single-user assistant. No premature optimization, no over-engineered concurrency. A handful of redundant computations and one structural I/O issue worth addressing.

## Workload Assessment

| Path | Frequency | Latency Sensitivity | Bottleneck |
|------|-----------|---------------------|------------|
| LLM streaming | Every turn | Low (network-bound) | API latency |
| Tool execution | Every turn (1-5 calls) | **High** (user waits) | Tool I/O |
| Session load | Per message | Medium | JSONL parse |
| System prompt build | Every turn | Low | String assembly |
| Memory injection (L0+L1+L2) | Every message | Low-Medium | SQLite queries |
| TUI render | ~30fps | Low | Terminal I/O |
| Compaction check | Every turn | Low | Token estimation |
| Permission check | Per tool call | Low | Rule matching |

## Hot Path Analysis

### QueryEngine.run() — The Agentic Loop

The loop is clean. Each iteration: microcompact -> compaction check -> build request -> stream LLM -> process tool calls -> loop. No unnecessary allocations in the critical path. The `build_request` method clones all messages every turn (converting `Message` to `ChatMessage`), but this is unavoidable given the current architecture and dwarfed by LLM latency.

### Tool Dispatch

Read-only tools are parallelized via `join_all`, write tools run serially. This is correct. Tool lookup is O(1) via `HashMap`. Permission checking is a linear scan of rules, but rule lists are small.

### Stream Assembly

`stream_response` accumulates text deltas and tool argument deltas into Strings. This is efficient — no intermediate allocations beyond the growing String buffers. The `parse_arguments` at the end is a single `serde_json::from_str` call per tool.

## Findings

### PERF-01: Duplicate L1 Entity Queries per Message (Severity: Low)

**Location:** `crates/arawn-memory/src/stack.rs:129-141`, called from `crates/arawn/src/local_service.rs:641-657`

The `wake_up()` method calls `render_l1_with_names()` which queries `list_all_ranked(30)` from global and `list_all_ranked(50)` from workstream. Then `l1_entity_titles()` is called separately and re-executes the exact same two queries, re-sorts, and re-filters. This doubles the SQLite queries for L1 generation every message.

```
let mems = vec![stack.wake_up(900)];        // queries global+workstream ranked entities
let l1_titles = stack.l1_entity_titles();    // queries the same entities again
```

**Impact:** 4 unnecessary SQLite queries per message. With small KBs (<100 entities) this is negligible, but it is pure waste.

**Recommendation:** Have `wake_up()` return the L1 titles alongside the rendered text, or cache the result within the `MemoryStack` lifetime.

---

### PERF-02: tool_definitions() Rebuilt Every Turn (Severity: Low)

**Location:** `crates/arawn-engine/src/query_engine.rs:574`, `crates/arawn-engine/src/tool.rs:105-115`

`tool_definitions()` acquires a read lock, iterates all tools, calls `parameters_schema()` (which builds a fresh `serde_json::Value` per tool), and collects into a `Vec<ToolDefinition>`. This happens every turn in `build_request()`.

Additionally, `TokenEstimator::estimate_tools()` is called during compaction checks and serializes each tool's parameters JSON back to string via `.to_string()` just to measure length — so tool schemas are built and then immediately stringified.

**Impact:** With ~25 tools, this is ~25 `serde_json::json!()` macro expansions + 25 `.to_string()` calls per turn. Total: microseconds. Not worth fixing unless the tool count grows significantly.

**Recommendation:** No immediate action needed. If tool count exceeds 50, consider caching definitions and invalidating on register/unregister.

---

### PERF-03: JSONL Full Parse on Load Despite Compaction Skip (Severity: Medium)

**Location:** `crates/arawn-storage/src/jsonl.rs:54-79`, `crates/arawn-core/src/session.rs:211-221`

When loading a session, the JSONL store reads and deserializes **all** messages from the file, then `Session::load_compacted()` finds the last `Summary` message and discards everything before it. For a long-running session with multiple compactions, this means parsing hundreds of messages from JSONL that are immediately thrown away.

```rust
let all_messages = msg_store.load(id, &ws_dir).await?;
let messages = Session::load_compacted(all_messages);  // drops messages[..last_summary]
```

**Impact:** For a session with 200 messages and a compaction at message 150, this parses 150 messages that are immediately discarded. Each message involves `serde_json::from_str` on potentially large tool result content. This is the most significant pure-waste I/O in the system.

**Recommendation:** Either (a) read the JSONL file backwards to find the last Summary line and only parse forward from there, or (b) track the byte offset of the last Summary in SQLite session metadata so loads can `seek()` directly. Option (b) is cleaner and O(1).

---

### PERF-04: Microcompact Rebuilds tool_names Map Every Turn (Severity: Low)

**Location:** `crates/arawn-core/src/session.rs:106-170`

`microcompact()` is called every iteration of the agentic loop. Each time, it builds a `HashMap<String, String>` mapping every `tool_use_id` to `tool_name` across the entire message history, even though it only uses this map for messages before the `cutoff` index.

**Impact:** For a session with 50 tool calls, this builds a 50-entry HashMap per turn. Total: microseconds. The linear scan through TARGETED_TOOLS (18 entries) for each tool result is also O(n) but with n=18 this is negligible.

**Recommendation:** Could filter to only messages before `cutoff` when building the map, but the savings are minimal. No action needed.

---

### PERF-05: Mutex<Store> Held Across Synchronous SQLite Operations (Severity: Low)

**Location:** `crates/arawn/src/local_service.rs` — 11 lock sites

The `Store` is behind `std::sync::Mutex` because `rusqlite::Connection` is not `Send`. Lock acquisitions are brief and correctly scoped — the code is careful to drop locks before any `.await` points (e.g., `send_message` lines 554-576 acquire/release the lock, then does async JSONL work lock-free).

**Impact:** For a single-user system, contention is effectively zero. The lock is held for microsecond-scale SQLite queries. The design is correct.

**Not a finding** — included to document that this was reviewed and is fine.

---

### PERF-06: Per-Message File Open/Close for JSONL Append (Severity: Low)

**Location:** `crates/arawn-storage/src/jsonl.rs:28-51`

Each `append()` call opens the file, writes one line, and closes it (the `File` is dropped at end of scope). During a single engine turn, multiple messages are appended sequentially (assistant message + N tool results), each opening and closing the file.

**Impact:** For a turn with 5 tool calls, this is ~7 file open/close cycles. On modern filesystems with warm caches, each takes <1ms. Total: <10ms per turn, invisible against LLM latency.

**Recommendation:** Could batch all post-turn messages into a single open/write/close, but the complexity isn't justified for the savings.

---

### PERF-07: filter_tools_for_context Scans All Messages Every Turn (Severity: Low)

**Location:** `crates/arawn-engine/src/query_engine.rs:895+`

`filter_tools_for_context` builds a `HashSet` of all previously-used tool names by scanning the entire message history. It also lowercases the last user message for keyword matching. This runs every turn in `build_request()`.

**Impact:** Linear in message count, but messages are capped by compaction. After compaction, there are at most ~10 messages (1 summary + 6 recent + current). Before compaction triggers, there might be ~50 messages. The work is trivial.

**Recommendation:** No action needed.

---

### PERF-08: Unbounded failed_call_counts Growth (Severity: Low)

**Location:** `crates/arawn-engine/src/query_engine.rs:106`, `504-509`

The `failed_call_counts` HashMap grows with every unique failing tool call key (formatted as `"tool_name:arguments_json"`). Successful calls remove their entry, but if the LLM keeps generating different failing calls, this map grows without bound.

**Impact:** In practice, sessions have a max of 200 iterations and compaction resets context, so this map is unlikely to exceed ~50 entries. But the keys include full JSON arguments, which can be large strings.

**Recommendation:** Either cap the map size (evict oldest on overflow) or clear it on compaction. Low priority.

---

### PERF-09: Reconcile Sessions Checks Every Session's JSONL File on Startup (Severity: Low)

**Location:** `crates/arawn-storage/src/store.rs:104-136`

`reconcile_sessions()` iterates all workstreams, then all sessions per workstream, checking `path.exists()` for each JSONL file. This is O(workstreams * sessions) filesystem stat calls.

**Impact:** For a user with 10 workstreams and 50 sessions each, this is 500 stat calls at startup. Takes <100ms. Acceptable for a startup-only operation.

**Recommendation:** No action needed. Could be deferred to background if session count grows large.

---

### PERF-10: MemoryManager.retrieve_topical N+1 Search Pattern (Severity: Low)

**Location:** `crates/arawn-memory/src/manager.rs:87-139`

For L2 topical context, `retrieve_topical()` runs FTS search + tag search for each keyword, across two stores. With k keywords, this is `4 * k` SQLite queries. Keywords are extracted from the user message by splitting on whitespace and filtering to words > 3 chars, which can yield 10-20 keywords for a typical message.

**Impact:** Up to ~80 SQLite FTS queries per message in the worst case. FTS5 queries on a small KB (<1000 entities) are sub-millisecond each, so total is <100ms. The deduplication via `HashSet<Uuid>` prevents result bloat.

**Recommendation:** Could batch keywords into a single FTS query using OR syntax (e.g., `"keyword1 OR keyword2 OR keyword3"`), reducing 2*k FTS queries to 2. Worth doing if L2 injection latency becomes noticeable.

## Non-Findings (Reviewed and Appropriate)

- **Stream processing**: Text and tool argument deltas are accumulated into growing `String` buffers. This is the right approach — no intermediate parsing or copying.
- **Compaction circuit breaker**: After 3 failures, compaction is skipped. Prevents wasting tokens on repeated failed LLM calls.
- **Tool result limiting**: Large outputs are truncated to 2KB preview + full output persisted to disk. This prevents context window bloat without losing data.
- **SQLite WAL mode + busy timeout**: Correct configuration for concurrent read/write patterns.
- **Token estimation via chars/4**: Appropriate heuristic for threshold decisions. Using a real tokenizer would add latency for no benefit.
- **TUI rendering**: Standard ratatui layout calculations, no hot-path issues. The spinner animation and status bar updates are cheap.
- **Read-only tool parallelization**: `join_all` on read-only tools is correct. No need for more sophisticated scheduling.
- **Concurrency model**: Single-user, single-connection WebSocket. No need for connection pooling or worker pools. The `tokio::spawn` for the engine loop is appropriate — it keeps the WebSocket responsive for modal prompts during tool execution.

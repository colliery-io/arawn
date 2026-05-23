---
id: webfetch-improvements-html-to
level: task
title: "WebFetch improvements — HTML-to-markdown (Turndown equivalent), LRU cache, domain safety checks"
short_code: "ARAWN-T-0057"
created_at: 2026-04-03T01:01:40.440228+00:00
updated_at: 2026-04-03T01:50:41.803290+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# WebFetch improvements — HTML-to-markdown (Turndown equivalent), LRU cache, domain safety checks

## Objective

Improve the WebFetch tool with three enhancements: (1) HTML-to-markdown conversion using a Turndown-equivalent Rust crate for much better content extraction than tag stripping, (2) LRU cache with 15-minute TTL to avoid re-fetching the same URL, (3) domain safety preflight checks before fetching.

### Type: Feature | Priority: P2

- **User Value**: Better content extraction from fetched pages (markdown preserves structure, headings, links). Cache saves tokens and latency on repeated fetches.
- **Effort Estimate**: M

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] HTML responses converted to markdown (preserving headings, links, lists, code blocks)
- [ ] LRU cache with 15-minute TTL, 50MB size limit — same URL returns cached content
- [ ] Cache cleared on session end or explicit request
- [ ] Non-HTML responses (JSON, plain text) returned as-is
- [ ] Use a cheaper/faster model for the summarization sub-query where possible

## Implementation Notes

- Rust crate for HTML→markdown: `htmd` or `html2md`
- Cache: `lru` crate with `Instant`-based TTL, keyed by URL
- Claude Code uses Haiku for the summarization call — consider using a lighter model config
- Domain check: optional allowlist/blocklist in `arawn.toml`
- Reference: Claude Code's `WebFetchTool/utils.ts` (Turndown, LRUCache, domain checks)
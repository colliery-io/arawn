---
id: operational-readiness
level: initiative
title: "Operational Readiness"
short_code: "ARAWN-I-0031"
created_at: 2026-03-22T00:39:11.687665+00:00
updated_at: 2026-03-22T22:24:34.441094+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: operational-readiness
---

# Operational Readiness Initiative

## Context

Arawn is transitioning from development to daily personal use. Operational analysis (March 2026) identified critical gaps in graceful shutdown, logging management, backup tooling, health monitoring, and database maintenance that must be addressed for reliable always-on operation.

## Goals & Non-Goals

**Goals:**
- Implement graceful shutdown with proper cleanup sequencing
- Add log rotation and cleanup to prevent disk exhaustion
- Create backup and restore tooling for all persistent data
- Deepen health checks to detect degradation before crash
- Add basic observability (metrics endpoint)
- Document and automate maintenance operations

**Non-Goals:**
- Multi-node deployment or clustering
- Cloud-managed infrastructure
- Enterprise monitoring stack (Prometheus/Grafana — just expose the endpoint)

## Detailed Design

### Graceful Shutdown (P0)
1. **Signal handling** (`arawn-server/src/lib.rs:236`): Add `.with_graceful_shutdown(shutdown_signal())` using `tokio::signal::ctrl_c()` + SIGTERM
2. **Cleanup sequencing** (`arawn/src/commands/start.rs:1489-1506`): Wire post-server cleanup (pipeline shutdown, MCP shutdown, session flush) into the shutdown signal path so it actually executes
3. **In-flight request draining**: Allow active WebSocket connections to complete current operation before teardown

### Log Management (P0)
4. **Log cleanup**: Delete log files older than 30 days — either in-app on startup or via documented cron job
5. **Interaction log rotation**: The LLM interaction JSONL logs also need size/age-based cleanup
6. **Log size monitoring**: Warn when log directory exceeds configurable threshold

### Backup & Recovery (P1)
7. **`scripts/backup.sh`**: Atomic SQLite backups via `.backup` command for memory.db, graph.db, workstreams.db, pipeline.db + config copy + JSONL message files
8. **`arawn backup` CLI command**: Wrapper around the backup script with retention rotation
9. **Restore procedure**: Document and test restoration from backup

### Health & Monitoring (P1)
10. **Deep health check** (`/health/deep`): Verify SQLite connectivity (`SELECT 1`), LLM backend reachability, disk pressure against `PathConfig` thresholds
11. **Basic metrics endpoint** (`/metrics`): Request count/latency by endpoint, active WebSocket connections, session cache size, database sizes, token usage counters
12. **Startup validation**: Validate config values on startup (e.g., negative port, invalid paths) and fail fast with clear error messages

### Database Maintenance (P2)
13. **SQLite WAL checkpoint**: Periodic `PRAGMA wal_checkpoint(TRUNCATE)` — either on startup or via scheduled task
14. **Database vacuum**: Optional `VACUUM` on startup with `--maintenance` flag
15. **Memory schema migration to Refinery**: Migrate from manual SCHEMA_VERSION to Refinery framework, matching workstreams pattern

## Implementation Plan

- Phase 1: Graceful shutdown + log management (2-3 days)
- Phase 2: Backup tooling + health checks (3-4 days)
- Phase 3: Database maintenance + metrics (2-3 days)
#!/usr/bin/env bash
#
# Arawn Backup Script
#
# Creates atomic SQLite backups of all Arawn databases plus config files.
# Uses sqlite3 .backup for consistent snapshots (safe even while server is running).
#
# Usage:
#   ./scripts/backup.sh                    # Backup to default location
#   ./scripts/backup.sh /path/to/backups   # Backup to custom location
#   ARAWN_KEEP_BACKUPS=7 ./scripts/backup.sh  # Keep only 7 most recent backups
#
# Default backup location: ~/.arawn-backups/YYYY-MM-DD_HHMMSS/

set -euo pipefail

# Configuration
ARAWN_DIR="${ARAWN_CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/arawn}"
BACKUP_ROOT="${1:-$HOME/.arawn-backups}"
KEEP_BACKUPS="${ARAWN_KEEP_BACKUPS:-30}"
TIMESTAMP=$(date +%Y-%m-%d_%H%M%S)
BACKUP_DIR="$BACKUP_ROOT/$TIMESTAMP"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
DIM='\033[0;90m'
NC='\033[0m'

info()  { echo -e "${GREEN}✓${NC} $1"; }
warn()  { echo -e "${YELLOW}⚠${NC} $1"; }
error() { echo -e "${RED}✗${NC} $1" >&2; }
dim()   { echo -e "${DIM}  $1${NC}"; }

echo "Arawn Backup"
echo "────────────────────────────────────"
echo "Source:  $ARAWN_DIR"
echo "Target:  $BACKUP_DIR"
echo ""

# Check sqlite3 is available
if ! command -v sqlite3 &>/dev/null; then
    error "sqlite3 not found. Install it to use backup."
    exit 1
fi

# Check source exists
if [ ! -d "$ARAWN_DIR" ]; then
    error "Arawn config directory not found: $ARAWN_DIR"
    exit 1
fi

# Create backup directory
mkdir -p "$BACKUP_DIR"

ERRORS=0
BACKED_UP=0

# Backup SQLite databases using .backup (atomic, WAL-safe)
for db in memory.db memory.graph.db workstreams.db pipeline.db; do
    db_path="$ARAWN_DIR/$db"
    if [ -f "$db_path" ]; then
        if sqlite3 "$db_path" ".backup '$BACKUP_DIR/$db'" 2>/dev/null; then
            size=$(du -sh "$BACKUP_DIR/$db" | cut -f1)
            info "$db ($size)"
            BACKED_UP=$((BACKED_UP + 1))
        else
            error "Failed to backup $db"
            ERRORS=$((ERRORS + 1))
        fi
    else
        dim "$db (not found, skipping)"
    fi
done

# Backup config files (simple copy)
for cfg in config.toml client.toml env; do
    cfg_path="$ARAWN_DIR/$cfg"
    if [ -f "$cfg_path" ]; then
        cp "$cfg_path" "$BACKUP_DIR/"
        info "$cfg"
        BACKED_UP=$((BACKED_UP + 1))
    fi
done

# Backup secrets (encrypted, safe to copy)
if [ -d "$ARAWN_DIR/secrets" ]; then
    cp -r "$ARAWN_DIR/secrets" "$BACKUP_DIR/secrets"
    info "secrets/"
    BACKED_UP=$((BACKED_UP + 1))
fi

# Backup identity key
if [ -f "$ARAWN_DIR/identity.age" ]; then
    cp "$ARAWN_DIR/identity.age" "$BACKUP_DIR/"
    info "identity.age"
    BACKED_UP=$((BACKED_UP + 1))
fi

# Backup workstream message files (JSONL)
if [ -d "$ARAWN_DIR/workstreams" ]; then
    ws_count=0
    mkdir -p "$BACKUP_DIR/workstreams"
    for ws_dir in "$ARAWN_DIR/workstreams"/*/; do
        if [ -d "$ws_dir" ]; then
            ws_name=$(basename "$ws_dir")
            mkdir -p "$BACKUP_DIR/workstreams/$ws_name"
            # Copy JSONL message files
            for jsonl in "$ws_dir"*.jsonl; do
                [ -f "$jsonl" ] && cp "$jsonl" "$BACKUP_DIR/workstreams/$ws_name/"
            done
            ws_count=$((ws_count + 1))
        fi
    done
    if [ $ws_count -gt 0 ]; then
        info "workstreams/ ($ws_count workstream(s))"
        BACKED_UP=$((BACKED_UP + 1))
    fi
fi

# Retention: delete old backups
if [ -d "$BACKUP_ROOT" ] && [ "$KEEP_BACKUPS" -gt 0 ]; then
    old_count=$(ls -1d "$BACKUP_ROOT"/????-??-??_?????? 2>/dev/null | head -n -"$KEEP_BACKUPS" | wc -l | tr -d ' ')
    if [ "$old_count" -gt 0 ]; then
        ls -1d "$BACKUP_ROOT"/????-??-??_?????? 2>/dev/null | head -n -"$KEEP_BACKUPS" | xargs rm -rf
        dim "Cleaned up $old_count old backup(s) (keeping $KEEP_BACKUPS)"
    fi
fi

# Summary
echo ""
total_size=$(du -sh "$BACKUP_DIR" | cut -f1)
echo "────────────────────────────────────"
if [ $ERRORS -eq 0 ]; then
    info "Backup complete: $BACKED_UP item(s), $total_size"
else
    warn "Backup completed with $ERRORS error(s): $BACKED_UP item(s), $total_size"
    exit 1
fi

#!/usr/bin/env bash
# Arawn uninstall script.
# Removes the binary, service files, and optionally config/data.
#
# Usage:
#   bash scripts/uninstall.sh           # Interactive — prompts before removing data
#   bash scripts/uninstall.sh --all     # Remove everything without prompting
#   bash scripts/uninstall.sh --keep-data  # Remove binary + service, keep config/data

set -euo pipefail

# --- Defaults ---
MODE="interactive"  # interactive | all | keep-data

ARAWN_BIN="$HOME/.local/bin/arawn"
CONFIG_DIR="$HOME/.config/arawn"
DATA_DIR="$HOME/.arawn"

PLIST_LABEL="io.colliery.arawn"
PLIST_DEST="$HOME/Library/LaunchAgents/$PLIST_LABEL.plist"
SYSTEMD_DEST="$HOME/.config/systemd/user/arawn.service"

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${BLUE}==>${NC} ${BOLD}$*${NC}"; }
ok()    { echo -e "${GREEN}  ✓${NC} $*"; }
warn()  { echo -e "${YELLOW}  !${NC} $*"; }
err()   { echo -e "${RED}  ✗${NC} $*" >&2; }

# --- Parse arguments ---
for arg in "$@"; do
    case "$arg" in
        --all)       MODE="all" ;;
        --keep-data) MODE="keep-data" ;;
        -h|--help)
            echo "Usage: bash scripts/uninstall.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --all         Remove everything (binary, service, config, data)"
            echo "  --keep-data   Remove binary and service, keep config and data"
            echo "  -h, --help    Show this help message"
            echo ""
            echo "Without flags, prompts before removing config and data."
            exit 0
            ;;
        *)
            err "Unknown argument: $arg"
            exit 1
            ;;
    esac
done

# --- Detect platform ---
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"

echo ""
info "Arawn uninstall"
echo ""

# --- Step 1: Stop and remove service ---
info "Removing service..."

if [ "$OS" = "darwin" ]; then
    if [ -f "$PLIST_DEST" ]; then
        launchctl bootout "gui/$(id -u)/$PLIST_LABEL" 2>/dev/null || true
        rm -f "$PLIST_DEST"
        ok "Stopped and removed launchd plist"
    else
        warn "No launchd plist found"
    fi
else
    if [ -f "$SYSTEMD_DEST" ]; then
        systemctl --user stop arawn 2>/dev/null || true
        systemctl --user disable arawn 2>/dev/null || true
        rm -f "$SYSTEMD_DEST"
        systemctl --user daemon-reload 2>/dev/null || true
        ok "Stopped and removed systemd unit"
    else
        warn "No systemd unit found"
    fi
fi

# Remove wrapper script if present
if [ -f "$CONFIG_DIR/arawn-wrapper.sh" ]; then
    rm -f "$CONFIG_DIR/arawn-wrapper.sh"
    ok "Removed wrapper script"
fi

# --- Step 2: Kill any running arawn processes ---
if pgrep -f "arawn start" >/dev/null 2>&1; then
    pkill -f "arawn start" 2>/dev/null || true
    ok "Killed running arawn processes"
fi

# --- Step 3: Remove binary ---
info "Removing binary..."
if [ -f "$ARAWN_BIN" ]; then
    rm -f "$ARAWN_BIN"
    ok "Removed $ARAWN_BIN"
else
    warn "No binary found at $ARAWN_BIN"
fi

# --- Step 4: Remove config and data ---
REMOVE_CONFIG=false
REMOVE_DATA=false

case "$MODE" in
    all)
        REMOVE_CONFIG=true
        REMOVE_DATA=true
        ;;
    keep-data)
        REMOVE_CONFIG=false
        REMOVE_DATA=false
        ;;
    interactive)
        echo ""
        if [ -d "$CONFIG_DIR" ]; then
            read -rp "  Remove config ($CONFIG_DIR)? [y/N] " answer
            if [[ "$answer" =~ ^[Yy] ]]; then
                REMOVE_CONFIG=true
            fi
        fi
        if [ -d "$DATA_DIR" ]; then
            read -rp "  Remove data ($DATA_DIR)? [y/N] " answer
            if [[ "$answer" =~ ^[Yy] ]]; then
                REMOVE_DATA=true
            fi
        fi
        ;;
esac

info "Removing config and data..."

if $REMOVE_CONFIG; then
    if [ -d "$CONFIG_DIR" ]; then
        rm -rf "$CONFIG_DIR"
        ok "Removed $CONFIG_DIR"
    else
        warn "$CONFIG_DIR not found"
    fi
else
    if [ -d "$CONFIG_DIR" ]; then
        warn "Kept $CONFIG_DIR"
    fi
fi

if $REMOVE_DATA; then
    if [ -d "$DATA_DIR" ]; then
        rm -rf "$DATA_DIR"
        ok "Removed $DATA_DIR"
    else
        warn "$DATA_DIR not found"
    fi
else
    if [ -d "$DATA_DIR" ]; then
        warn "Kept $DATA_DIR"
    fi
fi

# --- Done ---
echo ""
info "Uninstall complete."
echo ""

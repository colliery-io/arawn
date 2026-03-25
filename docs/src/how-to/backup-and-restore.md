# Back Up and Restore Data

This guide walks through backing up all Arawn data and restoring it when needed. Arawn stores state across several SQLite databases, configuration files, encrypted secrets, and workstream message logs. A regular backup strategy protects against data loss from hardware failure, accidental deletion, or botched upgrades.

## Prerequisites

- **sqlite3** must be installed and available on your `PATH`. The backup process uses the SQLite `.backup` command for atomic, WAL-safe snapshots.

  ```sh
  # macOS (pre-installed)
  sqlite3 --version

  # Debian/Ubuntu
  sudo apt install sqlite3

  # Fedora
  sudo dnf install sqlite
  ```

## What gets backed up

| Category | Files | Method |
|----------|-------|--------|
| Databases | `memory.db`, `memory.graph.db`, `workstreams.db`, `pipeline.db` | Atomic SQLite `.backup` |
| Configuration | `config.toml`, `client.toml`, `env` | File copy |
| Secrets | `secrets.age` (age-encrypted JSON map) | File copy |
| Identity key | `identity.age` | File copy |
| Workstream logs | `workstreams/*//*.jsonl` | File copy |

All files live under the Arawn config directory, typically `~/.config/arawn/`.

The SQLite `.backup` command creates a consistent snapshot even while the server is running. You do not need to stop Arawn before taking a backup.

## Run a one-off backup with the CLI

```sh
arawn backup
```

This invokes `scripts/backup.sh` under the hood with default settings: backups go to `~/.arawn-backups/` and the 30 most recent are retained.

### Customize the output directory

```sh
arawn backup --output /mnt/nas/arawn-backups
```

### Customize retention

Keep only the 7 most recent backups:

```sh
arawn backup --keep 7
```

## Run a backup with the shell script directly

For cron jobs or environments where the `arawn` binary is not in `PATH`, use the script directly:

```sh
./scripts/backup.sh                         # defaults
./scripts/backup.sh /mnt/nas/arawn-backups  # custom destination
```

Control retention through the `ARAWN_KEEP_BACKUPS` environment variable:

```sh
ARAWN_KEEP_BACKUPS=7 ./scripts/backup.sh
```

## Backup directory structure

Each backup is stored in a timestamped subdirectory:

```
~/.arawn-backups/
├── 2026-03-22_140000/
│   ├── memory.db
│   ├── memory.graph.db
│   ├── workstreams.db
│   ├── pipeline.db
│   ├── config.toml
│   ├── client.toml
│   ├── env
│   ├── identity.age
│   ├── secrets.age
│   └── workstreams/
│       ├── default/
│       │   └── messages.jsonl
│       └── research/
│           └── messages.jsonl
├── 2026-03-23_140000/
│   └── ...
└── 2026-03-24_140000/
    └── ...
```

## Rolling retention

After each backup, the script removes old backup directories beyond the retention limit. Only the N most recent timestamped directories (matching `YYYY-MM-DD_HHMMSS`) are kept.

The retention count is determined in this order:

1. `--keep` flag passed to `arawn backup`
2. `ARAWN_KEEP_BACKUPS` environment variable
3. Default: 30

## Schedule daily backups with cron

Open your crontab:

```sh
crontab -e
```

Add an entry to run backups daily at 2:00 AM:

```cron
0 2 * * * /usr/local/bin/arawn backup --keep 30 >> /var/log/arawn-backup.log 2>&1
```

Or use the script directly if `arawn` is not in the cron `PATH`:

```cron
0 2 * * * ARAWN_KEEP_BACKUPS=30 /home/you/arawn/scripts/backup.sh >> /var/log/arawn-backup.log 2>&1
```

### macOS launchd alternative

Create `~/Library/LaunchAgents/io.colliery.arawn.backup.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>io.colliery.arawn.backup</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/arawn</string>
        <string>backup</string>
        <string>--keep</string>
        <string>30</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>2</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
    <key>StandardOutPath</key>
    <string>/tmp/arawn-backup-stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/arawn-backup-stderr.log</string>
</dict>
</plist>
```

Load it:

```sh
launchctl load ~/Library/LaunchAgents/io.colliery.arawn.backup.plist
```

## Restore from a backup

Restoring is a manual process. You copy backed-up files back into the Arawn config directory.

### 1. Stop the server

If Arawn is running, stop it first to avoid database corruption:

```sh
arawn stop
```

If running as a system service:

```sh
# macOS
launchctl unload ~/Library/LaunchAgents/io.colliery.arawn.plist

# Linux
systemctl --user stop arawn
```

### 2. Identify the backup to restore

List available backups:

```sh
ls -1 ~/.arawn-backups/
```

Pick the timestamped directory you want to restore from, for example `2026-03-23_140000`.

### 3. Copy files back

```sh
BACKUP=~/.arawn-backups/2026-03-23_140000
ARAWN_DIR=~/.config/arawn

# Restore databases
cp "$BACKUP"/memory.db "$ARAWN_DIR/"
cp "$BACKUP"/memory.graph.db "$ARAWN_DIR/"
cp "$BACKUP"/workstreams.db "$ARAWN_DIR/"
cp "$BACKUP"/pipeline.db "$ARAWN_DIR/"

# Restore config files
cp "$BACKUP"/config.toml "$ARAWN_DIR/" 2>/dev/null
cp "$BACKUP"/client.toml "$ARAWN_DIR/" 2>/dev/null
cp "$BACKUP"/env "$ARAWN_DIR/" 2>/dev/null

# Restore secrets and identity
cp "$BACKUP"/identity.age "$ARAWN_DIR/" 2>/dev/null
cp "$BACKUP"/secrets.age "$ARAWN_DIR/" 2>/dev/null

# Restore workstream message logs
cp -r "$BACKUP"/workstreams/* "$ARAWN_DIR/workstreams/" 2>/dev/null
```

### 4. Remove WAL and SHM files

SQLite WAL files from the running server are now stale. Remove them so the restored databases start clean:

```sh
rm -f "$ARAWN_DIR"/*.db-wal "$ARAWN_DIR"/*.db-shm
```

### 5. Restart the server

```sh
arawn start
# or
arawn start --daemon
```

Verify the restore worked:

```sh
arawn status
arawn memory search "test query"
```

## Verify backup integrity

You can verify a backup contains valid SQLite databases:

```sh
BACKUP=~/.arawn-backups/2026-03-24_140000
for db in memory.db memory.graph.db workstreams.db pipeline.db; do
    if [ -f "$BACKUP/$db" ]; then
        result=$(sqlite3 "$BACKUP/$db" "PRAGMA integrity_check;" 2>&1)
        echo "$db: $result"
    fi
done
```

All databases should report `ok`.

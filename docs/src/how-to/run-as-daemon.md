# Run as a Daemon

This guide covers running the Arawn server as a background daemon, managing it with system service managers, and working with logs. A daemonized server starts at boot, restarts on failure, and keeps your agent available without an open terminal.

## Start as a daemon

Pass the `--daemon` (or `-d`) flag to background the server:

```sh
arawn start --daemon
```

Arawn forks into the background and writes its process ID to a PID file at:

```
~/.config/arawn/arawn.pid
```

All other `start` flags work in daemon mode. For example, to run a daemon on a custom port with authentication:

```sh
arawn start --daemon --port 9090 --token my-secret-token
```

## Stop the daemon

```sh
arawn stop
```

This reads the PID from `~/.config/arawn/arawn.pid`, sends `SIGTERM` to the process, and removes the PID file. If the PID file is stale (the process is no longer running), the file is cleaned up and an error is reported.

## Check if the server is running

```sh
arawn status
```

This connects to the server's health endpoint and reports whether it is reachable, along with the version. For machine-readable output:

```sh
arawn status --json
```

## Graceful shutdown

The server handles `SIGTERM` and `SIGINT` with a graceful shutdown sequence:

1. Stops accepting new connections.
2. Waits for in-flight requests and active WebSocket sessions to complete.
3. Flushes pending writes to databases.
4. Cleans up the PID file.
5. Exits with code 0.

If you need to force-kill a hung process, find the PID and send `SIGKILL`:

```sh
kill -9 $(cat ~/.config/arawn/arawn.pid)
rm ~/.config/arawn/arawn.pid
```

## Environment file

Both system service configurations source environment variables from:

```
~/.config/arawn/env
```

This file uses standard shell syntax. Set your LLM API keys and any other environment overrides here:

```sh
# ~/.config/arawn/env
ANTHROPIC_API_KEY=sk-ant-...
ARAWN_API_TOKEN=my-server-token
```

The wrapper script and systemd `EnvironmentFile` directive both read this file before launching Arawn.

## macOS: launchd service

Arawn ships with a launchd plist at `scripts/service/io.colliery.arawn.plist` and a wrapper script at `scripts/service/arawn-wrapper.sh`.

### Install the service

1. Copy the wrapper script to the Arawn config directory:

   ```sh
   cp scripts/service/arawn-wrapper.sh ~/.config/arawn/arawn-wrapper.sh
   chmod +x ~/.config/arawn/arawn-wrapper.sh
   ```

2. Copy the plist to `~/Library/LaunchAgents/`:

   ```sh
   cp scripts/service/io.colliery.arawn.plist ~/Library/LaunchAgents/
   ```

3. Set up your environment file:

   ```sh
   cp scripts/service/arawn.env ~/.config/arawn/env
   # Edit with your API keys
   vi ~/.config/arawn/env
   ```

4. Load the service:

   ```sh
   launchctl load ~/Library/LaunchAgents/io.colliery.arawn.plist
   ```

The plist is configured with `RunAtLoad` and `KeepAlive` set to `true`, so the server starts at login and restarts automatically if it crashes.

### What the wrapper does

The `arawn-wrapper.sh` script:

1. Adds `~/.cargo/bin` to `PATH` (needed for WASM runtime compilation).
2. Sources `~/.config/arawn/env` to load API keys.
3. Execs `~/.local/bin/arawn start`.

### Manage the service

```sh
# Stop
launchctl unload ~/Library/LaunchAgents/io.colliery.arawn.plist

# Start
launchctl load ~/Library/LaunchAgents/io.colliery.arawn.plist

# Check status
launchctl list | grep arawn
```

### View launchd output

The plist routes stdout and stderr to temporary log files:

```sh
tail -f /tmp/arawn-launchd-stdout.log
tail -f /tmp/arawn-launchd-stderr.log
```

## Linux: systemd user service

Arawn ships with a systemd unit file at `scripts/service/arawn.service`. It is configured as a user service (no root required).

### Install the service

1. Create the systemd user directory if needed:

   ```sh
   mkdir -p ~/.config/systemd/user
   ```

2. Copy the unit file:

   ```sh
   cp scripts/service/arawn.service ~/.config/systemd/user/
   ```

3. Set up your environment file:

   ```sh
   cp scripts/service/arawn.env ~/.config/arawn/env
   vi ~/.config/arawn/env
   ```

4. Reload systemd and enable the service:

   ```sh
   systemctl --user daemon-reload
   systemctl --user enable arawn
   systemctl --user start arawn
   ```

### Unit file details

The unit file runs as a `simple` service type, restarts on failure after a 5-second delay, and reads environment variables from `~/.config/arawn/env`:

```ini
[Unit]
Description=Arawn AI Agent Service
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=%h/.local/bin/arawn start
Restart=on-failure
RestartSec=5
EnvironmentFile=%h/.config/arawn/env
WorkingDirectory=%h
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

### Manage the service

```sh
# Check status
systemctl --user status arawn

# View logs
journalctl --user -u arawn -f

# Restart
systemctl --user restart arawn

# Stop
systemctl --user stop arawn

# Disable auto-start
systemctl --user disable arawn
```

### Enable lingering

By default, user services only run while you are logged in. To keep Arawn running after logout:

```sh
sudo loginctl enable-linger $(whoami)
```

## Logs

Arawn writes daily rotating log files to:

```
~/.config/arawn/logs/arawn.log.YYYY-MM-DD
```

A new log file is created each day. Old log files are cleaned up automatically by the `cleanup_old_logs()` function during server startup.

### View recent logs

```sh
arawn logs               # last 25 lines from the current log
arawn logs -n 100        # last 100 lines
arawn logs --file 2026-03-22   # read a specific day's log
```

### Tail logs in real time

```sh
arawn logs -f
```

This follows the current log file, similar to `tail -f`.

### Fetch logs from a running server

If you are connecting to a remote Arawn instance, use the `--remote` flag to fetch logs over the API instead of reading local files:

```sh
arawn logs --remote
arawn logs --remote -n 50
arawn logs --remote --list-files
```

## Monitoring with health checks

The server exposes a `GET /health` endpoint that returns the server status and version:

```sh
curl http://localhost:8080/health
```

```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

Use this endpoint for uptime monitoring with tools like Uptime Kuma, Healthchecks.io, or a simple cron-based check:

```sh
# Simple health check script for cron
if ! curl -sf http://localhost:8080/health > /dev/null 2>&1; then
    echo "Arawn is down!" | mail -s "Arawn Alert" you@example.com
fi
```

For systemd environments, you can add a watchdog check:

```sh
# /usr/local/bin/arawn-healthcheck.sh
#!/bin/bash
if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
    exit 0
else
    systemctl --user restart arawn
    echo "$(date): Arawn restarted by health check" >> /var/log/arawn-health.log
fi
```

```cron
*/5 * * * * /usr/local/bin/arawn-healthcheck.sh
```

## Summary of paths

| Item | Path |
|------|------|
| PID file | `~/.config/arawn/arawn.pid` |
| Environment file | `~/.config/arawn/env` |
| Log directory | `~/.config/arawn/logs/` |
| Daily log file | `~/.config/arawn/logs/arawn.log.YYYY-MM-DD` |
| launchd plist | `~/Library/LaunchAgents/io.colliery.arawn.plist` |
| launchd wrapper | `~/.config/arawn/arawn-wrapper.sh` |
| systemd unit | `~/.config/systemd/user/arawn.service` |

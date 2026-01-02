# Configuration

Gity uses sensible defaults but can be customized via environment variables and local configuration.

## Data Directory

Gity stores all data in `$GITY_HOME`:

| Platform | Default Location |
|----------|------------------|
| Linux/macOS | `~/.gity` |
| Windows | `%APPDATA%\Gity` |

### Directory Structure

```
~/.gity/
├── config/           # Reserved for future settings
├── data/
│   ├── sled/         # Metadata database
│   └── status_cache/ # Cached status results
└── logs/
    └── daemon.log    # Daemon logs
```

### Changing the Data Directory

Set the `GITY_HOME` environment variable:

```bash
export GITY_HOME=/custom/path/.gity
gity register /path/to/repo
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `GITY_HOME` | Data directory location | `~/.gity` (Unix) or `%APPDATA%\Gity` (Windows) |
| `GITY_DAEMON_ADDR` | IPC socket address | `tcp://127.0.0.1:7557` |

### GITY_HOME

Controls where Gity stores its database, caches, and logs.

```bash
# Linux/macOS
export GITY_HOME=~/.gity

# Windows PowerShell
$env:GITY_HOME = "$env:APPDATA\Gity"
```

### GITY_DAEMON_ADDR

Changes the IPC address the daemon listens on.

```bash
export GITY_DAEMON_ADDR=tcp://127.0.0.1:8888
```

!!! warning
    If you change this, both the daemon and CLI must use the same address.

## Git Configuration

When you register a repository, Gity modifies the repo's `.git/config`:

```ini
[core]
    fsmonitor = gity fsmonitor-helper
    untrackedCache = true

[feature]
    manyFiles = true
```

### What These Settings Do

| Setting | Purpose |
|---------|---------|
| `core.fsmonitor` | Tells Git to use Gity for change detection |
| `core.untrackedCache` | Caches untracked file lists (complements fsmonitor) |
| `feature.manyFiles` | Enables optimizations for large repos (index v4, etc.) |

These settings are automatically removed when you unregister.

### Manual Git Configuration

You can verify the settings:

```bash
git config --get core.fsmonitor
# Should output: gity fsmonitor-helper
```

To manually disable fsmonitor (without unregistering):

```bash
git config --unset core.fsmonitor
```

## Resource Limits

The daemon monitors resource usage and applies back-pressure when limits are exceeded.

### Default Thresholds

| Resource | Threshold | Action |
|----------|-----------|--------|
| CPU usage | 50% sustained | Throttle prefetch jobs |
| Memory (RSS) | 500MB | Pause low-priority jobs |
| File descriptors | 80% of limit | Reduce watcher aggressiveness |
| Sled cache | 100MB per repo | Evict old entries |

When resources return to normal, deferred jobs resume automatically.

### Viewing Resource Usage

```bash
gity list --stats
gity daemon metrics
```

## Logging

Daemon logs are written to `$GITY_HOME/logs/daemon.log`.

### Viewing Logs

```bash
# View recent logs
cat ~/.gity/logs/daemon.log

# Stream logs for a specific repo
gity logs /path/to/repo --follow
```

### Log Format

Logs are structured and include:

- Timestamp
- Event type
- Repository path (when applicable)
- Resource metrics
- Error details

## Startup Behavior

### Auto-start

CLI commands automatically start the daemon if not running:

```bash
gity register /path/to/repo  # Starts daemon if needed
gity list                     # Starts daemon if needed
```

### Manual Control

```bash
gity daemon start   # Start in background
gity daemon stop    # Stop gracefully
gity daemon run     # Start in foreground (for debugging)
```

### System Startup

To start Gity automatically at login:

=== "Linux (systemd)"

    Create `~/.config/systemd/user/gity.service`:

    ```ini
    [Unit]
    Description=Gity Daemon

    [Service]
    ExecStart=/usr/local/bin/gity daemon run
    Restart=on-failure

    [Install]
    WantedBy=default.target
    ```

    Enable:

    ```bash
    systemctl --user enable gity
    systemctl --user start gity
    ```

=== "macOS (launchd)"

    Create `~/Library/LaunchAgents/com.gity.daemon.plist`:

    ```xml
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
      "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
        <key>Label</key>
        <string>com.gity.daemon</string>
        <key>ProgramArguments</key>
        <array>
            <string>/usr/local/bin/gity</string>
            <string>daemon</string>
            <string>run</string>
        </array>
        <key>RunAtLoad</key>
        <true/>
        <key>KeepAlive</key>
        <true/>
    </dict>
    </plist>
    ```

    Load:

    ```bash
    launchctl load ~/Library/LaunchAgents/com.gity.daemon.plist
    ```

=== "Windows"

    Use Task Scheduler or add to startup folder. The MSI installer can configure this automatically.

## Cache Sharing

When the same repository is registered in multiple locations (e.g., worktrees), Gity shares cache data via rykv replication:

- Hot keys are mirrored between related registrations
- Cold starts are faster for new worktrees
- No network traffic—sharing is local only

# Troubleshooting

Solutions to common issues with Gity.

## Git Status is Still Slow

### Check Registration

Verify the repository is registered:

```bash
gity list
```

If not listed, register it:

```bash
gity register /path/to/repo
```

### Check Health

Run diagnostics:

```bash
gity health /path/to/repo
```

Look for:

- **Watcher status**: Should be "active"
- **Reconciliation needed**: If true, the cache is rebuilding
- **Throttling active**: Resources may be constrained

### Verify FSMonitor

Check that Git is configured to use Gity:

```bash
git config core.fsmonitor
# Should output: gity fsmonitor-helper
```

If missing or incorrect:

```bash
gity unregister /path/to/repo
gity register /path/to/repo
```

### Check Git Version

Gity requires Git 2.37+:

```bash
git --version
```

Upgrade Git if necessary.

---

## Daemon Won't Start

### Check Logs

```bash
cat ~/.gity/logs/daemon.log
```

Common issues:

- Port already in use
- Permission denied
- Corrupted database

### Port Conflict

Check if something else is using the port:

```bash
# Linux/macOS
lsof -i :7557

# Windows
netstat -ano | findstr :7557
```

Kill the conflicting process or change `GITY_DAEMON_ADDR`:

```bash
export GITY_DAEMON_ADDR=tcp://127.0.0.1:7558
```

### Permission Issues

Ensure you have write access to `$GITY_HOME`:

```bash
ls -la ~/.gity
```

### Corrupted Database

If the sled database is corrupted:

```bash
gity daemon stop
rm -rf ~/.gity/data/sled
gity daemon start
# Re-register your repos
gity register /path/to/repo
```

---

## Changes Not Detected

### After Sleep/Hibernate

The file watcher may miss events during system sleep. Force a refresh:

```bash
gity health /path/to/repo
git status  # Triggers reconciliation if needed
```

### Watcher Errors

Check for watcher issues:

```bash
gity health /path/to/repo
```

If the watcher stopped, unregister and re-register:

```bash
gity unregister /path/to/repo
gity register /path/to/repo
```

### Linux: inotify Limits

Large repos can exceed inotify watch limits. See [Linux Guide](../platform-guides/linux.md#inotify-limits).

### Network Filesystems

File watching is unreliable on NFS, SMB, SSHFS. Consider:

- Moving the repo to local storage
- Disabling fsmonitor for that repo:

```bash
git config core.fsmonitor false
```

---

## WSL2 Issues

### Events Not Working

If your repo is on `/mnt/c/` (Windows filesystem), inotify doesn't work across the 9P boundary.

**Solution**: Move repos to the Linux filesystem:

```bash
cd ~
git clone https://github.com/org/repo.git
gity register ~/repo
```

See [WSL2 Guide](../platform-guides/wsl2.md) for details.

---

## High Resource Usage

### Check Metrics

```bash
gity daemon metrics
gity list --stats
```

### Too Many Repositories

Each registered repo has overhead. Consider unregistering repos you're not actively using:

```bash
gity unregister /path/to/inactive-repo
```

### Large Dirty Set

If many files changed (e.g., after switching branches), the cache may be processing:

```bash
gity health /path/to/repo
# Wait for dirty count to decrease
```

### Background Jobs

Check pending jobs:

```bash
gity list --stats
# Look at "Jobs" column
```

Jobs are processed in priority order. High-priority work completes first.

---

## Tray Icon Issues

### Icon Not Appearing

The tray depends on platform-specific APIs:

- **Linux**: Requires a system tray (GNOME, KDE, etc.)
- **macOS**: Should work automatically
- **Windows**: Should work automatically

Try restarting:

```bash
gity daemon stop
gity tray
```

### Tray Can't Connect

If the tray shows "Disconnected":

```bash
gity daemon stop
gity daemon start
gity tray
```

---

## Error Messages

### "Repository not found"

The path doesn't contain a valid Git repository:

```bash
ls -la /path/to/repo/.git
```

Ensure you're pointing to the repo root, not a subdirectory.

### "Already registered"

The repo is already being watched. Check status:

```bash
gity health /path/to/repo
```

### "Permission denied"

Ensure you have read/write access:

```bash
ls -la /path/to/repo
ls -la /path/to/repo/.git
```

### "Connection refused"

The daemon isn't running:

```bash
gity daemon start
```

### "Token mismatch"

The fsmonitor token is stale. This triggers automatic reconciliation:

```bash
git status  # Forces refresh
```

---

## Getting Help

### Collect Diagnostics

Before reporting an issue, gather:

```bash
# Version info
gity --version
git --version

# Repository health
gity health /path/to/repo

# Resource usage
gity list --stats
gity daemon metrics

# Recent logs
gity logs /path/to/repo --limit 100

# Git configuration
git config --list --show-origin | grep -E "(fsmonitor|untracked)"
```

### Report an Issue

Open an issue at [github.com/yourusername/gity/issues](https://github.com/yourusername/gity/issues) with:

1. Description of the problem
2. Steps to reproduce
3. Diagnostic output (above)
4. Platform and version information

---

## Reset Everything

If all else fails, start fresh:

```bash
# Stop daemon
gity daemon stop

# Remove all Gity data
rm -rf ~/.gity

# Restart
gity daemon start

# Re-register repos
gity register /path/to/repo
```

!!! warning
    This removes all cached data and registrations. You'll need to re-register all repositories.

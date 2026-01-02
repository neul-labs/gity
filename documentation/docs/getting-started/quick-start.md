# Quick Start

Get your first repository accelerated in under a minute.

## Step 1: Register Your Repository

Navigate to a large repository and register it:

```bash
cd /path/to/your/large-repo
gity register .
```

Or specify the path directly:

```bash
gity register /path/to/your/large-repo
```

!!! success "What happens"
    - Gity starts watching the repository for file changes
    - Git is configured to use Gity as the fsmonitor
    - An initial scan caches the current state

## Step 2: Use Git Normally

That's it! Git commands are now accelerated:

```bash
git status    # Fast!
git diff      # Fast!
git add .     # Fast!
```

## Step 3: Verify It's Working

Check that the repository is registered and healthy:

```bash
gity list
```

Output:

```
Registered repositories:
  /path/to/your/large-repo
    Status: healthy
    Watcher: active
    Cache: warm
```

For detailed diagnostics:

```bash
gity health /path/to/your/large-repo
```

## Daemon Management

The daemon starts automatically when needed. For manual control:

```bash
# Check daemon status
gity daemon metrics

# Start daemon in background
gity daemon start

# Stop daemon
gity daemon stop
```

## System Tray (Optional)

Launch the system tray UI for at-a-glance status:

```bash
gity tray
```

The tray icon shows registered repos, health status, and provides quick access to common actions.

## What's Next?

- [Registering Repositories](../user-guide/registering-repos.md) — Learn about multi-repo setups
- [Commands Reference](../user-guide/commands.md) — Explore all available commands
- [Troubleshooting](../user-guide/troubleshooting.md) — Fix common issues

## Benchmarking

Want to see the improvement? Try this before and after:

```bash
# Before registering (or after unregistering)
time git status

# After registering
gity register .
time git status
```

Typical improvements:

| Repository Size | Before Gity | After Gity |
|-----------------|-------------|------------|
| 10,000 files | 1-2 seconds | <50ms |
| 100,000 files | 5-10 seconds | <100ms |
| 1,000,000 files | 30+ seconds | <500ms |

Results vary based on disk speed, file system, and repository structure.

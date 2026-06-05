# Registering Repositories

Learn how to register, manage, and unregister repositories with Gity.

## Basic Registration

Register a repository to start accelerating Git operations:

```bash
gity register /path/to/repo
```

Or from within the repository:

```bash
cd /path/to/repo
gity register .
```

## What Registration Does

When you register a repository, Gity:

1. **Starts a file watcher** — Monitors the working tree for changes using OS-native APIs
2. **Configures Git** — Sets up fsmonitor and related optimizations in `.git/config`
3. **Performs initial scan** — Caches the current file state for fast comparisons
4. **Schedules maintenance** — Queues background tasks like prefetch and commit-graph updates

## Git Configuration Changes

Registration adds these settings to your repository's `.git/config`:

```ini
[core]
    fsmonitor = gity fsmonitor-helper
    untrackedCache = true

[feature]
    manyFiles = true
```

These are automatically removed when you unregister.

## Managing Multiple Repositories

### Listing Registered Repos

```bash
gity list
```

Output (one repo per line, with queue depth, status, and watcher generation):

```
/home/user/projects/frontend [0 jobs, status idle, gen 12]
/home/user/projects/backend  [1 jobs, status busy, gen 47]
/home/user/work/monorepo     [0 jobs, status idle, gen 5]
```

### With Statistics

```bash
gity list --stats
```

`--stats` augments each repo line with daemon-side resource numbers (CPU %, RSS, cache size, queued jobs). The exact column layout is driven by the daemon's metrics output — see `gity daemon metrics` for the underlying data.

## Worktrees

Git worktrees are separate working directories that share the same `.git` object store. Register each worktree independently:

```bash
# Main worktree
gity register /path/to/repo

# Feature worktree
gity register /path/to/repo-feature-x

# Another worktree
gity register /path/to/repo-bugfix-y
```

Benefits:

- Each worktree has its own file watcher
- Cache data is shared when possible (via rykv replication)
- Independent health tracking

## Unregistering

Stop accelerating a repository:

```bash
gity unregister /path/to/repo
```

This:

- Stops the file watcher
- Removes Gity-specific Git configuration
- Cleans up cached metadata

!!! warning "Always unregister before deleting"
    If you delete a repository directory without unregistering, run `gity list` and manually unregister any stale entries.

## Repository Health

Check the health of a registered repository:

```bash
gity health /path/to/repo
```

Output includes:

- Watcher status (active, reconciling, error)
- Current generation token
- Dirty path count
- Scheduled jobs
- Resource usage

## Troubleshooting Registration

### Repository Not Found

```
Error: Repository not found at /path/to/repo
```

Ensure the path points to a valid Git repository (contains `.git`).

### Already Registered

```
Warning: Repository already registered
```

The repository is already being watched. Use `gity health` to check its status.

### Permission Denied

```
Error: Permission denied accessing /path/to/repo
```

Ensure you have read/write access to the repository and its `.git` directory.

### Watcher Limits

On Linux, you may hit inotify watch limits. See [Linux Platform Guide](../platform-guides/linux.md#inotify-limits) for solutions.

## Best Practices

1. **Register at the root** — Always register the repository root, not subdirectories

2. **One registration per worktree** — Each worktree needs its own registration

3. **Avoid network filesystems** — File watching is unreliable on NFS, SMB, SSHFS

4. **WSL2 users** — Keep repos on the Linux filesystem, not `/mnt/c/`. See [WSL2 Guide](../platform-guides/wsl2.md)

5. **Clean up stale registrations** — Run `gity list` periodically and unregister deleted repos

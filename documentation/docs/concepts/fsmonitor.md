# FSMonitor Integration

Deep dive into how Gity implements Git's fsmonitor protocol.

## Overview

Git's fsmonitor feature allows an external process to tell Git which files have changed since the last query. This avoids expensive full-tree scans.

Gity implements **fsmonitor protocol version 2** (Git 2.37+).

## Protocol Details

### Query Format

Git invokes the fsmonitor helper with:

```bash
gity fsmonitor-helper <version> <token>
```

- **version** — Protocol version (must be `2`)
- **token** — Opaque token from the previous response (or empty for first query)

### Response Format

The helper outputs NUL-separated data:

```
<new_token>\0<path1>\0<path2>\0...
```

- **new_token** — Token for Git to use in the next query
- **path1, path2, ...** — Relative paths that changed

If nothing changed:

```
<same_token>\0
```

### Example Session

```
# First query (no previous token)
$ gity fsmonitor-helper 2 ""
1\0

# Nothing changed since gen=1
$ gity fsmonitor-helper 2 "1"
1\0

# User edits file.rs
$ gity fsmonitor-helper 2 "1"
2\0file.rs\0

# Nothing changed since gen=2
$ gity fsmonitor-helper 2 "2"
2\0
```

## Token Semantics

Gity uses a **generation counter** as the token:

- Each filesystem event increments the generation
- Querying with an old generation returns all paths since then
- Querying with the current generation returns an empty list

This provides consistency: you always see all changes that occurred between tokens.

## Working Tree Filtering

The file watcher sees all filesystem events, including changes inside `.git/`:

- `.git/HEAD` — Branch switches
- `.git/index` — Staging changes
- `.git/refs/*` — New commits, tags

However, Git's fsmonitor contract expects only **working tree paths**. The `.git` directory is managed by Git itself.

Gity filters internal paths before responding:

```rust
fn is_git_internal_path(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == ".git")
}
```

This ensures:

| Event | Reported to Git? |
|-------|------------------|
| `src/main.rs` changed | Yes |
| `.git/HEAD` changed | No |
| `.git/index` changed | No |
| `submodule/.git/HEAD` changed | No |
| `submodule/src/lib.rs` changed | Yes |

## Branch Switches

When you run `git checkout <branch>`:

1. Git updates `.git/HEAD`
2. Git checks out files from the new branch
3. Watcher sees `.git` changes AND working tree changes
4. Gity filters out `.git` paths
5. Git receives only working tree paths
6. Git updates its index correctly

This "just works" because only actual working tree changes are reported.

## Edge Cases

### Ignored Files

Gity reports **all** changed paths, including files matching `.gitignore`:

```
.gitignore contains: *.log
app.log changes → gity reports it → Git filters it out
```

This is correct: fsmonitor reports filesystem changes, Git applies ignore rules.

### Submodules

Submodules have their own `.git` directory. Gity filters appropriately:

- `submodule/.git/HEAD` → Filtered (internal)
- `submodule/src/lib.rs` → Reported (working tree)

!!! note
    Submodules should be registered separately if you want acceleration within them.

### Nested Repositories

Non-submodule nested repos (e.g., vendored dependencies):

```
myproject/
├── .git/
└── vendor/
    └── somelib/
        ├── .git/        ← nested repo
        └── src/
```

Gity filters `vendor/somelib/.git/*` but reports `vendor/somelib/src/*`.

The nested repo won't get fsmonitor acceleration unless separately registered.

### Symlinks

- Linux/macOS: Symlink targets are not followed
- Windows: May be resolved depending on filesystem config

Only the symlink itself is reported, not target content.

### Rapid Changes

Build systems can generate rapid changes:

- OS watcher batches events
- Gity coalesces changes to the same path
- Generation advances once per batch

### Case Sensitivity

Gity preserves path case as reported by the filesystem. On case-insensitive filesystems (Windows, macOS default), Git handles normalization.

## Platform Considerations

### Linux: inotify Limits

Large repos may exceed watch limits:

```bash
# Check limit
cat /proc/sys/fs/inotify/max_user_watches

# Increase temporarily
sudo sysctl fs.inotify.max_user_watches=524288

# Increase permanently
echo "fs.inotify.max_user_watches=524288" | sudo tee -a /etc/sysctl.conf
```

### macOS: FSEvents Latency

FSEvents has inherent latency (~300ms-1s). Status immediately after saving may use stale cache; the next query will be correct.

### Windows: Long Paths

Enable long path support if you have deeply nested files:

```
HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\LongPathsEnabled = 1
```

### WSL2

File watching only works for repos on the Linux filesystem (`~/`), not Windows filesystem (`/mnt/c/`). See [WSL2 Guide](../platform-guides/wsl2.md).

### Network Filesystems

File notifications are unreliable on NFS, SMB, SSHFS. Consider disabling fsmonitor:

```bash
git config core.fsmonitor false
```

## Reconciliation

If the daemon was offline, events may have been missed. On startup:

1. Compare watcher token with stored token
2. If drift detected, mark repo for reconciliation
3. Next query triggers full scan
4. Synthetic events generated for any differences
5. Normal watching resumes

## Git Configuration

When registered, Gity sets:

```ini
[core]
    fsmonitor = gity fsmonitor-helper
    untrackedCache = true

[feature]
    manyFiles = true
```

| Setting | Purpose |
|---------|---------|
| `fsmonitor` | Use Gity for change detection |
| `untrackedCache` | Cache untracked files (complements fsmonitor) |
| `manyFiles` | Enable large-repo optimizations |

## Debugging

### Verify fsmonitor is active

```bash
git config core.fsmonitor
# Should output: gity fsmonitor-helper
```

### Test the helper directly

```bash
# First query
gity fsmonitor-helper 2 ""

# Subsequent query
gity fsmonitor-helper 2 "42"
```

### View real-time events

```bash
gity events
```

### Check health

```bash
gity health /path/to/repo
```

## Performance

Typical response times:

| Scenario | Response Time |
|----------|---------------|
| No changes | < 1ms |
| Few files changed | < 5ms |
| Many files changed | < 50ms |
| After reconciliation | 100-500ms (one-time) |

The fsmonitor response is essentially a cache lookup plus IPC round-trip.

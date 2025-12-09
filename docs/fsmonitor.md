# FSMonitor Integration

This document describes how gity integrates with Git's fsmonitor feature to accelerate `git status` and related commands.

## Overview

Git's fsmonitor feature allows an external process to tell Git which files have changed since the last query. Instead of Git scanning every file in the working tree, it asks the fsmonitor "what changed?" and only examines those files.

Gity implements the fsmonitor protocol by:

1. Running a file watcher that tracks all filesystem changes
2. Maintaining a dirty paths cache with generation tokens
3. Responding to Git's fsmonitor queries with only the changed paths

## Protocol

Gity implements **fsmonitor protocol version 2** (Git 2.37+).

### Query Format

Git invokes the fsmonitor helper with:

```
gity fsmonitor-helper <version> <token>
```

- `version`: Protocol version (must be `2`)
- `token`: Opaque token from the previous response (or empty for first query)

### Response Format

The helper outputs NUL-separated data:

```
<new_token>\0<path1>\0<path2>\0...
```

- `new_token`: Token for Git to pass in the next query
- `path1`, `path2`, ...: Relative paths that changed since the previous token

If nothing changed, the response is just:

```
<same_token>\0
```

### Token Semantics

Gity uses a **generation counter** as the token:

- Each filesystem event increments the generation
- When Git queries with an old generation, gity returns all paths that changed since then
- When Git queries with the current generation and nothing changed, gity returns an empty path list

## Implementation Details

### File Watching

Gity watches the repository directory recursively using the `notify` crate:

| Platform | Backend |
|----------|---------|
| Linux | inotify |
| macOS | FSEvents |
| Windows | ReadDirectoryChangesW |

The watcher monitors:

- All files in the working tree
- The `.git` directory (for internal state tracking)

### Working Tree Path Filtering

**Important**: Git's fsmonitor only expects paths in the **working tree**. The `.git` directory is managed by Git itself and should never be reported to fsmonitor.

When a file changes, gity:

1. Records the event with its relative path
2. Marks the path as dirty in the metadata store
3. **Filters out `.git` internal paths** before responding to fsmonitor

```rust
// Paths like .git/HEAD, .git/index are filtered out
fn is_git_internal_path(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == ".git")
}
```

This filtering is critical because:

- Git modifies `.git/HEAD`, `.git/index`, etc. during normal operations
- Reporting these paths would confuse Git and cause incorrect behavior
- The fsmonitor contract is specifically for working tree files

### Branch Switches

When you run `git checkout <branch>`:

1. Git updates `.git/HEAD` to point to the new branch
2. Git checks out files from the new branch tip
3. The file watcher sees both `.git` changes and working tree changes
4. Gity filters out `.git` paths and reports only working tree changes
5. Git receives the correct list of files that need re-verification

This works correctly because:

- The working tree files that change during checkout are reported
- The `.git` internal changes are filtered out
- Git uses the reported paths to update its index

### Generation Tokens

The generation token provides consistency guarantees:

```
Timeline:
────────────────────────────────────────────────────►
    gen=1       gen=2       gen=3       gen=4
      │           │           │           │
   file.rs    src/lib.rs    (idle)    Cargo.toml
   changed    changed                  changed

Query with token=1:
  Returns: file.rs, src/lib.rs, Cargo.toml (gen=4)

Query with token=4:
  Returns: (empty list, gen=4)
```

The generation increments:

- When the dirty paths set is drained for a query
- When new file events are recorded

### Reconciliation After Downtime

If the daemon was offline (machine sleep, daemon restart), file events may have been missed. On startup:

1. Gity checks if the watcher token matches the stored token
2. If there's drift, it marks the repo as needing reconciliation
3. The next query triggers a full working tree scan
4. After reconciliation, normal watching resumes

## Git Configuration

When you run `gity register`, it configures:

```ini
[core]
    fsmonitor = gity fsmonitor-helper
    untrackedCache = true

[feature]
    manyFiles = true
```

These settings enable:

- `fsmonitor`: Use gity as the fsmonitor provider
- `untrackedCache`: Cache untracked file lists (complements fsmonitor)
- `manyFiles`: Optimizations for large repos (index v4, etc.)

Run `gity unregister` to remove these settings.

## Edge Cases

### Git LFS

Gity works alongside Git LFS but doesn't coordinate with it:

- **LFS pointer files** are tracked like normal files
- **LFS smudge/clean filters** run during checkout independently
- **Large file downloads** happen when Git needs the content, not during gity prefetch

**Recommendation**: Gity accelerates status checks; LFS handles large file storage. They operate at different layers and don't conflict, but gity won't pre-fetch LFS objects.

### .gitignore and Ignored Files

Gity reports **all changed paths** to Git's fsmonitor, including files that would be ignored:

```
.gitignore contains: *.log
app.log changes → gity reports it → Git filters it out
```

This is correct behavior:
- The fsmonitor contract is to report filesystem changes
- Git applies `.gitignore` rules after receiving the path list
- This ensures ignore rule changes are handled correctly

### Submodules

Submodules have their own `.git` directory (either inline or via `.git` file pointing to `../.git/modules/`).

Gity handles submodules by filtering paths containing `.git`:

```rust
fn is_git_internal_path(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == ".git")
}
```

This filters:
- `submodule/.git/HEAD` → filtered (submodule internals)
- `submodule/src/lib.rs` → reported (submodule working tree)

**Note**: Submodules are excluded from gity's `working_tree_status` calls (`.exclude_submodules(true)`). Each submodule should be registered separately if you want acceleration.

### Nested Git Repositories

If you have nested repos (not submodules), e.g., a vendored dependency with its own `.git`:

```
myproject/
├── .git/
├── src/
└── vendor/
    └── somelib/
        ├── .git/        ← nested repo
        └── src/
```

Gity filters `vendor/somelib/.git/*` paths but reports `vendor/somelib/src/*` changes. However:

- **The nested repo won't get fsmonitor acceleration** unless separately registered
- **Changes to the nested repo appear as changes to the parent**

**Recommendation**: Avoid nested repos. Use submodules or package managers instead.

### Rapid File Changes

When files change rapidly (build systems, file generators):

- Events are batched by the OS watcher backend
- Gity coalesces rapid changes to the same path
- The generation advances once per batch, not per event

### Symlinks

Symlink behavior depends on the platform:

- Linux/macOS: Symlink targets are not followed; only the symlink itself is reported
- Windows: Symlinks may be resolved depending on filesystem configuration

### Case Sensitivity

Gity preserves path case as reported by the filesystem. On case-insensitive filesystems (Windows, macOS default), Git handles case normalization.

### Network Filesystems

Gity relies on OS file notification APIs, which may not work reliably on network filesystems:

- NFS: Limited support, may miss events
- SMB/CIFS: Works on Windows, unreliable on Unix
- SSHFS: Generally unreliable

For network filesystems, consider disabling fsmonitor:

```bash
git config core.fsmonitor false
```

### Git Worktrees

Each worktree is a separate working directory sharing the same `.git` object store. Gity treats each worktree as an independent repository:

```bash
gity register /path/to/main-worktree
gity register /path/to/feature-worktree
```

Worktrees share object data but have independent:
- Working tree file watchers
- Dirty path caches
- Generation tokens

This is the correct behavior—each worktree has its own filesystem state.

### Sparse Checkout

With `git sparse-checkout`, only a subset of files are materialized in the working tree.

**Current behavior**: Gity watches and reports all materialized files. Non-materialized files don't exist on disk, so no events occur.

**Potential issue**: If sparse patterns change, gity doesn't know which paths are now relevant. Git handles this correctly because it tracks sparse patterns internally.

**Recommendation**: After changing sparse patterns, the next `git status` will reconcile correctly. No special handling needed.

### Partial Clone

With `git clone --filter=blob:none`, blob objects are fetched on demand.

**Current behavior**: Gity prefetch uses `git maintenance run --task=prefetch`, which respects partial clone settings and doesn't fetch filtered objects.

**Note**: Status checks may trigger blob fetches if Git needs to compare content. This is Git's behavior, not gity's.

### File Moves and Renames

The file watcher reports moves as separate delete + create events:

```
mv foo.rs bar.rs
→ Event: foo.rs deleted
→ Event: bar.rs created
```

Both paths are marked dirty. Git's rename detection handles this during status.

### Hard Links

Hard links can cause issues:
- Modifying a hard-linked file may not trigger an event on all link paths
- Only the path actually written to may be reported

**Recommendation**: Avoid hard links in Git repositories.

### Watched Directory Deletion

If the watched repository directory is deleted or moved:
- The watcher will error and stop
- Subsequent commands will fail with "repo not found"
- Run `gity unregister` to clean up

### Linux inotify Watch Limits

Linux has a system-wide limit on inotify watches (default: 8192). Large repos can exceed this.

**Symptoms**: Watcher fails to start, or misses events in deep directories.

**Fix**:
```bash
# Check current limit
cat /proc/sys/fs/inotify/max_user_watches

# Increase temporarily
sudo sysctl fs.inotify.max_user_watches=524288

# Increase permanently
echo "fs.inotify.max_user_watches=524288" | sudo tee -a /etc/sysctl.conf
```

### macOS FSEvents Latency

FSEvents on macOS has inherent latency (~300ms-1s). File changes may not be immediately visible to gity.

**Impact**: `git status` immediately after saving may use stale cache. Running again will be correct.

**Mitigation**: This is an OS limitation. The latency is usually acceptable for interactive use.

### Windows Long Paths

Windows has a 260-character path limit (MAX_PATH) by default.

**Symptoms**: Files in deeply nested directories may not trigger events or may fail to be reported.

**Fix**: Enable long path support in Windows 10+:
```
# Group Policy: Computer Configuration > Administrative Templates >
#   System > Filesystem > Enable Win32 long paths

# Or registry:
HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\LongPathsEnabled = 1
```

### WSL2 (Windows Subsystem for Linux)

WSL2 has significant file system notification limitations that affect gity:

**Scenario 1: Repo on Windows filesystem (`/mnt/c/...`)**

```
Windows (NTFS)          WSL2 (Linux)
┌──────────────┐       ┌──────────────┐
│ C:\code\repo │ ←───→ │ /mnt/c/code/ │
│              │  9P   │    repo      │
└──────────────┘       └──────────────┘
```

- Files accessed via `/mnt/c/` use the 9P protocol
- **inotify does NOT work** across this boundary
- File changes made in Windows are invisible to Linux watchers
- File changes made in WSL2 may have delayed/missing notifications

**Result**: Gity running in WSL2 watching `/mnt/c/...` will miss most events.

**Scenario 2: Repo on Linux filesystem (`~/code/...`)**

```
WSL2 (ext4)             Windows
┌──────────────┐       ┌──────────────┐
│ ~/code/repo  │ ←───→ │ \\wsl$\...   │
│              │       │              │
└──────────────┘       └──────────────┘
```

- Files on the Linux filesystem (ext4) work correctly with inotify
- **Gity works properly** when repo is on Linux filesystem
- Accessing from Windows via `\\wsl$\` doesn't affect Linux-side watching

**Result**: Gity works correctly.

**Scenario 3: Mixed access patterns**

If you edit files from both Windows and WSL2:
- Windows editors modifying Linux filesystem files → events detected
- WSL2 tools modifying Windows filesystem files → events may be missed

**Recommendations**:

| Workflow | Recommendation |
|----------|----------------|
| Repo on `/mnt/c/`, edit in Windows | Run gity natively on Windows |
| Repo on `/mnt/c/`, edit in WSL2 | Move repo to Linux filesystem |
| Repo on `~/`, edit in WSL2 | Works correctly |
| Repo on `~/`, edit in Windows via `\\wsl$\` | Works correctly |

**Best practice for WSL2**:

```bash
# Clone repos to Linux filesystem, not /mnt/c/
cd ~
git clone https://github.com/org/repo.git
gity register ~/repo  # Works correctly

# Avoid this:
gity register /mnt/c/Users/me/repo  # Will miss events!
```

**Detecting WSL2**:

```bash
# Check if running in WSL
if grep -qi microsoft /proc/version; then
    echo "Running in WSL"
fi

# Check filesystem type
df -T /path/to/repo | grep -E "9p|drvfs"  # Windows filesystem
df -T /path/to/repo | grep ext4           # Linux filesystem (good)
```

### Docker and Containers

File events may not propagate correctly through Docker bind mounts:

| Mount Type | Event Support |
|------------|---------------|
| Bind mount (Linux) | Usually works |
| Bind mount (macOS/Windows) | Often broken |
| Named volume | Events work inside container only |
| NFS/network | Unreliable |

**Recommendation**: Run gity inside the container if using bind mounts on macOS/Windows, or disable fsmonitor for containerized workflows.

### Docker in WSL2

When running Docker Desktop with WSL2 backend:

```
Windows → Docker Desktop → WSL2 → Container
```

- Bind mounts from Windows (`-v C:\code:/app`) have the same 9P limitations
- Bind mounts from WSL2 Linux filesystem (`-v ~/code:/app`) work better
- For best results, keep repos on WSL2 Linux filesystem

### Concurrent Git Operations

Multiple simultaneous `git status` calls are safe:
- Each query gets a consistent snapshot
- Generation tokens ensure proper sequencing
- The dirty paths cache is drained atomically per query

However, very rapid queries may see slightly stale data due to event batching.

### Pre-commit Hooks

Pre-commit hooks that modify files (formatters, linters) cause rapid changes:

1. You run `git commit`
2. Pre-commit modifies files
3. Gity sees the modifications
4. Commit proceeds (or fails)
5. Next status reflects final state

This works correctly—gity just sees the filesystem changes.

## Debugging

### Verify fsmonitor is active

```bash
git config core.fsmonitor
# Should output: gity fsmonitor-helper
```

### Test the helper directly

```bash
# First query (no token)
gity fsmonitor-helper 2 ""

# Subsequent query (with token from previous response)
gity fsmonitor-helper 2 "42"
```

### Check daemon health

```bash
gity health /path/to/repo
```

Shows:

- Current generation token
- Dirty path count
- Whether reconciliation is needed
- Watcher status

### View real-time events

```bash
gity events
```

Streams all file watcher events as they occur.

## Performance

Typical response times:

| Scenario | Response Time |
|----------|---------------|
| No changes | < 1ms |
| Few files changed | < 5ms |
| Many files changed | < 50ms |
| After reconciliation | 100-500ms (one-time) |

The fsmonitor response is essentially a cache lookup plus IPC round-trip.

## Related Documentation

- [architecture.md](architecture.md) - System design overview
- [commands.md](commands.md) - CLI reference
- [Git fsmonitor documentation](https://git-scm.com/docs/git-config#Documentation/git-config.txt-corefsmonitor)

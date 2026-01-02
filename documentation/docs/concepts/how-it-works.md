# How It Works

Understanding how Gity accelerates Git operations.

## The Performance Problem

When you run `git status` in a large repository, Git must:

1. **Read every file's metadata** — Check timestamps, sizes, permissions
2. **Compare against the index** — Determine if files have changed
3. **Scan for untracked files** — Walk every directory for new files
4. **Apply ignore rules** — Check each path against `.gitignore`

In a repository with 100,000+ files, this can take seconds or even minutes.

## The Solution: FSMonitor

Git's **fsmonitor** feature allows an external process to tell Git which files changed. Instead of scanning everything, Git asks "what changed since last time?" and only examines those files.

```
┌─────────────────────────────────────────────────────────┐
│                    Your Workflow                         │
│  $ git status                                            │
│       │                                                  │
│       ▼                                                  │
│  ┌─────────┐    "what changed?"    ┌─────────────────┐  │
│  │   Git   │ ───────────────────►  │  gity daemon    │  │
│  │         │ ◄─────────────────── │                 │  │
│  └─────────┘    "only foo.rs"      │  • file watcher │  │
│       │                            │  • dirty cache  │  │
│       ▼                            │  • maintenance  │  │
│  Only scans foo.rs                 └─────────────────┘  │
│  instead of 100,000 files                               │
└─────────────────────────────────────────────────────────┘
```

## Gity's Role

Gity implements Git's fsmonitor protocol and adds:

### 1. File Watching

Using OS-native APIs for maximum efficiency:

| Platform | Backend |
|----------|---------|
| Linux | inotify |
| macOS | FSEvents |
| Windows | ReadDirectoryChangesW |

The watcher monitors your entire repository and records every file change instantly.

### 2. Dirty Path Tracking

Changes are recorded with generation tokens:

```
Timeline:
────────────────────────────────────────────────────────►
    gen=1       gen=2       gen=3       gen=4
      │           │           │           │
   file.rs    src/lib.rs    (idle)    Cargo.toml
   changed    changed                  changed

Query with token=1:
  Returns: file.rs, src/lib.rs, Cargo.toml (gen=4)

Query with token=4:
  Returns: (empty list, gen=4)
```

### 3. FSMonitor Protocol

When Git runs, it invokes:

```bash
gity fsmonitor-helper 2 <previous-token>
```

Gity responds with:

```
<new-token>\0<changed-path-1>\0<changed-path-2>\0...
```

Git then only examines the listed paths.

### 4. Background Maintenance

During idle periods, Gity runs:

- `git maintenance run --task=prefetch` — Pre-fetch remote objects
- `git maintenance run --auto` — Commit-graph, loose-objects, incremental-repack

This keeps your repo optimized without interrupting your work.

## Data Flow

### Registration

When you run `gity register`:

```
┌──────────────────────────────────────────────────────────┐
│                    Registration Flow                      │
│                                                          │
│  1. gity register /path/to/repo                          │
│           │                                              │
│           ▼                                              │
│  2. Record repo in sled database                         │
│           │                                              │
│           ▼                                              │
│  3. Configure .git/config (fsmonitor, untrackedCache)    │
│           │                                              │
│           ▼                                              │
│  4. Start file watcher                                   │
│           │                                              │
│           ▼                                              │
│  5. Perform initial scan → cache file metadata           │
│           │                                              │
│           ▼                                              │
│  6. Schedule background maintenance                      │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### During Development

```
┌──────────────────────────────────────────────────────────┐
│                    Active Watching                        │
│                                                          │
│  File changed on disk                                    │
│           │                                              │
│           ▼                                              │
│  Watcher receives OS event                               │
│           │                                              │
│           ▼                                              │
│  Path added to dirty set, generation incremented         │
│           │                                              │
│           ▼                                              │
│  Event published on PUB/SUB channel                      │
│           │                                              │
│           ▼                                              │
│  Interested clients (IDEs, etc.) can react               │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### Git Status Query

```
┌──────────────────────────────────────────────────────────┐
│                    Status Query Flow                      │
│                                                          │
│  $ git status                                            │
│           │                                              │
│           ▼                                              │
│  Git reads core.fsmonitor config                         │
│           │                                              │
│           ▼                                              │
│  Git invokes: gity fsmonitor-helper 2 <token>            │
│           │                                              │
│           ▼                                              │
│  Gity daemon looks up changes since <token>              │
│           │                                              │
│           ▼                                              │
│  Returns: <new-token>\0path1\0path2\0...                 │
│           │                                              │
│           ▼                                              │
│  Git only stats/compares returned paths                  │
│           │                                              │
│           ▼                                              │
│  Fast result (milliseconds instead of seconds)           │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

## Why It's Fast

### Without Gity

```
git status (100,000 files):
├── Read index (5ms)
├── Stat every file (3,000ms)
├── Check untracked (2,000ms)
├── Apply ignores (500ms)
└── Total: ~5,500ms
```

### With Gity

```
git status (100,000 files, 5 changed):
├── Read index (5ms)
├── Query fsmonitor (2ms)
├── Stat 5 files (1ms)
├── Check untracked (cached) (2ms)
└── Total: ~10ms
```

## Reconciliation

If Gity was offline (system sleep, daemon restart), it may have missed events. On startup:

1. Compare stored watcher token with filesystem journal
2. If drift detected, schedule reconciliation scan
3. Scan compares cached mtimes vs disk
4. Synthetic events generated for any discrepancies
5. Normal watching resumes

This ensures correctness even after downtime.

## Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Watcher | `notify` crate | Cross-platform file watching |
| IPC | `async-nng` | Lightweight daemon communication |
| Database | `sled` | Crash-safe embedded storage |
| Replication | `rykv` | Share caches between worktrees |
| Runtime | Tokio | Async I/O and scheduling |

## Further Reading

- [FSMonitor Integration](fsmonitor.md) — Protocol details and edge cases
- [Architecture](architecture.md) — Component deep-dive

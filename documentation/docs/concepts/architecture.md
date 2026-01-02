# Architecture

Technical deep-dive into Gity's design and components.

## System Overview

Gity ships as a single executable with three interfaces:

```
┌─────────────────────────────────────────────────────┐
│                      gity                           │
│ ┌─────────┐   async-nng    ┌──────────────────────┐ │
│ │ CLI     │ ─────────────► │ Daemon Core          │ │
│ └─────────┘ ◄───────────── │ (watch/schedule/etc) │ │
│ ┌─────────┐   async-nng    └──────────────────────┘ │
│ │ Tray UI │ ─────────────►                          │
│ └─────────┘                                         │
└─────────────────────────────────────────────────────┘
```

- **CLI** — Parses commands like `gity register`, `gity list`
- **Daemon** — Long-running background service
- **Tray** — System tray UI for status and control

All three run unchanged on Windows, macOS, and Linux.

## Daemon Core

The daemon runs inside a Tokio runtime and hosts:

### Watch Service

Monitors filesystem changes using platform-specific backends:

| Platform | Backend |
|----------|---------|
| Linux | inotify |
| macOS | FSEvents |
| Windows | ReadDirectoryChangesW |

The watcher:

- Monitors the entire repository recursively
- Normalizes events into canonical change sets
- Updates the metadata store immediately
- Publishes notifications for interested clients

### Metadata Store

Uses **sled** for persistent storage:

- File classification and ignore results
- mtime/size snapshots
- Latest Git index token
- Job queues and metrics

Provides transactional reads/writes for safe coordination.

### Prefetch Scheduler

Manages background jobs:

- Fetch and prefetch
- Commit-graph refresh
- Garbage collection
- Incremental repack

Jobs are stored in sled with FIFO + priority semantics. Long tasks stream progress via PUB/SUB channels.

### Resource Monitor

Tracks system resources:

- CPU usage per repo
- Memory (RSS)
- File descriptors
- Sled cache size
- Job queue depths

When thresholds are exceeded, low-priority jobs are throttled.

## IPC Communication

Components communicate via **async-nng** sockets:

- **REQ/REP** — Request/response for commands
- **PUB/SUB** — Streaming notifications

Address: `tcp://127.0.0.1:7557` (configurable via `GITY_DAEMON_ADDR`)

Even though CLI/tray live in the same binary, they connect via sockets. This keeps the daemon resident while commands come and go.

## Local State Layout

```
$GITY_HOME/
├── config/           # Reserved for future settings
├── data/
│   ├── sled/         # Metadata database
│   └── status_cache/ # Cached status results
└── logs/
    └── daemon.log    # Daemon logs
```

Default locations:

- Unix: `~/.gity`
- Windows: `%APPDATA%\Gity`

## Repository Registration

When you register a repo:

1. **Record in sled** — Store `.git` path, ignore config digest, watcher tokens
2. **Spawn watcher** — Dedicated file watcher for this repo
3. **Configure Git** — Enable fsmonitor, untrackedCache, manyFiles
4. **Initial scan** — Populate metadata cache
5. **Schedule maintenance** — Queue background jobs

Unregistering reverses these steps.

## Data Flow

### 1. Discovery

On first run, the daemon:

- Scans the working tree
- Populates sled with file metadata
- Records initial generation token

### 2. Observation

When files change:

- Watcher receives OS event
- New state written to sled
- `IndexDelta` message published
- Paths marked "dirty"

### 3. Query Handling

When a client requests status:

- Look up current generation
- Compute suspect paths from cache
- Run targeted `git status -- <paths>` if needed
- Return new generation token

### 4. Background Work

Scheduler triggers maintenance:

- `git maintenance run --task=prefetch`
- `git maintenance run --auto`

Fetches go to `refs/prefetch/` without updating local refs.

### 5. Cache Sharing

If the same repo is registered in multiple locations (worktrees):

- `rykv` mirrors sled buckets tagged as shareable
- Warm caches reused without re-reading filesystem
- Sharing is local only—no network traffic

## Restart and Catch-up

On daemon startup:

1. Read all registrations from sled
2. For each repo, check watcher journal token
3. If tokens match, resume normal watching
4. If drift detected, schedule reconciliation scan
5. Reconciliation compares cached vs on-disk state
6. Synthetic events generated for discrepancies

This ensures correctness after downtime.

## Failure Handling

### Heartbeat Monitoring

The daemon writes heartbeat checkpoints to sled. If a client detects a stale heartbeat, it restarts the daemon.

### Watcher Gaps

If the watcher buffer overflows:

- Fallback to full scan
- Delta written as synthetic event
- Clients know verification occurred

### Job Retries

Prefetch jobs are idempotent. Retries only run if the previous attempt recorded failure.

## Technology Choices

### async-nng

Lightweight message-based IPC. REQ/REP + PUB/SUB patterns map naturally to command/notification needs.

### sled

Crash-safe embedded database with ordered keyspaces. No external service required.

### rykv

Simple async-friendly replication between sled stores on the same workstation. Mirrors caches between worktrees.

### Tokio

Async runtime for I/O and scheduling. Drives scheduler ticks and hosts the nng server.

## Extensibility

### Adding a Command

1. Define a serde-friendly message enum
2. Register a REQ handler in the daemon
3. Optionally emit telemetry via PUB sockets

### Adding a Job Type

1. Store job descriptor in sled
2. Provide executor function
3. Scheduler picks it up automatically

## Component Interactions

```
┌────────────────────────────────────────────────────────────┐
│                       Daemon Core                          │
│                                                            │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    │
│  │   Watcher   │───►│  Metadata   │◄───│  Scheduler  │    │
│  │   Service   │    │    Store    │    │             │    │
│  └─────────────┘    └─────────────┘    └─────────────┘    │
│         │                 ▲                   │            │
│         ▼                 │                   ▼            │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    │
│  │  PUB/SUB    │    │    Sled     │    │     Git     │    │
│  │  Channel    │    │   Database  │    │  Commands   │    │
│  └─────────────┘    └─────────────┘    └─────────────┘    │
│         │                                                  │
│         ▼                                                  │
│  ┌─────────────┐    ┌─────────────┐                       │
│  │     CLI     │    │    Tray     │                       │
│  │   Clients   │    │   Client    │                       │
│  └─────────────┘    └─────────────┘                       │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

## Further Reading

- [How It Works](how-it-works.md) — High-level operation
- [FSMonitor Integration](fsmonitor.md) — Protocol details
- [Commands Reference](../user-guide/commands.md) — CLI documentation

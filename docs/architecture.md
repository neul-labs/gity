# Architecture

Gity ships as a single executable that hosts three faces:

- **CLI** – parses commands like `gity register`, `gity list`, etc.
- **Daemon subcommand** – invoked via `gity daemon run` to keep the background service alive.
- **Tray client** – launched with `gity tray`, surfaces Info/Exit controls.

All three surfaces run unchanged on Windows, macOS, and Linux; the daemon selects the right watcher backend and tray bindings at startup. The daemon itself runs inside a Tokio runtime that drives the scheduler ticks and hosts the async-nng server on `tcp://127.0.0.1:7557` (overridable via `GITY_DAEMON_ADDR`).
Internally the CLI/tray faces connect to the daemon core through `async-nng` IPC sockets even though they live in the same binary. This keeps the runtime resident while short-lived commands come and go.

```
┌─────────────────────────────────────────────────────┐
│                      gity                           │
│ ┌─────────┐   async-nng    ┌──────────────────────┐ │
│ │ CLI     │ ─────────────► │ Daemon Core          │ │
│ └─────────┘ ◄───────────── │ (watch/schedule/etc) │ │
│ ┌─────────┐   async-nng    └──────────────────────┘ │
│ │ Tray UI │ ─────────────►                           │
│ └─────────┘                                         │
└─────────────────────────────────────────────────────┘
```

## Component Overview

### Watch Service

- Uses platform-specific backends (FSEvents on macOS, inotify on Linux, ReadDirectoryChangesW on Windows) via the `notify` crate and funnels those events through a unified watcher abstraction.
- Watches the entire repository directory recursively, including the `.git` directory.
- Normalizes events into a canonical change set and immediately updates the metadata store.
- Emits compact notifications through the async message bus so interested clients can react (e.g., IDEs updating decorations).

### FSMonitor Integration

Gity implements Git's fsmonitor protocol v2. When Git runs `git status`, it asks gity "what changed since token X?" and gity responds with only the changed paths.

**Critical: Working Tree Path Filtering**

The watcher sees all filesystem events, including changes inside `.git/` (HEAD updates, index changes, ref updates). However, Git's fsmonitor contract expects only **working tree paths**—the `.git` directory is managed by Git itself.

Before responding to fsmonitor queries, gity filters out all `.git` internal paths:

```rust
fn is_git_internal_path(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == ".git")
}
```

This filtering is essential for correct behavior during:

- **Branch switches**: Git updates `.git/HEAD` and checks out working tree files. Only the working tree changes are reported.
- **Commits**: Git updates `.git/index` and creates objects. These internal changes are not reported.
- **Rebases/merges**: Complex `.git` state changes are filtered; only resulting working tree changes are reported.

See [fsmonitor.md](fsmonitor.md) for complete protocol details and edge cases.

### Metadata & Cache Store

- Persists file classification, ignore results, mtime/size snapshots, and the latest Git index token inside `sled`.
- Provides transactional reads/writes so watchers and schedulers can coordinate safely.
- Replicates hot keys (e.g., last-known clean tree hash, packfile fingerprints) with `rykv` when the **same workstation** registers the repository in multiple locations (e.g., two worktrees). No metadata leaves the machine.

### Prefetch Scheduler

- Maintains a queue of background jobs (fetch, maintenance, GC, commit-graph refresh) stored in `sled` with FIFO + priority semantics.
- Schedules jobs with Tokio timers and records outcomes back into the store for observability.
- Talks to `git` via the `Command` API but never blocks IPC threads; long tasks stream progress updates over async-nng PUB/SUB channels.

### Resource Monitor

- Samples per-repo and global CPU, RSS, open file descriptors, sled cache usage, and queue depths.
- Exposes metrics via REQ handlers so commands like `gity list --stats` can print resource info.
- Applies back-pressure: when thresholds are exceeded, the monitor pauses low-priority jobs or reduces watcher aggressiveness until conditions recover.

### CLI & Tray Layers

- Each CLI invocation connects over async-nng REQ/REP sockets and multiplexes streaming updates via PUB/SUB.
- Commands (`gity list`, `gity status`, `gity logs`, etc.) map directly onto daemon handlers documented in `docs/commands.md`.
- `gity tray` launches a minimal UI that polls daemon info and exposes Info/Exit menu items; Info opens a summary window while Exit signals the daemon to shut down gracefully.
- These faces contain no Git logic; they marshal user intent, auto-start the daemon if needed, and render responses/logs.

## Local State Layout

- `$GITY_HOME` controls where the runtime stores everything; defaults to `~/.gity` on POSIX and `%APPDATA%\Gity` on Windows.
- Subdirectories:
  - `config/` – reserved for future editable settings.
  - `data/sled/` – sled database containing repo metadata, job queues, and metrics.
  - `logs/daemon.log` – plain-text log with lifecycle events and future diagnostics.
- CLI invocations create the directory tree on demand so even first-time commands succeed without manual setup.

## Repository Registration

1. `gity register /repo/path` records the repo inside `sled`, storing `.git` path, ignore config digest, and watcher tokens.
2. The runtime spawns a watcher + scheduler pair dedicated to that repo.
3. Registrations are local-only. Removing a repo via `gity unregister` deletes its sled keys but does not touch remote services.
4. As part of registration/unregistration we edit the repo’s `.git/config` to enable fsmonitor hooks, untracked-cache, commit-graph optimizations, and partial-clone settings for `origin`; unregistering removes those entries so repos stay clean.

## Data Flow

1. **Discovery** – On first run, the daemon scans the working tree once, populates `sled` with file metadata, and records a `generation` token.
2. **Observe** – Watch Service streams filesystem changes; for every event it:
   - Writes the new state into `sled`.
   - Publishes an `IndexDelta` message containing the paths that changed.
   - Marks the touched paths “dirty” so `gity status` can report cached results immediately.
3. **Answer commands** – When a client asks for `status`, the daemon:
   - Looks up the latest `generation`.
   - Computes “suspect paths” using cached metadata and ignore rules.
   - Runs targeted `git status -- <paths>` (or a full status if necessary) and records the resulting dirty list.
   - Returns the new generation token so clients can cache the output and skip redundant Git calls when nothing has changed.
4. **Prefetch** – Scheduler triggers `git maintenance run --task=prefetch` according to configured cadence, fetching into `refs/prefetch/` without updating local refs.
5. **Replication** – If the same repo is registered in multiple locations on this machine, `rykv` mirrors `sled` buckets tagged as shareable so warm caches can be reused without re-reading the filesystem.
6. **Introspection** – Resource metrics and structured logs are written to sled-backed rings so `gity list --stats`, `gity logs`, and the tray Info panel can query them without touching the filesystem.

## Restart & Catch-up

- On startup the runtime reads all repo registrations from `sled`.
- For each repo it checks the watcher journal token:
  - If the token matches the filesystem journal, normal watching resumes immediately.
  - If drift is detected (machine suspended, runtime offline too long), the runtime schedules a reconciliation scan that compares cached mtimes vs on-disk data and emits synthetic change events for any discrepancies.
- Once reconciliation finishes, timers and watchers resume as normal.

## Technology Choices

- **`async-nng`** – Gives us lightweight, message-based IPC inside the single binary. The REQ/REP + PUB/SUB patterns map naturally onto the command/notification duality we need.
- **`sled`** – Crash-safe embedded database with ordered keyspaces; ideal for storing per-path metadata and job queues without requiring an external service.
- **`rykv`** – Provides simple, async-friendly replication between sled-backed stores on the same workstation. We use it to mirror caches among sibling working trees so cold starts stay fast without network syncing.

## Background Triggers

- **Event-driven** – Watcher deltas enqueue verification tasks immediately.
- **Idle-time** – After N minutes of inactivity (configurable), the scheduler runs `git maintenance run --task=prefetch` followed by `git maintenance run --auto` which handles commit-graph, loose-objects, and incremental-repack as needed.
- **Branch changes** – When Git switches branches, the working tree files are updated by checkout. The watcher detects these working tree changes and marks them dirty. The `.git/HEAD` and `.git/refs` changes are seen by the watcher but filtered from fsmonitor output (see FSMonitor Integration above).
- **Manual commands** – `gity prefetch now`, `gity maintain`, `gity health`, and related subcommands insert jobs at the front of the queue or request diagnostic snapshots.
- **Resource budgets** – If the resource monitor reports high usage, the scheduler may delay prefetch jobs; when usage returns to normal, deferred jobs resume automatically.

## Failure Handling

- Every daemon writes heartbeat checkpoints to `sled`. If a client detects a stale heartbeat, it restarts the daemon.
- Watcher gaps (e.g., buffer overflow) trigger a fallback full scan; the delta is written as a synthetic event so clients know verification occurred.
- Prefetch jobs are idempotent; retries only run if the previous attempt recorded a non-success exit code.

## Extensibility

- Adding a new client command requires:
  1. Define a protobuf-like schema (we use serde-friendly enums).
  2. Register a REQ handler in the daemon.
  3. Optionally emit structured telemetry via PUB sockets.
- Adding a new scheduler job means storing a job descriptor in `sled` and providing an executor function.

## Related Documentation

- [fsmonitor.md](fsmonitor.md) – Git fsmonitor protocol details and edge cases
- [commands.md](commands.md) – CLI and daemon command reference
- [alternatives.md](alternatives.md) – Trade-offs against other approaches
- [process.md](process.md) – Contributing guidelines

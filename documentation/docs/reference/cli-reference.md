# CLI Reference

Complete command-line reference for Gity. Every command in this page is defined in `crates/gity-cli/src/lib.rs`.

## Synopsis

```
gity [OPTIONS] <COMMAND>
```

## Global Options

| Option | Description |
|--------|-------------|
| `--version` | Print version information |
| `--help` | Print help information |

## Repository Commands

### register

Register a repository for acceleration.

```
gity register <REPO_PATH>
```

**Example:**

```bash
gity register /home/user/projects/myrepo
gity register .
```

**Effects:**

- Records the repo in the daemon's sled metadata store.
- Edits the repo's `.git/config` to enable `core.fsmonitor`, `core.untrackedCache`, `feature.manyFiles`, and partial-clone settings.
- Starts a watcher and scheduler dedicated to the repo.
- Performs an initial scan to populate metadata.

---

### unregister

Stop accelerating a repository.

```
gity unregister <REPO_PATH>
```

**Effects:**

- Stops the watcher and removes scheduler entries.
- Removes the fsmonitor-related entries from the repo's `.git/config`.
- Drops cached metadata from sled.

---

### list

List all registered repositories.

```
gity list [--stats]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--stats` | Include CPU, RSS, cache size, and queue depth per repo. |

**Example output:**

```
/home/user/projects/frontend [0 jobs, status idle, gen 12]
/home/user/projects/backend  [1 jobs, status busy, gen 47]
```

---

### status

Print cached status for a repository.

```
gity status <REPO_PATH>
```

Returns either `clean (generation N)` or the repo path followed by the dirty paths.

---

### health

Run diagnostic checks against a registered repo.

```
gity health <REPO_PATH>
```

The output includes:

- Current generation token
- Pending job count
- Watcher state (`active` / `inactive`)
- Last filesystem event timestamp
- Dirty path count
- Sled integrity (`ok` / `ERROR`)
- Whether reconciliation is needed
- Whether resource throttling is active
- Next scheduled background job, if any

---

### changed

List files changed since a generation token.

```
gity changed <REPO_PATH> [--since <TOKEN>]
```

| Option | Description |
|--------|-------------|
| `--since <TOKEN>` | Numeric generation token (defaults to the daemon's last status token). |

**Example:**

```bash
gity changed /home/user/projects/myrepo
gity changed /home/user/projects/myrepo --since 42
```

---

### prefetch

Queue a background `git maintenance run --task=prefetch` job.

```
gity prefetch <REPO_PATH> [--now]
```

| Option | Description |
|--------|-------------|
| `--now` | Run immediately instead of waiting for the scheduler. |

---

### maintain

Force maintenance tasks (commit-graph refresh, GC, etc.) regardless of idle timers.

```
gity maintain <REPO_PATH>
```

---

### logs

Stream the daemon's structured logs for a repository.

```
gity logs <REPO_PATH> [--follow] [--limit <N>]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--follow` | off | Tail live output until interrupted. |
| `--limit <N>` | `50` | Number of entries to print. |

---

### events

Subscribe to the daemon's PUB socket and stream watcher events until interrupted.

```
gity events
```

---

### fsmonitor-helper

Implementation backing Git's `core.fsmonitor` hook. This is invoked by Git, not by users.

```
gity fsmonitor-helper [VERSION] [TOKEN] [--repo <PATH>]
```

| Argument / Option | Description |
|-------------------|-------------|
| `VERSION` | Protocol version (`1` or `2`, default `2`). |
| `TOKEN` | Opaque token from the previous response. |
| `--repo <PATH>` | Override the repo path (useful when invoked outside the working tree). |

Output is NUL-separated: `<new_token>\0<path1>\0<path2>\0...`.

---

## Daemon Subcommands

### daemon run

Run the daemon in the current process (foreground). Press `Ctrl+C` to exit.

```
gity daemon run
```

### daemon start

Start the daemon as a detached background process. Used implicitly by other CLI commands and by `gity tray` when the daemon isn't already running.

```
gity daemon start
```

### daemon stop

Signal the running daemon to shut down gracefully.

```
gity daemon stop
```

### daemon oneshot

Run the daemon for a single repository, service its queued jobs, then exit. Useful for CI pipelines that want a hot fsmonitor without leaving a persistent daemon behind.

```
gity daemon oneshot <REPO_PATH>
```

### daemon health

Fetch a health summary from the running daemon (repo count, pending jobs, uptime, per-repo generations).

```
gity daemon health
```

### daemon metrics

Print the latest daemon-wide metrics: CPU %, RSS, uptime, per-repo queue depth, and job counters (spawned/completed/failed) by kind.

```
gity daemon metrics
```

### daemon queue-job

Manually enqueue a background job.

```
gity daemon queue-job <REPO_PATH> <JOB>
```

| Job | Description |
|-----|-------------|
| `prefetch` | Background `git maintenance --task=prefetch`. |
| `maintenance` | Generic maintenance tasks (commit-graph, GC, etc.). |

---

## Database Subcommands

The `db` group operates on Gity's sled-backed storage.

### db stats

Show database statistics (file sizes and entry counts).

```
gity db stats
```

### db compact

Compact database files to reclaim space.

```
gity db compact
```

### db prune-logs

Prune old log entries from persistent storage.

```
gity db prune-logs [--older-than <DAYS>]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--older-than <DAYS>` | `7` | Remove log entries older than this many days. |

---

## Tray

### tray

Launch the cross-platform system tray UI.

```
gity tray
```

The tray menu provides an **Info** window (registered repos, watcher health, queue depths, resource usage) and an **Exit** item that is equivalent to `gity daemon stop`.

If the daemon is not running, `gity tray` starts it before showing the icon.

---

## Behaviour Notes

- CLI and tray commands implicitly call `gity daemon start` when the daemon is not already running.
- `gity list --stats` reports CPU %, RSS, cache size, and queued job count per repo.
- `gity logs` reads from the daemon's structured log ring stored in sled, so history survives daemon restarts.
- `gity health` is the safest command to run after long downtime: it surfaces reconciliation requirements, the next scheduled job, and active resource throttling.

See [Architecture](../concepts/architecture.md) for how each command interacts with the watcher, scheduler, and IPC layers.

# Commands

Complete reference for all Gity commands.

## Repository Commands

### register

Register a repository for acceleration.

```bash
gity register <path>
```

**Arguments:**

- `<path>` — Path to the Git repository

**Example:**

```bash
gity register /path/to/repo
gity register .  # Current directory
```

---

### unregister

Stop accelerating a repository and clean up.

```bash
gity unregister <path>
```

**Arguments:**

- `<path>` — Path to the registered repository

**Example:**

```bash
gity unregister /path/to/repo
```

---

### list

Show all registered repositories.

```bash
gity list [--stats]
```

**Options:**

- `--stats` — Include resource usage statistics (CPU, memory, cache size, job queue)

**Example:**

```bash
gity list
gity list --stats
```

---

### status

Get a fast status summary for a repository.

```bash
gity status <path>
```

Returns clean/dirty state and changed paths using cached metadata.

**Example:**

```bash
gity status /path/to/repo
```

---

### health

Run diagnostic checks on a repository.

```bash
gity health <path>
```

**Output includes:**

- Watcher status
- Generation token
- Dirty path count
- Scheduled jobs
- Resource throttling status
- Sled database integrity

**Example:**

```bash
gity health /path/to/repo
```

---

### changed

List files changed since a specific token.

```bash
gity changed <path> [--since <token>]
```

**Options:**

- `--since <token>` — Watcher token (defaults to last status token)

**Example:**

```bash
gity changed /path/to/repo
gity changed /path/to/repo --since 42
```

---

### prefetch

Trigger background fetch and maintenance.

```bash
gity prefetch <path> [now]
```

**Arguments:**

- `now` — Jump to front of job queue

**Example:**

```bash
gity prefetch /path/to/repo
gity prefetch /path/to/repo now  # Immediate
```

---

### maintain

Force maintenance tasks regardless of idle timers.

```bash
gity maintain <path>
```

Runs commit-graph refresh, GC, and other maintenance.

**Example:**

```bash
gity maintain /path/to/repo
```

---

### logs

Stream daemon logs for a repository.

```bash
gity logs <path> [--follow] [--limit N]
```

**Options:**

- `--follow` — Tail live output
- `--limit N` — Number of entries (default: 50)

**Example:**

```bash
gity logs /path/to/repo
gity logs /path/to/repo --follow
gity logs /path/to/repo --limit 100
```

---

### events

Stream file watcher events in real-time.

```bash
gity events
```

Subscribes to the daemon's PUB socket and shows notifications until interrupted.

**Example:**

```bash
gity events
# Press Ctrl+C to stop
```

---

## Daemon Commands

### daemon run

Start the daemon in the foreground.

```bash
gity daemon run [--config PATH]
```

**Options:**

- `--config PATH` — Path to configuration file

Press `Ctrl+C` to exit.

---

### daemon start

Start the daemon as a background process.

```bash
gity daemon start
```

Used by installers and `gity tray`. Most commands auto-start the daemon if needed.

---

### daemon stop

Stop the running daemon gracefully.

```bash
gity daemon stop
```

---

### daemon oneshot

Start daemon, service jobs for a repo, then exit.

```bash
gity daemon oneshot <path>
```

Useful for CI/CD pipelines.

**Example:**

```bash
gity daemon oneshot /path/to/repo
git status
git diff --cached
```

---

### daemon metrics

Print current resource metrics.

```bash
gity daemon metrics
```

Shows CPU, RSS, file descriptors, cache usage, and job queue depths.

---

## System Tray

### tray

Launch the system tray UI.

```bash
gity tray
```

The tray menu provides:

- **Info** — Window with repo status, watcher health, queue depths
- **Exit** — Equivalent to `gity daemon stop`

If the daemon isn't running, `gity tray` starts it automatically.

---

## Internal Commands

### fsmonitor-helper

Git fsmonitor protocol implementation.

```bash
gity fsmonitor-helper <version> <token>
```

!!! note
    This command is invoked by Git, not directly by users. It implements the fsmonitor protocol to report changed files.

**Arguments:**

- `<version>` — Protocol version (must be `2`)
- `<token>` — Opaque token from previous response

---

## Command Behavior

- **Auto-start daemon** — CLI commands automatically start the daemon if not running
- **IPC communication** — Commands communicate with the daemon via async-nng sockets
- **Streaming output** — Commands like `events` and `logs --follow` stream until interrupted

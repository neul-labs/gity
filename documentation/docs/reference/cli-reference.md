# CLI Reference

Complete command-line reference for Gity.

## Synopsis

```
gity [OPTIONS] <COMMAND>
```

## Global Options

| Option | Description |
|--------|-------------|
| `--version` | Print version information |
| `--help` | Print help information |

## Commands

### register

Register a repository for acceleration.

```
gity register <PATH>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to the Git repository |

**Example:**

```bash
gity register /home/user/projects/myrepo
gity register .
```

**Effects:**

- Starts file watcher for the repository
- Configures Git fsmonitor settings
- Performs initial cache population
- Schedules background maintenance

---

### unregister

Stop accelerating a repository.

```
gity unregister <PATH>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to the registered repository |

**Example:**

```bash
gity unregister /home/user/projects/myrepo
```

**Effects:**

- Stops file watcher
- Removes Git fsmonitor configuration
- Cleans up cached metadata

---

### list

List all registered repositories.

```
gity list [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--stats` | Include resource usage statistics |

**Example:**

```bash
gity list
gity list --stats
```

**Output:**

```
Registered repositories:
  /home/user/projects/frontend
  /home/user/projects/backend
```

With `--stats`:

```
Registered repositories:
  /home/user/projects/frontend
    CPU: 0.1%  RSS: 12MB  Cache: 5MB  Jobs: 0
  /home/user/projects/backend
    CPU: 0.2%  RSS: 15MB  Cache: 8MB  Jobs: 1
```

---

### status

Get fast status summary.

```
gity status <PATH>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to the registered repository |

**Example:**

```bash
gity status /home/user/projects/myrepo
```

**Output:**

Returns clean/dirty state and list of changed paths.

---

### health

Run diagnostic checks.

```
gity health <PATH>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to the registered repository |

**Example:**

```bash
gity health /home/user/projects/myrepo
```

**Output includes:**

- Watcher status
- Generation token
- Dirty path count
- Scheduled jobs
- Resource throttling status

---

### changed

List files changed since a token.

```
gity changed <PATH> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to the registered repository |

**Options:**

| Option | Description |
|--------|-------------|
| `--since <TOKEN>` | Watcher token (default: last status token) |

**Example:**

```bash
gity changed /home/user/projects/myrepo
gity changed /home/user/projects/myrepo --since 42
```

---

### prefetch

Trigger background fetch.

```
gity prefetch <PATH> [now]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to the registered repository |
| `now` | Optional: jump to front of queue |

**Example:**

```bash
gity prefetch /home/user/projects/myrepo
gity prefetch /home/user/projects/myrepo now
```

---

### maintain

Force maintenance tasks.

```
gity maintain <PATH>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to the registered repository |

**Example:**

```bash
gity maintain /home/user/projects/myrepo
```

**Tasks run:**

- Commit-graph refresh
- Garbage collection
- Other maintenance operations

---

### logs

Stream daemon logs.

```
gity logs <PATH> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to the registered repository |

**Options:**

| Option | Description |
|--------|-------------|
| `--follow` | Tail live output |
| `--limit <N>` | Number of entries (default: 50) |

**Example:**

```bash
gity logs /home/user/projects/myrepo
gity logs /home/user/projects/myrepo --follow
gity logs /home/user/projects/myrepo --limit 100
```

---

### events

Stream file watcher events.

```
gity events
```

Subscribes to the daemon's PUB socket and streams notifications.

**Example:**

```bash
gity events
# Press Ctrl+C to stop
```

---

### daemon run

Start daemon in foreground.

```
gity daemon run [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--config <PATH>` | Path to configuration file |

**Example:**

```bash
gity daemon run
gity daemon run --config /path/to/config.toml
```

Press `Ctrl+C` to exit.

---

### daemon start

Start daemon in background.

```
gity daemon start
```

**Example:**

```bash
gity daemon start
```

---

### daemon stop

Stop the daemon gracefully.

```
gity daemon stop
```

**Example:**

```bash
gity daemon stop
```

---

### daemon oneshot

Start daemon, service a repo, then exit.

```
gity daemon oneshot <PATH>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to the repository |

**Example:**

```bash
gity daemon oneshot /home/user/projects/myrepo
git status
git diff --cached
```

Useful for CI/CD pipelines.

---

### daemon metrics

Print current resource metrics.

```
gity daemon metrics
```

**Example:**

```bash
gity daemon metrics
```

**Output includes:**

- CPU usage
- Memory (RSS)
- File descriptors
- Cache usage
- Job queue depths

---

### tray

Launch system tray UI.

```
gity tray
```

**Example:**

```bash
gity tray
```

The tray provides:

- Repository status overview
- Health information
- Exit option (stops daemon)

---

### fsmonitor-helper

Git fsmonitor protocol implementation.

```
gity fsmonitor-helper <VERSION> <TOKEN>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `VERSION` | Protocol version (must be `2`) |
| `TOKEN` | Opaque token from previous response |

!!! note
    This command is invoked by Git, not directly by users.

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Repository not found |
| 4 | Daemon not running |
| 5 | Permission denied |

## Shell Completion

Generate shell completion scripts:

```bash
# Bash
gity completions bash > /etc/bash_completion.d/gity

# Zsh
gity completions zsh > ~/.zsh/completion/_gity

# Fish
gity completions fish > ~/.config/fish/completions/gity.fish

# PowerShell
gity completions powershell > gity.ps1
```

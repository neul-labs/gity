# Environment Variables

Gity is configured exclusively through environment variables — there is no configuration file format today. The variables below are the ones the binary actually reads at runtime.

## Core Variables

### GITY_HOME

Location of Gity's data directory (database, status cache, logs).

| Platform | Default |
|----------|---------|
| Linux / macOS | `~/.gity` |
| Windows | `%APPDATA%\Gity` |

**Usage:**

```bash
# Linux/macOS
export GITY_HOME=/custom/path/.gity

# Windows PowerShell
$env:GITY_HOME = "$env:APPDATA\Gity"

# Windows CMD
set GITY_HOME=D:\gity-data
```

**Directory structure:**

```
$GITY_HOME/
├── config/           # Reserved for future settings
├── data/
│   ├── sled/         # Metadata database
│   └── status_cache/ # Cached status results
└── logs/
    └── daemon.log    # Daemon logs
```

The CLI creates this tree on demand, so first-time commands succeed without manual setup.

---

### GITY_DAEMON_ADDR

IPC socket address used by the daemon's REQ/REP control plane. The CLI, tray, and daemon must agree on this address.

| Default |
|---------|
| `tcp://127.0.0.1:7557` |

**Usage:**

```bash
# Change port
export GITY_DAEMON_ADDR=tcp://127.0.0.1:8888

# Use an in-process address (advanced)
export GITY_DAEMON_ADDR=inproc://gity-daemon
```

When the CLI launches the daemon (`gity daemon start`), it propagates this value to the daemon process automatically.

---

### GITY_EVENTS_ADDR

Address for the daemon's PUB/SUB notification socket. `gity events` and `gity logs --follow` subscribe here.

| Default |
|---------|
| The daemon's default events address (set in code) |

**Usage:**

```bash
export GITY_EVENTS_ADDR=tcp://127.0.0.1:7558
```

Override this only when you have a reason to relocate the notification stream (e.g., port conflicts).

---

### GITY_FSMONITOR_HELPER

Overrides the command string written into `core.fsmonitor` during `gity register`.

By default Gity uses `gity fsmonitor-helper`, which works whenever the `gity` binary is on `PATH`. Set this variable when you need an absolute path or a custom invocation (for example, when Git runs in an environment with a different `PATH`):

```bash
export GITY_FSMONITOR_HELPER="/usr/local/bin/gity fsmonitor-helper"
```

The override is read by `gity register` and recorded in the repo's `.git/config`.

---

## Logging

### RUST_LOG

Standard [`tracing` / `env_logger`](https://docs.rs/env_logger/) filter string. Gity emits structured logs through this filter.

| Value | Description |
|-------|-------------|
| `error` | Only errors |
| `warn` | Warnings and errors |
| `info` | Informational messages |
| `debug` | Debug information |
| `trace` | Detailed tracing |

**Usage:**

```bash
# Verbose daemon logging
RUST_LOG=info gity daemon run

# Per-module filtering
export RUST_LOG=gity=debug,sled=warn
```

---

## Standard Variables Gity Honors

| Variable | Effect |
|----------|--------|
| `HOME` (Unix) / `USERPROFILE` (Windows) | Used to compute the default `GITY_HOME`. |
| `APPDATA` (Windows) | Used to compute the default `GITY_HOME`. |
| `PATH` | Must contain `gity` and `git`. `core.fsmonitor` is invoked by Git, so `gity` must be discoverable in Git's environment too. |
| `GIT_DIR`, `GIT_WORK_TREE` | Standard Git variables; honored transparently because Gity shells out to `git`. |

---

## Example Configurations

### Development

```bash
export RUST_LOG=gity=debug
export GITY_HOME=/tmp/gity-dev
gity daemon run
```

### CI / Ephemeral runners

```bash
export GITY_HOME="$RUNNER_TEMP/gity"
export RUST_LOG=warn
gity daemon oneshot "$GITHUB_WORKSPACE"
```

### Shared workstation, multiple users

```bash
# Default behavior: each user gets their own ~/.gity
# To force isolation explicitly:
export GITY_HOME="$HOME/.gity"
```

---

## Setting Variables Permanently

### Linux / macOS

Add to `~/.bashrc`, `~/.zshrc`, or `~/.profile`:

```bash
export GITY_HOME="$HOME/.gity"
export RUST_LOG=info
```

### Windows

PowerShell profile (`$PROFILE`):

```powershell
$env:GITY_HOME = "$env:APPDATA\Gity"
```

Or set via **System Properties → Environment Variables** for a permanent system-wide value.

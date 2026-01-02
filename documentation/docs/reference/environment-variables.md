# Environment Variables

Configuration via environment variables.

## Core Variables

### GITY_HOME

Location of Gity's data directory.

| Default (Unix) | Default (Windows) |
|----------------|-------------------|
| `~/.gity` | `%APPDATA%\Gity` |

**Usage:**

```bash
# Linux/macOS
export GITY_HOME=/custom/path/.gity

# Windows PowerShell
$env:GITY_HOME = "D:\gity-data"

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

---

### GITY_DAEMON_ADDR

IPC socket address for daemon communication.

| Default |
|---------|
| `tcp://127.0.0.1:7557` |

**Usage:**

```bash
# Change port
export GITY_DAEMON_ADDR=tcp://127.0.0.1:8888

# Use Unix socket (Linux/macOS only)
export GITY_DAEMON_ADDR=ipc:///tmp/gity.sock
```

!!! warning
    Both daemon and CLI must use the same address.

---

## Logging Variables

### RUST_LOG

Control log verbosity (standard Rust logging).

| Value | Description |
|-------|-------------|
| `error` | Only errors |
| `warn` | Warnings and errors |
| `info` | Informational messages (default) |
| `debug` | Debug information |
| `trace` | Detailed tracing |

**Usage:**

```bash
# More verbose logging
export RUST_LOG=debug

# Per-module logging
export RUST_LOG=gity=debug,sled=warn
```

---

### GITY_LOG_FORMAT

Log output format.

| Value | Description |
|-------|-------------|
| `pretty` | Human-readable (default for tty) |
| `json` | JSON format (default for non-tty) |
| `compact` | Compact single-line format |

**Usage:**

```bash
export GITY_LOG_FORMAT=json
```

---

## Git Variables

These standard Git variables affect Gity's behavior:

### GIT_DIR

Override the Git directory location.

```bash
export GIT_DIR=/path/to/.git
```

### GIT_WORK_TREE

Override the working tree location.

```bash
export GIT_WORK_TREE=/path/to/worktree
```

---

## System Variables

### PATH

Ensure `gity` and `git` are accessible.

```bash
# Add Gity to PATH
export PATH="/usr/local/bin:$PATH"

# Verify
which gity
which git
```

### HOME (Unix) / USERPROFILE (Windows)

Used to determine default `GITY_HOME` location.

---

## Platform-Specific Variables

### Linux

#### XDG_DATA_HOME

If set, Gity may use this for data storage in future versions.

```bash
export XDG_DATA_HOME=~/.local/share
```

### macOS

No additional variables.

### Windows

#### APPDATA

Default location for `GITY_HOME` on Windows.

Typically: `C:\Users\<username>\AppData\Roaming`

---

## Debug Variables

### GITY_DEBUG

Enable debug mode with additional diagnostics.

```bash
export GITY_DEBUG=1
```

### GITY_TRACE

Enable detailed tracing for troubleshooting.

```bash
export GITY_TRACE=1
```

---

## Example Configurations

### Development Setup

```bash
# More verbose logging
export RUST_LOG=gity=debug

# Custom data directory (not backed up)
export GITY_HOME=/tmp/gity-dev
```

### CI/CD Setup

```bash
# JSON logs for parsing
export GITY_LOG_FORMAT=json

# Minimal logging
export RUST_LOG=warn

# Ephemeral data directory
export GITY_HOME=/tmp/gity-ci
```

### Production Setup

```bash
# Standard logging
export RUST_LOG=info

# Custom location with more space
export GITY_HOME=/var/lib/gity
```

### Multiple Users

For shared systems with multiple users:

```bash
# Each user gets their own data directory (default behavior)
# GITY_HOME defaults to ~/.gity per user

# Or use a shared location with user subdirectories
export GITY_HOME=/shared/gity/$USER
```

---

## Setting Variables Permanently

### Linux/macOS

Add to `~/.bashrc`, `~/.zshrc`, or `~/.profile`:

```bash
export GITY_HOME=~/.gity
export RUST_LOG=info
```

### Windows

**System Properties:**

1. Search for "Environment Variables"
2. Click "Edit the system environment variables"
3. Click "Environment Variables"
4. Add user or system variables

**PowerShell Profile:**

Add to `$PROFILE`:

```powershell
$env:GITY_HOME = "$env:APPDATA\Gity"
```

---

## Precedence

Environment variables take precedence over:

1. Default values
2. Configuration files (when supported)

Command-line arguments take precedence over environment variables.

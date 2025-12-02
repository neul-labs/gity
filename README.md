# Gitz

**Make large Git repositories feel instant.**

Gitz is a lightweight daemon that accelerates Git operations on large repositories. It watches your files, maintains warm caches, and runs background maintenance—so `git status` and other commands stay fast even in repos with millions of files.

## The Problem

In large repositories, everyday Git commands become painfully slow:

```bash
$ time git status
# ... 8 seconds later ...
nothing to commit, working tree clean
```

This happens because Git must scan the entire working tree, check file timestamps, and compare against the index. The larger your repo, the worse it gets.

## The Solution

Gitz runs a background daemon that:

1. **Watches your files** — Detects changes instantly via OS-native file watchers (inotify, FSEvents, ReadDirectoryChangesW)
2. **Tells Git what changed** — Implements Git's fsmonitor protocol so Git only checks files that actually changed
3. **Keeps objects fresh** — Runs `git maintenance` during idle periods so fetches stay fast
4. **Caches results** — Remembers status results and serves them instantly when nothing changed

The result: `git status` in milliseconds instead of seconds.

## Quick Start

```bash
# Install
cargo install --path crates/gitz

# Register your large repo (one-time setup)
gitz register /path/to/large-repo

# That's it! Git commands are now accelerated
cd /path/to/large-repo
git status  # Fast!
```

The daemon starts automatically when needed. For manual control:

```bash
gitz daemon start   # Start in background
gitz daemon stop    # Stop gracefully
gitz list           # See registered repos
gitz health <repo>  # Check repo health
```

## Use Cases

### Monorepo Development

You work in a large monorepo with thousands of packages. Every `git status` takes 10+ seconds, breaking your flow.

```bash
gitz register ~/work/monorepo
cd ~/work/monorepo
git status  # Now instant
```

### Multiple Worktrees

You have several worktrees of the same repo for parallel feature development.

```bash
gitz register ~/projects/app
gitz register ~/projects/app-feature-x
gitz register ~/projects/app-bugfix-y
# Caches are shared between related repos on the same machine
```

### CI/CD Optimization

Your CI builds clone large repos and run status checks. Use oneshot mode to accelerate without a persistent daemon:

```bash
gitz daemon oneshot /path/to/repo
git status
git diff --cached
```

### IDE Integration

IDEs constantly poll `git status` for file decorations. With gitz, these polls return instantly:

```bash
# IDE calls this repeatedly
git status --porcelain  # Returns in <10ms with gitz
```

## How It Works

```
┌─────────────────────────────────────────────────────────┐
│                    Your Workflow                         │
│  $ git status                                            │
│       │                                                  │
│       ▼                                                  │
│  ┌─────────┐    "what changed?"    ┌─────────────────┐  │
│  │   Git   │ ───────────────────►  │  gitz daemon    │  │
│  │         │ ◄─────────────────── │                 │  │
│  └─────────┘    "only foo.rs"      │  • file watcher │  │
│       │                            │  • dirty cache  │  │
│       ▼                            │  • maintenance  │  │
│  Only scans foo.rs                 └─────────────────┘  │
│  instead of 100,000 files                               │
└─────────────────────────────────────────────────────────┘
```

When you register a repo, gitz:

1. Starts watching the working tree for file changes
2. Configures Git to use `gitz fsmonitor-helper` as the fsmonitor
3. Tracks which files changed since the last query
4. Schedules background `git maintenance` during idle periods

When Git runs `git status`:

1. Git asks the fsmonitor "what changed since token X?"
2. Gitz returns only the files that actually changed
3. Git scans just those files instead of the entire tree

See [docs/architecture.md](docs/architecture.md) for the full technical deep-dive.

## Commands

| Command | Description |
|---------|-------------|
| `gitz register <path>` | Start accelerating a repository |
| `gitz unregister <path>` | Stop accelerating and clean up |
| `gitz list [--stats]` | Show registered repos and health |
| `gitz status <path>` | Fast status summary |
| `gitz health <path>` | Detailed diagnostics |
| `gitz prefetch <path>` | Trigger background fetch |
| `gitz maintain <path>` | Trigger maintenance tasks |
| `gitz daemon start` | Start daemon in background |
| `gitz daemon stop` | Stop daemon gracefully |
| `gitz tray` | Launch system tray UI |

See [docs/commands.md](docs/commands.md) for complete reference.

## Requirements

- Git 2.37+ (for fsmonitor-watchman protocol v2)
- Rust 1.75+ (to build from source)
- Linux, macOS, or Windows

## Installation

### From Source

```bash
cargo install --path crates/gitz
```

### Platform Packages

- **Linux**: `.deb` package or AppImage (see [releases](../../releases))
- **macOS**: `.pkg` installer (see [releases](../../releases))
- **Windows**: MSI installer (see [releases](../../releases))

## Configuration

Gitz stores data in `$GITZ_HOME` (defaults to `~/.gitz` on Unix, `%APPDATA%\Gitz` on Windows):

```
~/.gitz/
├── data/
│   ├── sled/           # Metadata database
│   └── status_cache/   # Cached status results
└── logs/
    └── daemon.log      # Daemon logs
```

Environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `GITZ_HOME` | Data directory | `~/.gitz` |
| `GITZ_DAEMON_ADDR` | IPC address | `tcp://127.0.0.1:7557` |

## Documentation

| Document | Description |
|----------|-------------|
| [architecture.md](docs/architecture.md) | System design, data flow, component details |
| [fsmonitor.md](docs/fsmonitor.md) | Git fsmonitor integration and edge cases |
| [commands.md](docs/commands.md) | Complete CLI reference |
| [alternatives.md](docs/alternatives.md) | Comparison with other approaches |
| [process.md](docs/process.md) | Contributing guidelines |

## Troubleshooting

### Git status is still slow

Check that the repo is registered and healthy:

```bash
gitz list
gitz health /path/to/repo
```

Verify fsmonitor is configured:

```bash
git config core.fsmonitor
# Should show: gitz fsmonitor-helper
```

### Daemon won't start

Check logs:

```bash
cat ~/.gitz/logs/daemon.log
```

Verify the port is available:

```bash
lsof -i :7557
```

### Changes not detected

The file watcher might have missed events (can happen after sleep/hibernate). Force a refresh:

```bash
gitz health /path/to/repo  # Shows if reconciliation is needed
git status                  # Triggers full scan if needed
```

## Contributing

1. Read [docs/process.md](docs/process.md) for conventions
2. Open a draft PR early for feedback
3. Run tests: `cargo test --all`

## License

MIT

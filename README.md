# Gity

**Make large Git repositories feel instant.**

Gity is a lightweight, cross-platform daemon that accelerates Git operations on large repositories. A single binary runs on **Linux**, **macOS**, and **Windows**—watching your files, maintaining warm caches, and running background maintenance so `git status` stays fast even in repos with millions of files.

[![Crates.io](https://img.shields.io/crates/v/gity)](https://crates.io/crates/gity)
[![npm](https://img.shields.io/npm/v/gity-cli)](https://www.npmjs.com/package/gity-cli)
[![PyPI](https://img.shields.io/pypi/v/gity-cli)](https://pypi.org/project/gity-cli/)
[![Documentation](https://img.shields.io/badge/docs-neullabs.com-blue)](http://docs.neullabs.com/gity)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)
[![Cross-platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-blue)](http://docs.neullabs.com/gity)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

## Cross-Platform Support

| Platform | File Watcher | Status |
|----------|--------------|--------|
| **Linux** | inotify | ✅ Full support |
| **macOS** | FSEvents | ✅ Full support |
| **Windows** | ReadDirectoryChangesW | ✅ Full support |
| **WSL2** | inotify (Linux FS only) | ⚠️ [See notes](docs/fsmonitor.md#wsl2-windows-subsystem-for-linux) |

One binary, same features everywhere. No platform-specific configuration needed.

## The Problem

In large repositories, everyday Git commands become painfully slow:

```bash
$ time git status
# ... 8 seconds later ...
nothing to commit, working tree clean
```

This happens because Git must scan the entire working tree, check file timestamps, and compare against the index. The larger your repo, the worse it gets.

## The Solution

Gity runs a background daemon that:

1. **Watches your files** — Detects changes instantly via OS-native file watchers (inotify, FSEvents, ReadDirectoryChangesW)
2. **Tells Git what changed** — Implements Git's fsmonitor protocol so Git only checks files that actually changed
3. **Keeps objects fresh** — Runs `git maintenance` during idle periods so fetches stay fast
4. **Caches results** — Remembers status results and serves them instantly when nothing changed

The result: `git status` in milliseconds instead of seconds.

## Quick Start

```bash
# Install
cargo install gity

# Register your large repo (one-time setup)
gity register /path/to/large-repo

# That's it! Git commands are now accelerated
cd /path/to/large-repo
git status  # Fast!
```

The daemon starts automatically when needed. For manual control:

```bash
gity daemon start   # Start in background
gity daemon stop    # Stop gracefully
gity list           # See registered repos
gity health <repo>  # Check repo health
```

## Installation

### From Source (Rust)

```bash
cargo install gity
```

With system tray support:

```bash
cargo install gity --features tray
```

### Via Homebrew (macOS & Linux)

```bash
brew tap neul-labs/tap
brew install gity
```

### Via npm

```bash
npm install -g gity-cli
```

Or with npx:

```bash
npx gity-cli register /path/to/large-repo
```

### Via pip

```bash
pip install gity
```

### Platform Packages

- **Linux**: `.deb` package (see [releases](../../releases))
- **macOS**: `.pkg` installer (see [releases](../../releases))
- **Windows**: MSI installer (see [releases](../../releases))
- **Snap**: `snap install gity`
- **Chocolatey**: `choco install gity`

## Supply Chain Security

All release artifacts are built and published via GitHub Actions with **OIDC-based trusted publishing** (no long-lived secrets) and **signed attestations**:

- **crates.io** — Published via [Trusted Publishing](https://crates.io/docs/trusted-publishing) (OIDC)
- **PyPI** — Published via [Trusted Publishing](https://docs.pypi.org/trusted-publishers/) (OIDC)
- **npm** — Published via [Trusted Publishing](https://docs.npmjs.com/generating-provenance-statements) with automatic provenance attestations
- **GitHub Releases** — Binaries are attested with `actions/attest-build-provenance`

Verify a release binary:

```bash
gh attestation verify gity-0.1.2-x86_64-unknown-linux-gnu.tar.gz --owner neul-labs
```

## Use Cases

### Monorepo Development

You work in a large monorepo with thousands of packages. Every `git status` takes 10+ seconds, breaking your flow.

```bash
gity register ~/work/monorepo
cd ~/work/monorepo
git status  # Now instant
```

### Multiple Worktrees

You have several worktrees of the same repo for parallel feature development.

```bash
gity register ~/projects/app
gity register ~/projects/app-feature-x
gity register ~/projects/app-bugfix-y
# Caches are shared between related repos on the same machine
```

### CI/CD Optimization

Your CI builds clone large repos and run status checks. Use oneshot mode to accelerate without a persistent daemon:

```bash
gity daemon oneshot /path/to/repo
git status
git diff --cached
```

### IDE Integration

IDEs constantly poll `git status` for file decorations. With gity, these polls return instantly:

```bash
# IDE calls this repeatedly
git status --porcelain  # Returns in <10ms with gity
```

## How It Works

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

When you register a repo, gity:

1. Starts watching the working tree for file changes
2. Configures Git to use `gity fsmonitor-helper` as the fsmonitor
3. Tracks which files changed since the last query
4. Schedules background `git maintenance` during idle periods

When Git runs `git status`:

1. Git asks the fsmonitor "what changed since token X?"
2. Gity returns only the files that actually changed
3. Git scans just those files instead of the entire tree

See [docs/architecture.md](docs/architecture.md) for the full technical deep-dive.

## Commands

| Command | Description |
|---------|-------------|
| `gity register <path>` | Start accelerating a repository |
| `gity unregister <path>` | Stop accelerating and clean up |
| `gity list [--stats]` | Show registered repos and health |
| `gity status <path>` | Fast status summary |
| `gity health <path>` | Detailed diagnostics |
| `gity prefetch <path>` | Trigger background fetch |
| `gity maintain <path>` | Trigger maintenance tasks |
| `gity daemon start` | Start daemon in background |
| `gity daemon stop` | Stop daemon gracefully |
| `gity tray` | Launch system tray UI |

See [docs/commands.md](docs/commands.md) for complete reference.

## Requirements

- Git 2.37+ (for fsmonitor-watchman protocol v2)
- Rust 1.75+ (to build from source)
- Linux, macOS, or Windows

## Configuration

Gity stores data in `$GITY_HOME` (defaults to `~/.gity` on Unix, `%APPDATA%\Gity` on Windows):

```
~/.gity/
├── data/
│   ├── sled/           # Metadata database
│   └── status_cache/   # Cached status results
└── logs/
    └── daemon.log      # Daemon logs
```

Environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `GITY_HOME` | Data directory | `~/.gity` |
| `GITY_DAEMON_ADDR` | IPC address | `tcp://127.0.0.1:7557` |

## Documentation

| Document | Description |
|----------|-------------|
| [architecture.md](docs/architecture.md) | System design, data flow, component details |
| [fsmonitor.md](docs/fsmonitor.md) | Git fsmonitor integration and edge cases |
| [commands.md](docs/commands.md) | Complete CLI reference |
| [alternatives.md](docs/alternatives.md) | Comparison with other approaches |
| [process.md](docs/process.md) | Contributing guidelines |
| [release-requirements.md](docs/release-requirements.md) | Release process and OIDC setup |

## Troubleshooting

### Git status is still slow

Check that the repo is registered and healthy:

```bash
gity list
gity health /path/to/repo
```

Verify fsmonitor is configured:

```bash
git config core.fsmonitor
# Should show: gity fsmonitor-helper
```

### Daemon won't start

Check logs:

```bash
cat ~/.gity/logs/daemon.log
```

Verify the port is available:

```bash
lsof -i :7557
```

### Changes not detected

The file watcher might have missed events (can happen after sleep/hibernate). Force a refresh:

```bash
gity health /path/to/repo  # Shows if reconciliation is needed
git status                  # Triggers full scan if needed
```

### WSL2: Events not working

If using WSL2, file watching only works for repos on the **Linux filesystem**:

```bash
# Good - works correctly
gity register ~/code/repo

# Bad - inotify doesn't work across 9P filesystem
gity register /mnt/c/Users/me/repo
```

See [docs/fsmonitor.md](docs/fsmonitor.md#wsl2-windows-subsystem-for-linux) for details.

## Contributing

1. Read [docs/process.md](docs/process.md) for conventions
2. Open a draft PR early for feedback
3. Run tests: `cargo test --all`

## License

MIT

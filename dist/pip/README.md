# gity

**Make large Git repositories feel instant.**

Gity is a lightweight, cross-platform daemon that accelerates Git operations on large repositories.

## Installation

```bash
pip install gity
```

## Quick Start

```bash
# Register your large repo (one-time setup)
gity register /path/to/large-repo

# That's it! Git commands are now accelerated
cd /path/to/large-repo
git status  # Fast!
```

## How It Works

Gity runs a background daemon that:

1. **Watches your files** — Detects changes instantly via OS-native file watchers
2. **Tells Git what changed** — Implements Git's fsmonitor protocol
3. **Keeps objects fresh** — Runs `git maintenance` during idle periods
4. **Caches results** — Remembers status results and serves them instantly

## Commands

| Command | Description |
|---------|-------------|
| `gity register <path>` | Start accelerating a repository |
| `gity unregister <path>` | Stop accelerating and clean up |
| `gity list` | Show registered repos |
| `gity health <path>` | Detailed diagnostics |
| `gity daemon start` | Start daemon in background |
| `gity daemon stop` | Stop daemon gracefully |

## Documentation

See [GitHub](https://github.com/neul-labs/gity) for full documentation.

## License

MIT

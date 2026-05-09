# gity

**Make large Git repositories feel instant — install via pip.**

[![PyPI Version](https://img.shields.io/pypi/v/gity)](https://pypi.org/project/gity/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-blue)](https://github.com/neul-labs/gity)

## What is gity?

Gity is a lightweight, cross-platform daemon that accelerates Git operations on large repositories. It watches your files, maintains warm caches, and runs background maintenance so `git status` stays fast even in repos with millions of files.

This PyPI package is a thin wrapper that downloads and installs the correct pre-built native binary for your platform on first use.

## Installation

```bash
pip install gity
```

Or with uv:

```bash
uv tool install gity
```

## Quick Start

```bash
# Register your large repo (one-time setup)
gity register /path/to/large-repo

# Git commands are now accelerated
cd /path/to/large-repo
git status  # Fast!
```

## Supported Platforms

| Platform | Architecture | Status |
|----------|--------------|--------|
| Linux    | x86_64, aarch64 | Supported |
| macOS    | x86_64, arm64   | Supported |
| Windows  | x86_64          | Supported |

## Commands

| Command | Description |
|---------|-------------|
| `gity register <path>` | Start accelerating a repository |
| `gity unregister <path>` | Stop accelerating and clean up |
| `gity list` | Show registered repos |
| `gity status <path>` | Fast status summary |
| `gity health <path>` | Detailed diagnostics |
| `gity daemon start/stop` | Control the background daemon |

## Requirements

- Python 3.8+
- Git 2.37+
- Linux, macOS, or Windows

## Links

- [Source Code](https://github.com/neul-labs/gity)
- [Releases](https://github.com/neul-labs/gity/releases)
- [Issues](https://github.com/neul-labs/gity/issues)
- [Changelog](https://github.com/neul-labs/gity/releases)

## License

MIT

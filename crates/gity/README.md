# gity

**Make large Git repositories feel instant.**

[![Crates.io](https://img.shields.io/crates/v/gity)](https://crates.io/crates/gity)
[![Documentation](https://img.shields.io/badge/docs-neullabs.com-blue)](http://docs.neullabs.com/gity)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)

Gity is a lightweight, cross-platform daemon that accelerates Git operations on large repositories. A single binary runs on Linux, macOS, and Windows - watching your files, maintaining warm caches, and running background maintenance so `git status` stays fast even in repos with millions of files.

## Quick Start

```bash
# Install
cargo install gity

# Register your large repo
gity register /path/to/large-repo

# Git commands are now accelerated
cd /path/to/large-repo
git status  # Fast!
```

## Features

- **File watching** - Detects changes instantly via OS-native watchers
- **fsmonitor integration** - Tells Git exactly what changed
- **Background maintenance** - Runs `git maintenance` during idle periods
- **Status caching** - Serves results instantly when nothing changed

See the [main README](https://github.com/neul-labs/gity#readme) for full documentation.

## License

MIT

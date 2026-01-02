# Gity

**Make large Git repositories feel instant.**

Gity is a lightweight, cross-platform daemon that accelerates Git operations on large repositories. A single binary runs on **Linux**, **macOS**, and **Windows**—watching your files, maintaining warm caches, and running background maintenance so `git status` stays fast even in repos with millions of files.

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

1. **Watches your files** — Detects changes instantly via OS-native file watchers
2. **Tells Git what changed** — Implements Git's fsmonitor protocol so Git only checks files that actually changed
3. **Keeps objects fresh** — Runs `git maintenance` during idle periods so fetches stay fast
4. **Caches results** — Remembers status results and serves them instantly when nothing changed

The result: `git status` in milliseconds instead of seconds.

## Quick Example

```bash
# Register your large repo (one-time setup)
gity register /path/to/large-repo

# That's it! Git commands are now accelerated
cd /path/to/large-repo
git status  # Fast!
```

## Cross-Platform Support

| Platform | File Watcher | Status |
|----------|--------------|--------|
| **Linux** | inotify | Full support |
| **macOS** | FSEvents | Full support |
| **Windows** | ReadDirectoryChangesW | Full support |
| **WSL2** | inotify (Linux FS only) | [See notes](platform-guides/wsl2.md) |

One binary, same features everywhere. No platform-specific configuration needed.

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
# Caches are shared between related repos
```

### CI/CD Optimization

Your CI builds clone large repos and run status checks. Use oneshot mode:

```bash
gity daemon oneshot /path/to/repo
git status
git diff --cached
```

### IDE Integration

IDEs constantly poll `git status` for file decorations. With Gity, these polls return instantly:

```bash
# IDE calls this repeatedly
git status --porcelain  # Returns in <10ms with gity
```

## Next Steps

- [Installation](getting-started/installation.md) — Get Gity running on your system
- [Quick Start](getting-started/quick-start.md) — Accelerate your first repository
- [How It Works](concepts/how-it-works.md) — Understand the architecture

## Requirements

- Git 2.37+ (for fsmonitor protocol v2)
- Linux, macOS, or Windows

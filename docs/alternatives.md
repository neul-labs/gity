# Alternatives & Trade-offs

This document captures the major options we considered and why the current design landed on an async Rust daemon backed by sled/rykv and async-nng.

## Git Features Only (fsmonitor, untrackedCache)

**Pros**
- Built directly into Git, minimal code to maintain.
- Works everywhere Git runs.

**Cons**
- Requires external fsmonitor implementation anyway; Windows/Linux/macOS parity is inconsistent.
- No facilities for background fetch, prefetch planning, or metadata replication.
- Hard to extend for IDE integrations or multiple working trees.
- Performance impact: clean repos still trigger global scans when caches fall out of sync, and Git’s native watchers lack aggressive coalescing, so CPU usage stays high on busy trees.

**Why we moved on:** These features are necessary but not sufficient. They become inputs to our daemon rather than the entire solution.

## Watchman + Custom Scripts

**Pros**
- Mature watcher with high performance.
- Already used by many monorepos; well documented.

**Cons**
- Requires Python/Lua scripting; limited Rust reuse.
- IPC semantics are JSON/Unix-socket centric; harder to embed in a portable CLI.
- No opinionated story for metadata persistence or replication.
- Performance impact: Watchman excels at event delivery but leaves cache management and throttling to custom scripts, often leading to duplicated scans across related clones.

**Why we moved on:** Watchman is great at watching, but we lacked a cohesive scheduler + metadata layer. Rebuilding everything around Watchman would fragment the project.

## HTTP/gRPC Daemon

**Pros**
- Familiar protocol; easy to inspect traffic.
- Works across machines without extra effort.

**Cons**
- Overkill for local CLI↔daemon messaging; introduces TLS/auth even when not needed.
- Harder to do low-latency PUB/SUB notifications.
- Pulls in heavyweight dependencies instead of tight async channels.
- Performance impact: HTTP stacks add latency and memory overhead for every command; idle daemons still need to keep accept loops alive, which increases baseline CPU use.

**Why we moved on:** Async request/response plus streaming updates map directly onto Nanomsg patterns, and `async-nng` keeps the binary lean. HTTP remains an option if we later add remote agents.

## External Databases (SQLite, Redis)

**Pros**
- Rich query languages, mature tooling.
- Easy multi-process access.

**Cons**
- Requires additional installations or containers.
- Costly for portable field setups (e.g., dev laptops without admin rights).
- Adds more moving parts than we need.
- Performance impact: external databases incur IPC and fsync costs that dwarf sled’s in-process writes, making quick status answers slower and less predictable.

**Why we moved on:** `sled` delivers durability and ordered iteration without leaving the project repo. `rykv` fills the replication gap when we truly need to share data.

## Kernel-level VFS (ProjFS, GVFS-style)

**Pros**
- Transparent virtualization of the working tree; Git only touches relevant blobs.

**Cons**
- Requires admin privileges, kernel extensions, and platform-specific code.
- Hard to distribute to every developer environment, especially containers.
- Debugging is difficult; tight coupling to OS updates.
- Performance impact: while VFS layers can keep disk usage low, they rely on network round-trips whenever materializing data, so latency simply shifts from local disks to remote storage.

**Why we moved on:** We prefer user-space tooling that anyone can run. Sparse checkout + partial clone + background prefetch cover the same pain points with far less risk.

## Current Direction

By pairing async-nng messaging with sled/rykv for state and same-machine replication, we achieve:

- Portable binaries (no external services).
- Event-driven workflows (watcher deltas immediately feed clients).
- Extensibility (new commands/jobs are just message handlers).
- Predictable performance (bounded watchers, resource-aware scheduling, and shared caches between related local clones).

We keep monitoring competing approaches—especially Watchman integrations and Git’s evolving maintenance features—but the present stack balances reliability, performance, and ease of contribution for our goals.

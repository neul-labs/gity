# Gitz

Gitz is a Rust accelerator for very large Git repositories. Everything ships as a **single cross-platform binary** called `gitz`. It exposes familiar CLI subcommands plus a `gitz daemon ...` family that keeps a background service alive. The daemon core watches the working tree, keeps metadata warm in an embedded database, and schedules background Git maintenance. A built-in system-tray integration (Info, Exit) lets you monitor and stop the service without touching a terminal. The goal is simple: make local Git feel instant, without syncing anything to other machines, no matter whether you run Windows, macOS, or Linux.

The project leans on pragmatic crates such as `async-nng` for low-latency IPC, `sled` for embeddable persistence, and `rykv` for replicating caches between multiple registrations of the same repository on a single workstation.

## Highlights

- **Instant status** – multi-platform filesystem watchers and cached ignore results keep everyday commands from re-scanning the world.
- **Fresh objects** – a prefetch planner maintains bare object stores and runs `git fetch`/`git maintenance run` outside the critical path.
- **Composable IPC** – all user-facing commands talk to the daemon over `async-nng`, enabling multiple CLIs or IDE integrations without extra glue.
- **Resilient metadata** – data persisted through `sled` survives crashes, while `rykv` replicates critical keys between related clones on the same workstation.
- **Extensible workflows** – the docs describe how we work, how to extend the daemon, and how to trade off alternative designs.
- **Cross-platform delivery** – the same binary, tray UI, and installers span Windows, macOS, and Linux thanks to native watcher backends.

## Quick Start

1. Install Rust stable (1.75+ recommended).
2. Install `cmake`/`pkg-config` if your platform needs them for `async-nng`.
3. Install the binary (places it on your `$PATH`):

   ```bash
   cargo install --path .
   ```

4. Start the daemon manually (optional—CLI and tray commands auto-start it if missing):

   ```bash
   gitz daemon run &
   ```

5. (Optional) Launch the tray UI so you can inspect status or exit with two clicks:

   ```bash
   gitz tray
   ```

6. Register the repository you want accelerated (runs once per repo):

   ```bash
   gitz register /path/to/repo
   ```

7. Query the daemon with the CLI:

   ```bash
   gitz list              # view registered repos
   gitz status <repo>     # fast status summary
   gitz logs <repo>       # follow daemon logs for that repo
   ```

If the daemon is not running, `gitz` will auto-start `gitz daemon start` (equivalent to `gitz daemon run` in the background) the first time you issue a command or launch the tray icon.

## Repository Layout

| Path           | Purpose                                  |
| -------------- | ---------------------------------------- |
| `README.md`    | This overview and quick-start guide.     |
| `docs/`        | Detailed design docs, workflows, and ADRs. |
| `src/bin/`     | `gitz` entry point (CLI + daemon subcommands).   |
| `src/lib/`     | Shared crates: watchers, scheduler, IPC. |

## Repository Registration & Lifecycle

- `gitz register` stores a repo entry in `sled` keyed by its `.git` directory and begins watching it immediately.
- Registration is local-only; no metadata is pushed to other machines or services.
- If the runtime has been offline, the next `gitz` invocation loads the cached snapshot, performs a targeted reconciliation scan, and resumes watching. Long gaps trigger a full walk so the cache can be trusted again.
- `rykv` replicates hot metadata between multiple registrations on the **same** workstation (e.g., worktree pairs or repo clones in different directories) so cold starts stay fast without any network sync.
- `gitz list` displays every registered repository, its health, and the last event time so you can prune stale entries with `gitz unregister`.

See `docs/architecture.md` for more detail about how watchers rehydrate after downtime.

## Background Work Triggers

Background Git commands run only when they help local developers:

- **File events** – watcher deltas enqueue verification jobs so `git status` stays incremental.
- **Idle timers** – a low-priority loop triggers `git fetch --filter=blob:none` and `git maintenance run` after configurable idle periods.
- **Manual nudges** – commands like `gitz prefetch now` push urgent jobs to the front of the queue.

When a repository hasn’t been touched for a while, the runtime suspends its timers. Once a new file event or CLI command appears, the scheduler resumes, performs a freshness check, then issues deferred fetch/maintenance jobs.

## Branch Awareness

The watcher monitors both the working tree and `.git/refs`. When a branch switches:

- Gitz records the new HEAD in `sled`.
- Cached metadata invalidated by the branch change is marked “dirty” so the next status call verifies it.
- Background fetch gets nudged to ensure the new upstream tracking branch is current.

The filesystem integration does not enforce branch policies; it only keeps metadata aligned with whichever branch the developer selects locally.

## Resource Awareness

The daemon core (reachable via `gitz daemon ...`) samples its own CPU, memory, open file handles, and per-repo cache sizes. The metrics are:

- Exposed via `gitz list --stats` for a quick overview.
- Logged inside the per-repo log streams so `gitz logs <repo>` shows spikes and throttling decisions.
- Used internally to pause low-priority background work if resource budgets are exceeded (e.g., when the daemon is already consuming configured CPU/memory limits).

## Performance Considerations

We aim to keep the daemon invisible when a developer is doing other work. Key safeguards:

- **Back-pressure** – the resource monitor halts prefetch/maintenance jobs when CPU/RSS/FD thresholds are exceeded, preventing runaway processes on busy laptops.
- **Bounded watchers** – each repo uses configurable event coalescing and rate limits to avoid flooding the scheduler on frequently changing directories.
- **Cache hygiene** – sled buckets store only metadata needed for fast status checks; older entries age out automatically so disk usage stays predictable.
- **Cold-start fairness** – when multiple related clones exist on the same workstation, `rykv` shares metadata snapshots so only the first registration performs a full scan.
- **Tray visibility** – the Info view surfaces queue depth, last run duration, and throttling flags so developers can investigate if commands slow down.

## System Tray

Running `gitz tray` launches a cross-platform tray icon (Windows, macOS, Linux). The menu exposes:

- **Info** – opens a small window summarizing registered repos, queue depths, and resource usage.
- **Exit** – gracefully shuts down the daemon, watchers, and timers.

The tray client uses the same async-nng IPC as the CLI and requires no additional binaries.

## Distribution

In addition to Cargo (and ecosystem package managers like npm, pip, Homebrew, Chocolatey), we ship platform-native installers so developers can set up the single binary without a compiler:

- Windows: MSI installer that installs `gitz.exe`, registers it in PATH, and optionally auto-starts `gitz daemon run`.
- macOS: notarized `.pkg` that places `gitz` in `/usr/local/bin` and can register a LaunchAgent for the daemon/tray.
- Linux: AppImage and `.deb` packages that install `gitz`, a systemd user service for the daemon, and a tray desktop entry.

Installer scripts simply wrap the same binary; upgrades replace it in-place and restart the daemon if it was running.

## Documentation Map

- `docs/architecture.md` – deep dive into components, data flow, and technology choices.
- `docs/alternatives.md` – trade-offs against other Git acceleration approaches.
- `docs/process.md` – how we work: conventions, testing, release cadence.
- `docs/commands.md` – CLI, daemon subcommands, and tray reference.

Start with the architecture doc if you need to understand how the parts fit together, then jump to the alternatives doc when you need context for design decisions.

## Contributing

1. Read `docs/process.md` to align on conventions.
2. Open a draft PR early so reviewers can help shape the direction.
3. Update relevant docs whenever you change behavior.

Please include notes on how you tested your changes and any regressions you considered. The CI matrix (to be added) will run the same checks documented in our process guide.

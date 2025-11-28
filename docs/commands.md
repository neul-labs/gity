# Command Reference

All functionality lives in the single `gitz` binary. Commands fall into three categories: daemon control, CLI workflows, and tray UX.

## Daemon Subcommands (`gitz daemon ...`)

| Command | Description |
| ------- | ----------- |
| `gitz daemon run [--config PATH]` | Starts the background service in the foreground (press `Ctrl+C` to exit). |
| `gitz daemon start` | Starts the daemon as a detached process; used by installers and `gitz tray`. |
| `gitz daemon stop` | Signals the running daemon to shut down gracefully (same effect as the tray “Exit” action). |
| `gitz daemon oneshot <repo>` | Boots the daemon, services queued jobs for `<repo>`, then exits—handy for CI. |
| `gitz daemon metrics` | Prints the latest CPU/RSS/fd/cache metrics and job queue depths, then exits. |

The daemon automatically tracks CPU, memory, file descriptors, and sled storage size per registered repository. When limits defined in the config are exceeded, low-priority jobs (prefetch, maintenance) are throttled until resources fall below the thresholds.

## CLI Commands

| Command | Description |
| ------- | ----------- |
| `gitz register <repo>` | Adds a repository to the registry, kicks off an initial scan, and starts watching it. |
| `gitz unregister <repo>` | Removes the repo from the registry, stops watchers, and drops cached metadata. |
| `gitz list [--stats]` | Shows every registered repository, health indicators, and optional resource stats when `--stats` is specified. |
| `gitz status <repo>` | Returns a fast status summary (clean/dirty, changed paths) using cached metadata plus targeted Git verification. |
| `gitz changed <repo> [--since <token>]` | Lists files changed since the provided watcher token (defaults to last status token). |
| `gitz prefetch <repo> [now]` | Enqueues background fetch/maintenance jobs. With `now`, the job jumps to the front of the queue. |
| `gitz maintain <repo>` | Forces maintenance tasks (commit-graph refresh, GC) regardless of idle timers. |
| `gitz logs <repo> [--follow]` | Streams structured daemon logs for the repo. `--follow` tails live output; without it, the command prints the most recent entries. Each log line includes resource metrics. |
| `gitz health <repo>` | Runs diagnostic checks (sled integrity, watcher tokens, scheduler backlog, resource throttling) and prints a human-friendly report. |

## System Tray

| Command | Description |
| ------- | ----------- |
| `gitz tray` | Launches the cross-platform tray icon. The tray menu contains: |
|             | **Info** – opens a window summarizing registered repos, watcher health, queue depths, and resource usage. |
|             | **Exit** – equivalent to `gitz daemon stop`; shuts down the daemon and removes the tray icon. |

The tray client attaches to the daemon through async-nng just like the CLI. If the daemon is not running, `gitz tray` will start it before showing the icon.

### Command Behavior Notes

- CLI and tray commands implicitly start `gitz daemon start` if the daemon is not already running.
- `gitz list --stats` displays daemon resource usage per repo: CPU %, RSS, cache size, and number of queued jobs.
- `gitz logs` reads from the daemon’s structured log ring stored in `sled`, so historical context survives restarts.
- `gitz health` is the safest way to understand what happens after a long downtime: it surfaces whether a reconciliation scan is scheduled, when the next background job will run, and whether resource throttling is active.

Refer to `docs/architecture.md` for deeper context on how each command interacts with the watcher, scheduler, metadata layers, and system tray.

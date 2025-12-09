# Command Reference

All functionality lives in the single `gity` binary. Commands fall into three categories: daemon control, CLI workflows, and tray UX.

## Daemon Subcommands (`gity daemon ...`)

| Command | Description |
| ------- | ----------- |
| `gity daemon run [--config PATH]` | Starts the background service in the foreground (press `Ctrl+C` to exit). |
| `gity daemon start` | Starts the daemon as a detached process; used by installers and `gity tray`. |
| `gity daemon stop` | Signals the running daemon to shut down gracefully (same effect as the tray “Exit” action). |
| `gity daemon oneshot <repo>` | Boots the daemon, services queued jobs for `<repo>`, then exits—handy for CI. |
| `gity daemon metrics` | Prints the latest CPU/RSS/fd/cache metrics and job queue depths, then exits. |

The daemon automatically tracks CPU, memory, file descriptors, and sled storage size per registered repository. When limits defined in the config are exceeded, low-priority jobs (prefetch, maintenance) are throttled until resources fall below the thresholds.

## CLI Commands

| Command | Description |
| ------- | ----------- |
| `gity register <repo>` | Adds a repository to the registry, kicks off an initial scan, and starts watching it. |
| `gity unregister <repo>` | Removes the repo from the registry, stops watchers, and drops cached metadata. |
| `gity list [--stats]` | Shows every registered repository, health indicators, and optional resource stats when `--stats` is specified (CPU/RSS/pending jobs + per-repo queue depth). |
| `gity status <repo>` | Returns a fast status summary (clean/dirty, changed paths) using cached metadata plus targeted Git verification. |
| `gity fsmonitor-helper [version token]` | Implementation backing `core.fsmonitor`; Git invokes this command with the protocol version + token, and it emits NUL-separated dirty paths from the daemon. |
| `gity events` | Subscribes to the daemon’s PUB socket and streams watcher notifications until interrupted. |
| `gity changed <repo> [--since <token>]` | Lists files changed since the provided watcher token (defaults to last status token). |
| `gity prefetch <repo> [now]` | Enqueues background fetch/maintenance jobs. With `now`, the job jumps to the front of the queue. |
| `gity maintain <repo>` | Forces maintenance tasks (commit-graph refresh, GC) regardless of idle timers. |
| `gity logs <repo> [--follow] [--limit N]` | Streams structured daemon logs for the repo. `--follow` tails live output; without it, the command prints the most recent entries (default `N=50`). Each log line includes resource metrics. |
| `gity health <repo>` | Runs diagnostic checks (sled integrity, watcher tokens, scheduler backlog, resource throttling) and prints a human-friendly report. Includes the current daemon-side generation token so cached status consumers can verify expectations. |

## System Tray

| Command | Description |
| ------- | ----------- |
| `gity tray` | Launches the cross-platform tray icon. The tray menu contains: |
|             | **Info** – opens a window summarizing registered repos, watcher health, queue depths, and resource usage. |
|             | **Exit** – equivalent to `gity daemon stop`; shuts down the daemon and removes the tray icon. |

The tray client attaches to the daemon through async-nng just like the CLI. If the daemon is not running, `gity tray` will start it before showing the icon.

### Command Behavior Notes

- CLI and tray commands implicitly start `gity daemon start` if the daemon is not already running.
- `gity list --stats` displays daemon resource usage per repo: CPU %, RSS, cache size, and number of queued jobs.
- `gity logs` reads from the daemon’s structured log ring stored in `sled`, so historical context survives restarts.
- `gity health` is the safest way to understand what happens after a long downtime: it surfaces whether a reconciliation scan is scheduled, when the next background job will run, and whether resource throttling is active.

Refer to `docs/architecture.md` for deeper context on how each command interacts with the watcher, scheduler, metadata layers, and system tray.

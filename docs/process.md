# Process & Working Agreements

This project optimizes for predictable iteration on a complex Rust daemon. The guidelines below describe how we collaborate, test, and release.

## Principles

1. **Document first** – any significant change starts with an issue or doc PR so reviewers understand the user impact.
2. **Automate repetitive work** – if a command must run for every change, encode it in `cargo xtask` once.
3. **Keep the daemon observable** – logs and metrics should make it obvious what the watcher, scheduler, and IPC layers are doing.

## Branches & Reviews

- Use feature branches named `feature/<topic>` or `fix/<topic>`.
- Draft PRs are welcome; they should link to any design doc or section within `docs/architecture.md`.
- Every PR requires at least one reviewer familiar with the part of the system being touched (watching, scheduler, IPC, storage).

## Testing Expectations

- **Unit tests** live near the code (`#[cfg(test)]`). Focus on deterministic logic: queue ordering, metadata transforms, IPC serialization.
- **Integration tests** run via `cargo test --all -- --nocapture` and should cover:
  - async-nng command routing (daemon <-> CLI handshake).
  - sled-backed metadata persistence.
  - rykv replication guards (simulated multi-worktree scenarios).
  - Resource monitor accounting (fake load to verify throttling).
- For filesystem-heavy flows, use `tempfile` + fake watchers to avoid flakiness.

Before opening a PR, run:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --all
```

CI will mirror those commands plus platform smoke tests (Linux, macOS, Windows).

## Documentation Discipline

- Update `README.md` when user-facing behavior changes.
- Keep architecture diagrams in `docs/architecture.md` synchronized with code.
- Log decisions in `docs/alternatives.md` if we reject an option after evaluation.
- Document CLI/daemon changes in `docs/commands.md`.

## Releases

1. Merge the latest `main` into a `release/<version>` branch.
2. Update the changelog and bump crate versions.
3. Tag the release and attach prebuilt binaries when available.
4. Announce in the internal channel with upgrade notes (breaking changes, migration steps).

## Support & Operations

- The daemon ships with a `gity health` command that checks sled integrity, async-nng socket status, watcher subscriptions, and resource budgets.
- For production-like deployments (CI workers, shared dev VMs), enable `rykv` replication so cache warmups happen once.
- File issues with repro steps, logs (`RUST_LOG=info gity daemon run`), and platform details.
- Keep the repo registry clean: `gity register` must be run once per repo path, and `gity unregister` should follow when a working copy is deleted to avoid stale watchers.

Following these rules ensures contributions land smoothly and the sidecar remains trustworthy across giant repositories.

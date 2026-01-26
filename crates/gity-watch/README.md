# gity-watch

Cross-platform file system watcher for [gity](https://github.com/neul-labs/gity).

[![Crates.io](https://img.shields.io/crates/v/gity-watch)](https://crates.io/crates/gity-watch)
[![Documentation](https://docs.rs/gity-watch/badge.svg)](https://docs.rs/gity-watch)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)

This crate provides cross-platform file system watching built on [notify](https://crates.io/crates/notify). It detects file changes in Git repositories and reports them to the daemon.

## Platform Support

| Platform | Backend |
|----------|---------|
| Linux | inotify |
| macOS | FSEvents |
| Windows | ReadDirectoryChangesW |

## Features

- Recursive directory watching
- Debouncing and batching of events
- Async event delivery via tokio channels
- Automatic handling of .gitignore patterns

## Usage

This crate is primarily intended for internal use by `gity-daemon`. See the [gity documentation](http://docs.neullabs.com/gity) for general usage.

## License

MIT

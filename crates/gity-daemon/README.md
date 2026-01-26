# gity-daemon

Background daemon for [gity](https://github.com/neul-labs/gity) - handles file watching, caching, and Git maintenance.

[![Crates.io](https://img.shields.io/crates/v/gity-daemon)](https://crates.io/crates/gity-daemon)
[![Documentation](https://docs.rs/gity-daemon/badge.svg)](https://docs.rs/gity-daemon)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)

This crate implements the core daemon that accelerates Git operations. It manages file watchers, maintains status caches, and schedules background maintenance tasks.

## Features

- IPC server for CLI and fsmonitor communication
- File system watcher management per repository
- Status result caching with invalidation
- Background `git maintenance` scheduling
- Repository health monitoring

## Architecture

The daemon uses:
- [async-nng](https://crates.io/crates/async-nng) for IPC transport
- [sled](https://crates.io/crates/sled) for persistent metadata storage
- [tokio](https://crates.io/crates/tokio) for async runtime

## Usage

This crate is primarily intended for internal use by the `gity` binary. See the [gity documentation](http://docs.neullabs.com/gity) for general usage.

## License

MIT

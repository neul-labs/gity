# gity-cli

CLI interface for [gity](https://github.com/neul-labs/gity) - Git repository acceleration daemon.

[![Crates.io](https://img.shields.io/crates/v/gity-cli)](https://crates.io/crates/gity-cli)
[![Documentation](https://docs.rs/gity-cli/badge.svg)](https://docs.rs/gity-cli)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)

This crate provides the command-line argument parsing and command handlers for gity. It uses [clap](https://crates.io/crates/clap) for argument parsing.

## Commands

| Command | Description |
|---------|-------------|
| `register <path>` | Start accelerating a repository |
| `unregister <path>` | Stop accelerating and clean up |
| `list` | Show registered repos |
| `status <path>` | Fast status summary |
| `health <path>` | Detailed diagnostics |
| `daemon start/stop` | Control the daemon |

## Usage

This crate is primarily intended for internal use by the `gity` binary. See the [gity documentation](http://docs.neullabs.com/gity) for general usage.

## License

MIT

# gity-cli

**Command-line interface and argument parsing for the gity Git accelerator.**

[![Crates.io](https://img.shields.io/crates/v/gity-cli)](https://crates.io/crates/gity-cli)
[![Documentation](https://docs.rs/gity-cli/badge.svg)](https://docs.rs/gity-cli)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

## What is gity-cli?

`gity-cli` provides the command-line argument parsing, command dispatch, and user-facing error messages for [gity](https://crates.io/crates/gity). Built on [clap](https://crates.io/crates/clap) with derive macros, it exposes the full `gity` command tree — from `register` and `daemon start` to `health` and `status` — in a structured, testable crate.

## Features

- **Clap-derived CLI** — Type-safe argument parsing with auto-generated `--help`
- **Command dispatch** — Clean separation of command definitions from execution logic
- **Error context** — `thiserror`-based errors with actionable user messages
- **Extensible** — Easy to add new subcommands and flags
- **Cross-platform** — Linux, macOS, Windows

## Commands

| Command | Description |
|---------|-------------|
| `register <path>` | Start accelerating a repository |
| `unregister <path>` | Stop accelerating and clean up |
| `list` | Show registered repos |
| `status <path>` | Fast status summary |
| `health <path>` | Detailed diagnostics |
| `daemon start/stop` | Control the background daemon |

## Usage

This crate is primarily intended for internal use by the `gity` binary. End users should install `gity` directly:

```bash
cargo install gity
```

If you are building a custom wrapper:

```rust
use gity_cli::{Cli, Commands};
use clap::Parser;

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::Register { path } => println!("Registering {}", path.display()),
        _ => {}
    }
}
```

## Installation

```toml
[dependencies]
gity-cli = "0.1.2"
```

## Platform Support

| Platform | Status |
|----------|--------|
| Linux    | Full support |
| macOS    | Full support |
| Windows  | Full support |

## See Also

- [gity](https://crates.io/crates/gity) — Main binary that bundles this CLI
- [gity-daemon](https://crates.io/crates/gity-daemon) — Background daemon controlled by these commands
- [gity-ipc](https://crates.io/crates/gity-ipc) — IPC protocol used to send commands to the daemon

## License

MIT

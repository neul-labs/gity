# gity-ipc

**Lightweight IPC protocol and message types for the gity Git acceleration daemon.**

[![Crates.io](https://img.shields.io/crates/v/gity-ipc)](https://crates.io/crates/gity-ipc)
[![Documentation](https://docs.rs/gity-ipc/badge.svg)](https://docs.rs/gity-ipc)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

## What is gity-ipc?

`gity-ipc` defines the request/response protocol used by the [gity](https://crates.io/crates/gity) daemon ecosystem. It provides strongly-typed, binary-serialized messages for communication between the CLI, background daemon, system tray, and Git fsmonitor hook. Built for speed and reliability in developer-tool workflows.

## Features

- **Strongly-typed messages** — Structured requests and responses for every daemon operation
- **Binary serialization** — Compact `bincode` encoding for near-zero-overhead IPC
- **Async transport traits** — Pluggable backends (NNG, TCP, Unix sockets)
- **Zero-copy friendly** — Message design minimizes allocations on hot paths
- **Cross-platform** — Runs on Linux, macOS, and Windows

## Usage

```rust
use gity_ipc::{Request, Response, IpcClient};

async fn check_status(client: &mut IpcClient) -> anyhow::Result<()> {
    let resp = client.send(Request::Status { repo: "/my/repo".into() }).await?;
    println!("{:?}", resp);
    Ok(())
}
```

## Installation

```toml
[dependencies]
gity-ipc = "0.1.2"
```

## Platform Support

| Platform | Status |
|----------|--------|
| Linux    | Full support |
| macOS    | Full support |
| Windows  | Full support |

## See Also

- [gity](https://crates.io/crates/gity) — Main binary and user-facing CLI
- [gity-daemon](https://crates.io/crates/gity-daemon) — Background daemon that uses this protocol
- [gity-cli](https://crates.io/crates/gity-cli) — CLI argument parsing and command handlers

## License

MIT

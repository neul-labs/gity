# gity-tray

**Cross-platform system tray UI for the gity Git accelerator.**

[![Crates.io](https://img.shields.io/crates/v/gity-tray)](https://crates.io/crates/gity-tray)
[![Documentation](https://docs.rs/gity-tray/badge.svg)](https://docs.rs/gity-tray)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

## What is gity-tray?

`gity-tray` adds a native system tray icon and menu to [gity](https://crates.io/crates/gity). It lets users monitor daemon status, view registered repositories, and start or stop the daemon without opening a terminal. A small quality-of-life addition for developers who prefer GUI controls over shell commands.

## Platform Support

| Platform | Toolkit | Status |
|----------|---------|--------|
| Linux    | GTK     | Full support |
| macOS    | winit   | Full support |
| Windows  | winit   | Full support |

## Features

- **Daemon status indicator** — Green/orange dot showing whether gity is running
- **Quick repo list** — See registered repositories at a glance
- **Start/stop controls** — Launch or quit the daemon from the menu
- **Health overview** — One-click health check for any registered repo
- **Native look** — Uses platform-native tray APIs (not a web view)

## Usage

Enable the tray feature when building gity:

```bash
cargo install gity --features tray
gity tray
```

If you are embedding the tray in your own application:

```rust
use gity_tray::TrayApp;

fn main() -> anyhow::Result<()> {
    TrayApp::new()?.run()?;
    Ok(())
}
```

## Installation

```toml
[dependencies]
gity-tray = "0.1.2"
```

## See Also

- [gity](https://crates.io/crates/gity) — Main binary; build with `--features tray` to enable
- [gity-daemon](https://crates.io/crates/gity-daemon) — Background daemon monitored by this tray UI
- [tray-icon](https://crates.io/crates/tray-icon) — Underlying cross-platform tray library

## License

MIT

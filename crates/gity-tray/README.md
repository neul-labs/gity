# gity-tray

Cross-platform system tray UI for [gity](https://github.com/neul-labs/gity).

[![Crates.io](https://img.shields.io/crates/v/gity-tray)](https://crates.io/crates/gity-tray)
[![Documentation](https://docs.rs/gity-tray/badge.svg)](https://docs.rs/gity-tray)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)

This crate provides a system tray interface for gity, allowing users to monitor and control the daemon from the menu bar.

## Platform Support

| Platform | Toolkit |
|----------|---------|
| Linux | GTK |
| macOS | winit |
| Windows | winit |

## Features

- Daemon status indicator
- Quick access to registered repositories
- Start/stop daemon controls
- Repository health overview

## Usage

Enable the tray feature when building gity:

```bash
cargo install gity --features tray
```

Then launch with:

```bash
gity tray
```

See the [gity documentation](http://docs.neullabs.com/gity) for more details.

## License

MIT

# gity-ipc

IPC protocol and message types for [gity](https://github.com/neul-labs/gity) daemon communication.

[![Crates.io](https://img.shields.io/crates/v/gity-ipc)](https://crates.io/crates/gity-ipc)
[![Documentation](https://docs.rs/gity-ipc/badge.svg)](https://docs.rs/gity-ipc)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)

This crate defines the protocol used for communication between gity components (CLI, daemon, tray). It uses bincode for efficient binary serialization.

## Features

- Request/response message types
- Binary serialization with bincode
- Async transport traits
- Error types for IPC failures

## Usage

This crate is primarily intended for internal use by gity components. See the [gity documentation](http://docs.neullabs.com/gity) for general usage.

## License

MIT

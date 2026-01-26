# gity-storage

Persistent storage layer for [gity](https://github.com/neul-labs/gity) using sled.

[![Crates.io](https://img.shields.io/crates/v/gity-storage)](https://crates.io/crates/gity-storage)
[![Documentation](https://docs.rs/gity-storage/badge.svg)](https://docs.rs/gity-storage)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)

This crate provides persistent key-value storage for gity metadata using [sled](https://crates.io/crates/sled), an embedded database.

## Features

- Repository registration storage
- Status cache persistence
- Maintenance history tracking
- Atomic operations with sled transactions

## Usage

This crate is primarily intended for internal use by `gity-daemon`. See the [gity documentation](http://docs.neullabs.com/gity) for general usage.

## License

MIT

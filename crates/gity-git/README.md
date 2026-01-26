# gity-git

Git operations library for [gity](https://github.com/neul-labs/gity).

[![Crates.io](https://img.shields.io/crates/v/gity-git)](https://crates.io/crates/gity-git)
[![Documentation](https://docs.rs/gity-git/badge.svg)](https://docs.rs/gity-git)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)

This crate provides Git operations built on [git2](https://crates.io/crates/git2) (libgit2 bindings) for the gity daemon. It handles repository introspection, status queries, and maintenance operations.

## Features

- Repository discovery and validation
- Status and diff operations
- Maintenance task execution
- Branch and remote management

## Usage

This crate is primarily intended for internal use by `gity-daemon`. See the [gity documentation](http://docs.neullabs.com/gity) for general usage.

## License

MIT

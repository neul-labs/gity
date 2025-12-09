# Contributing to Gity

Thank you for your interest in contributing to Gity! This document provides guidelines and information for contributors.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/gity.git`
3. Create a feature branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run tests and checks (see below)
6. Commit your changes
7. Push to your fork and submit a pull request

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Git 2.37 or later

### Building

```bash
cargo build --all
```

### Running Tests

```bash
cargo test --all
```

## Before Submitting a PR

Please run the following commands and ensure they pass:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --all
```

## Pull Request Guidelines

### Branch Naming

- Feature branches: `feature/<topic>`
- Bug fixes: `fix/<topic>`

### PR Process

1. Draft PRs are welcome for early feedback
2. Link to any relevant design docs or architecture sections
3. Every PR requires at least one reviewer familiar with the affected system component (watching, scheduler, IPC, storage)

### What to Include

- Clear description of what the PR does
- Link to any related issues
- Test coverage for new functionality
- Documentation updates if user-facing behavior changes

## Testing Expectations

- **Unit tests**: Focus on deterministic logic - queue ordering, metadata transforms, IPC serialization
- **Integration tests**: Cover async-nng command routing, sled persistence, resource monitor accounting
- Use `tempfile` + fake watchers for filesystem-heavy tests to avoid flakiness

## Documentation

When making changes, please update relevant documentation:

- `README.md` - User-facing behavior changes
- `docs/architecture.md` - System design changes
- `docs/commands.md` - CLI/daemon changes
- `docs/alternatives.md` - Rejected options after evaluation

## Reporting Issues

When filing issues, please include:

- Steps to reproduce
- Logs (`RUST_LOG=info gity daemon run`)
- Platform details (OS, Git version, Rust version)

## License

By contributing to Gity, you agree that your contributions will be licensed under the MIT License.

## Questions?

Feel free to open an issue for any questions about contributing.

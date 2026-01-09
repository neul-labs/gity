# Performance Demo Tool

The `gity-demo` tool showcases gity's performance benefits by creating large test repositories and comparing `git status` execution times with and without gity.

## Overview

The demo creates two identical git repositories:
- `/tmp/gity-demo-with` - Registered with gity (accelerated)
- `/tmp/gity-demo-without` - Baseline (standard git)

Both repositories are populated with the same nested directory structure containing up to 1 million files. A background task continuously modifies files while the benchmark runs, simulating real developer activity.

## Building

```bash
cd demo
cargo build --release
```

The binary will be at `demo/target/release/gity-demo`.

## Usage

```
gity-demo [OPTIONS]

Options:
  --files <N>         Number of files to create (default: 1000000)
  --mod-rate <N>      File modifications per second (default: 5)
  --iterations <N>    Number of benchmark iterations (default: 10)
  --skip-setup        Skip repository creation (use existing repos)
  --cleanup           Remove test repositories on exit
  --repo-path <PATH>  Base path for test repositories (default: /tmp)
  --gity-bin <PATH>   Path to gity binary (defaults to PATH)
  -h, --help          Print help
```

## Examples

### Full demo (1 million files)

```bash
# First run - creates repos and runs benchmark
./target/release/gity-demo

# Subsequent runs - reuse existing repos
./target/release/gity-demo --skip-setup
```

### Quick test (10,000 files)

```bash
./target/release/gity-demo --files 10000 --iterations 5
```

### Custom configuration

```bash
./target/release/gity-demo \
  --files 100000 \
  --mod-rate 10 \
  --iterations 20 \
  --repo-path /var/tmp
```

### Cleanup after demo

```bash
./target/release/gity-demo --skip-setup --cleanup
```

## Display

The demo displays a real-time side-by-side comparison:

```
                         GITY PERFORMANCE DEMO
============================================================================
              WITH GITY               |            WITHOUT GITY
----------------------------------------------------------------------------
            Last status: 12.34ms      |      Last status: 4823.45ms
            Avg status:  15.67ms      |      Avg status:  5102.89ms
----------------------------------------------------------------------------
                   Speedup: 325.6x faster with gity
        Files: 1000000 | Modifications: 23 | Iteration: 5/10
============================================================================

                      Press 'q' to stop benchmark
```

## How It Works

1. **Repository Setup**: Creates nested directory structure (e.g., 1000 dirs x 100 subdirs x 10 files = 1M files)
2. **Git Initialization**: Initializes both repos and creates initial commit with all files
3. **Gity Registration**: Registers one repo with the gity daemon
4. **File Modification**: Background task randomly modifies files at configurable rate
5. **Benchmarking**: Alternates running `git status --porcelain` on both repos
6. **Live Display**: Shows timing comparison with crossterm terminal UI

## File Structure

Generated repositories have this structure:

```
gity-demo-with/
├── .git/
├── module0000/
│   ├── sub000/
│   │   ├── file00.txt
│   │   ├── file01.txt
│   │   └── ...
│   ├── sub001/
│   └── ...
├── module0001/
└── ...
```

Each file contains a unique identifier and timestamp when modified.

## Performance Notes

- **Setup time**: Creating 1M files takes 5-15 minutes depending on disk speed
- **Expected speedup**: 100x-500x faster with gity on large repos
- **inotify limits**: On Linux, you may need to increase `fs.inotify.max_user_watches`:
  ```bash
  echo 'fs.inotify.max_user_watches=1048576' | sudo tee -a /etc/sysctl.conf
  sudo sysctl -p
  ```

## Dependencies

The demo is a standalone Rust project with these dependencies:
- `clap` - CLI argument parsing
- `tokio` - Async runtime
- `crossterm` - Terminal UI
- `indicatif` - Progress bars
- `rand` - Random file selection
- `git2` - Git repository operations
- `anyhow` - Error handling

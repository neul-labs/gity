# Performance Demo

See gity's performance benefits firsthand with our interactive demo tool. This tool creates large test repositories and shows a real-time side-by-side comparison of `git status` performance with and without gity.

## Quick Start

```bash
# Build the demo tool
cd demo
cargo build --release

# Run with default settings (1 million files)
./target/release/gity-demo
```

!!! note "Setup Time"
    Creating 1 million files takes 5-15 minutes. For a quick test, use `--files 10000`.

## What the Demo Does

1. **Creates two identical repositories** in `/tmp`:
   - `gity-demo-with` - Accelerated by gity
   - `gity-demo-without` - Standard git (baseline)

2. **Populates with nested directories** containing thousands of files

3. **Continuously modifies files** to simulate developer activity

4. **Displays real-time comparison** of `git status` execution times

## Example Output

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

## Command Options

| Option | Default | Description |
|--------|---------|-------------|
| `--files <N>` | 1000000 | Number of files to create |
| `--mod-rate <N>` | 5 | File modifications per second |
| `--iterations <N>` | 10 | Number of benchmark iterations |
| `--skip-setup` | false | Reuse existing test repos |
| `--cleanup` | false | Remove repos on exit |
| `--repo-path <PATH>` | /tmp | Base path for test repos |
| `--gity-bin <PATH>` | gity | Path to gity binary |

## Examples

### Quick Test (Recommended for First Run)

```bash
./target/release/gity-demo --files 10000 --iterations 5
```

### Full Benchmark

```bash
./target/release/gity-demo --files 1000000 --iterations 20
```

### Reuse Existing Repositories

```bash
# First run creates repos
./target/release/gity-demo --files 100000

# Subsequent runs skip setup
./target/release/gity-demo --skip-setup --iterations 10
```

### Clean Up When Done

```bash
./target/release/gity-demo --skip-setup --cleanup
```

## Expected Results

On a typical development machine with 1 million files:

| Metric | With Gity | Without Gity |
|--------|-----------|--------------|
| Average `git status` | 10-50ms | 3-10 seconds |
| Speedup | **100-500x faster** | baseline |

!!! tip "Why Such a Big Difference?"
    Without gity, Git must scan every file's metadata to detect changes. With gity's fsmonitor integration, Git only checks the files that actually changed, reducing a full repository scan to a targeted lookup.

## Troubleshooting

### "Too many open files" or inotify errors

On Linux, increase the inotify watch limit:

```bash
echo 'fs.inotify.max_user_watches=1048576' | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

### Gity daemon not starting

Ensure gity is installed and in your PATH:

```bash
which gity
gity daemon start
```

### Repository setup is slow

File creation speed depends on disk I/O. Using an SSD significantly improves setup time. You can also reduce file count with `--files 100000`.

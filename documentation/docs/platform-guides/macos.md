# macOS

Platform-specific guidance for running Gity on macOS.

## File Watching

Gity uses **FSEvents** for file watching on macOS. This is Apple's native file system event API.

## FSEvents Characteristics

### Latency

FSEvents has inherent latency, typically 300ms to 1 second. This means:

- File changes may not be immediately visible to Gity
- Running `git status` immediately after saving might use stale cache
- The next query will be correct

This is an OS limitation and is usually acceptable for interactive use.

### Coalescing

FSEvents coalesces rapid changes, which is efficient for build systems that modify many files quickly.

## Installation

### From Source

```bash
cargo install --path crates/gity
```

### Homebrew (Coming Soon)

```bash
brew install gity
```

### Package Installer

Download the `.pkg` from [releases](https://github.com/yourusername/gity/releases) and double-click to install.

## System Service

Run Gity as a launchd service for automatic startup.

### Create Launch Agent

Create `~/Library/LaunchAgents/com.gity.daemon.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.gity.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/gity</string>
        <string>daemon</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/gity.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/gity.stderr.log</string>
</dict>
</plist>
```

### Load the Agent

```bash
launchctl load ~/Library/LaunchAgents/com.gity.daemon.plist
```

### Unload the Agent

```bash
launchctl unload ~/Library/LaunchAgents/com.gity.daemon.plist
```

### Check Status

```bash
launchctl list | grep gity
```

## System Tray

The tray icon appears in the menu bar. Click it to access:

- Repository status
- Health information
- Exit option

If the icon doesn't appear, check System Preferences > Notifications.

## Data Location

Default: `~/.gity`

```
~/.gity/
├── data/
│   └── sled/
├── logs/
│   └── daemon.log
└── config/
```

Override with:

```bash
export GITY_HOME=/custom/path/.gity
```

## Permissions

### Full Disk Access

Gity may need Full Disk Access to monitor some directories:

1. Open **System Preferences** > **Security & Privacy** > **Privacy**
2. Select **Full Disk Access**
3. Click the lock to make changes
4. Add the `gity` binary

This is typically only needed for monitoring system directories or external volumes.

### Developer Tools

If prompted, allow Gity to use developer tools.

## Gatekeeper

If macOS blocks Gity from running:

```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine /usr/local/bin/gity
```

Or right-click the app and select "Open" to bypass Gatekeeper for that file.

## Apple Silicon (M1/M2/M3)

Gity runs natively on Apple Silicon. When building from source:

```bash
# For native ARM build
cargo build --release

# For universal binary (ARM + Intel)
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
lipo -create -output gity \
    target/aarch64-apple-darwin/release/gity \
    target/x86_64-apple-darwin/release/gity
```

## Case Sensitivity

macOS uses a case-insensitive filesystem by default (APFS/HFS+). This means:

- `File.txt` and `file.txt` refer to the same file
- Git handles case normalization
- Gity preserves the case as reported by the filesystem

If you need case sensitivity, format a volume as APFS (Case-sensitive).

## External Volumes

File watching works on external volumes, but:

- Network volumes (AFP, SMB) may not trigger events reliably
- External USB drives work if they use APFS/HFS+
- Some backup drives may be mounted read-only

## Time Machine

Gity's data directory (`~/.gity`) is included in Time Machine backups by default.

To exclude (saves backup space):

```bash
tmutil addexclusion ~/.gity
```

## Spotlight

The sled database in `~/.gity/data/sled` doesn't need to be indexed. macOS usually excludes hidden directories, but you can verify:

1. Open **System Preferences** > **Spotlight** > **Privacy**
2. Add `~/.gity` if you want to ensure it's excluded

## Performance Tips

1. **Use APFS** — Modern volumes are faster than HFS+

2. **Close unnecessary apps** — FSEvents performance degrades with many watchers

3. **Avoid network volumes** — Local storage is much faster

4. **SSD recommended** — Spinning disks are significantly slower

## Troubleshooting

### "Operation not permitted"

Grant Full Disk Access in System Preferences.

### Tray icon missing

1. Check System Preferences > Dock & Menu Bar
2. Ensure menu bar items are not hidden
3. Try restarting the daemon

### Slow after sleep

The watcher may need to reconcile missed events:

```bash
gity health /path/to/repo
git status  # Triggers reconciliation
```

### FSEvents not working

Rare, but can happen after macOS updates:

```bash
# Restart the FSEvents framework
sudo killall fseventsd
```

## Xcode Command Line Tools

Building from source requires Xcode Command Line Tools:

```bash
xcode-select --install
```

## Homebrew

If using Homebrew-installed Git:

```bash
# Ensure Homebrew Git is in your PATH
export PATH="/opt/homebrew/bin:$PATH"

# Verify version
git --version  # Should be 2.37+
```

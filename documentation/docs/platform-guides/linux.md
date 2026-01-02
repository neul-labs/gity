# Linux

Platform-specific guidance for running Gity on Linux.

## File Watching

Gity uses **inotify** for file watching on Linux. This is the kernel's native file notification system.

## inotify Limits

Linux limits the number of inotify watches per user. Large repositories can exceed this limit.

### Symptoms

- Watcher fails to start
- Some directories don't trigger events
- Error: "No space left on device" (misleading—it's about watches, not disk)

### Check Current Limit

```bash
cat /proc/sys/fs/inotify/max_user_watches
# Default is often 8192
```

### Increase Temporarily

```bash
sudo sysctl fs.inotify.max_user_watches=524288
```

### Increase Permanently

```bash
echo "fs.inotify.max_user_watches=524288" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

### Recommended Values

| Repository Size | Recommended Watches |
|-----------------|---------------------|
| < 10,000 files | 65536 |
| < 100,000 files | 262144 |
| < 500,000 files | 524288 |
| > 500,000 files | 1048576 |

## Installation

### From Source

```bash
cargo install --path crates/gity
```

### Debian/Ubuntu (.deb)

```bash
wget https://github.com/yourusername/gity/releases/latest/download/gity_amd64.deb
sudo dpkg -i gity_amd64.deb
```

### AppImage

```bash
wget https://github.com/yourusername/gity/releases/latest/download/gity-x86_64.AppImage
chmod +x gity-x86_64.AppImage
./gity-x86_64.AppImage
```

## System Service

Run Gity as a systemd user service for automatic startup.

### Create Service File

Create `~/.config/systemd/user/gity.service`:

```ini
[Unit]
Description=Gity Git Acceleration Daemon
After=default.target

[Service]
Type=simple
ExecStart=/usr/local/bin/gity daemon run
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

### Enable and Start

```bash
systemctl --user daemon-reload
systemctl --user enable gity
systemctl --user start gity
```

### Check Status

```bash
systemctl --user status gity
```

### View Logs

```bash
journalctl --user -u gity -f
```

## System Tray

The tray icon requires a system tray implementation:

- **GNOME**: Use an extension like "AppIndicator Support"
- **KDE Plasma**: Works natively
- **XFCE**: Works natively
- **i3/Sway**: Use a status bar with tray support (waybar, polybar, etc.)

If no tray is available, use CLI commands instead:

```bash
gity list --stats
gity daemon metrics
```

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

Gity needs:

- Read/write access to registered repositories
- Write access to `$GITY_HOME`
- Access to the inotify subsystem (usually available by default)

## SELinux

If SELinux is enabled and causing issues:

```bash
# Check for denials
sudo ausearch -m avc -ts recent

# Create a policy module if needed
# (Consult your distribution's documentation)
```

## AppArmor

If AppArmor is blocking Gity:

```bash
# Check status
sudo aa-status

# Disable profile temporarily (for testing only)
sudo aa-complain /usr/local/bin/gity
```

## Performance Tips

1. **Use fast storage** — SSD or NVMe significantly improves scan times

2. **Exclude from antivirus** — Add `~/.gity` to your antivirus exclusions

3. **tmpfs for cache** — For maximum speed, mount the cache on tmpfs:
   ```bash
   mkdir -p /tmp/gity-cache
   export GITY_HOME=/tmp/gity-cache
   ```
   Note: Cache is lost on reboot.

4. **Tune kernel** — For very large repos:
   ```bash
   # Increase file descriptor limit
   ulimit -n 65536

   # Increase inotify watches
   sudo sysctl fs.inotify.max_user_watches=1048576
   ```

## Troubleshooting

### "Too many open files"

Increase file descriptor limit:

```bash
# Temporary
ulimit -n 65536

# Permanent: add to /etc/security/limits.conf
*    soft    nofile    65536
*    hard    nofile    65536
```

### Slow on spinning disk

HDDs have high seek times. Consider:

- Using an SSD for development
- Increasing the status cache TTL
- Running `gity prefetch` less frequently

### Desktop notifications not working

Install a notification daemon:

```bash
# Debian/Ubuntu
sudo apt install dunst

# Fedora
sudo dnf install dunst
```

## Distribution-Specific Notes

### Ubuntu/Debian

```bash
# Install dependencies for building
sudo apt install build-essential git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Fedora

```bash
sudo dnf install @development-tools git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Arch Linux

```bash
sudo pacman -S base-devel git rust
```

### NixOS

Add to your configuration:

```nix
environment.systemPackages = with pkgs; [
  # gity (once packaged)
];
```

Or build from source in a nix-shell.

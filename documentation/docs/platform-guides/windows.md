# Windows

Platform-specific guidance for running Gity on Windows.

## File Watching

Gity uses **ReadDirectoryChangesW** for file watching on Windows. This is the native Windows API for directory change notifications.

## Installation

### MSI Installer

Download the MSI from [releases](https://github.com/yourusername/gity/releases) and run it.

The installer:

- Installs `gity.exe` to `C:\Program Files\Gity`
- Adds the install directory to PATH
- Optionally configures auto-start

### From Source

With Rust installed:

```powershell
cargo install --path crates/gity
```

### Scoop (Coming Soon)

```powershell
scoop install gity
```

## Auto-Start

### Task Scheduler

1. Open **Task Scheduler**
2. Create Basic Task > Name: "Gity Daemon"
3. Trigger: "When I log on"
4. Action: Start a program
5. Program: `C:\Program Files\Gity\gity.exe`
6. Arguments: `daemon run`
7. Finish

### Startup Folder

1. Press `Win+R`, type `shell:startup`
2. Create a shortcut to `gity.exe daemon run`

## Data Location

Default: `%APPDATA%\Gity`

```
%APPDATA%\Gity\
├── data\
│   └── sled\
├── logs\
│   └── daemon.log
└── config\
```

Typical path: `C:\Users\YourName\AppData\Roaming\Gity`

Override with:

```powershell
$env:GITY_HOME = "D:\gity-data"
```

## System Tray

The tray icon appears in the Windows notification area (system tray).

If the icon doesn't appear:

1. Click the up arrow (^) in the taskbar
2. Look for the Gity icon in the overflow area
3. Drag it to the main taskbar for quick access

## Long Paths

Windows has a default 260-character path limit (MAX_PATH). Deep directory structures may cause issues.

### Enable Long Paths (Windows 10+)

**Group Policy:**

1. Open `gpedit.msc`
2. Navigate to: Computer Configuration > Administrative Templates > System > Filesystem
3. Enable "Enable Win32 long paths"

**Registry:**

```powershell
reg add "HKLM\SYSTEM\CurrentControlSet\Control\FileSystem" /v LongPathsEnabled /t REG_DWORD /d 1 /f
```

Restart after making this change.

## Antivirus

Real-time antivirus scanning can slow down file operations. Consider adding exclusions:

### Windows Defender

1. Open **Windows Security**
2. Go to **Virus & threat protection** > **Manage settings**
3. Under **Exclusions**, click **Add or remove exclusions**
4. Add:
   - `C:\Program Files\Gity`
   - `%APPDATA%\Gity`
   - Your repository directories

### Other Antivirus

Consult your antivirus documentation to add similar exclusions.

## PowerShell

Use PowerShell for better Unicode support:

```powershell
# Check version
gity --version

# Register a repo
gity register C:\code\my-repo

# List repos
gity list --stats
```

## CMD (Command Prompt)

Also works in traditional Command Prompt:

```cmd
gity register C:\code\my-repo
gity list
```

## Git for Windows

Ensure Git for Windows is installed and accessible:

```powershell
git --version
# Should be 2.37 or higher
```

### PATH Configuration

Git for Windows should be in your PATH. Verify:

```powershell
where git
# Should show: C:\Program Files\Git\cmd\git.exe (or similar)
```

## Windows Terminal

For the best experience, use Windows Terminal:

- Better Unicode support
- Proper color rendering
- Multiple tabs

## WSL2

If you're using Windows Subsystem for Linux, see the dedicated [WSL2 Guide](wsl2.md).

Key points:

- Repos on Linux filesystem (`~/`) work well
- Repos on Windows filesystem (`/mnt/c/`) don't get proper file notifications

## Network Drives

File watching on mapped network drives (\\\\server\\share or Z:) is unreliable:

- SMB doesn't reliably propagate change notifications
- Consider disabling fsmonitor for network repos:

```powershell
git config core.fsmonitor false
```

## OneDrive

Repos in OneDrive-synced folders may have issues:

- File conflicts during sync
- Delayed change notifications
- Placeholder files that aren't fully downloaded

**Recommendation**: Keep repos in non-synced folders.

## Performance Tips

1. **Use SSD** — NVMe or SATA SSD significantly improves scan times

2. **Disable Windows Search indexing** — For repo directories:
   - Right-click folder > Properties > Advanced
   - Uncheck "Allow files in this folder to have contents indexed"

3. **Exclude from Defender** — Add repo folders to antivirus exclusions

4. **Use native Git** — Git for Windows performs better than Git in WSL2 for Windows-hosted repos

## Firewall

Gity uses local TCP (default: port 7557) for IPC. This shouldn't trigger firewall prompts since it's localhost-only.

If you see a firewall prompt:

- Allow private networks only
- No need for public network access

## Troubleshooting

### "Access is denied"

Run PowerShell as Administrator if accessing protected directories.

Or ensure you have permissions on the repository folder.

### Tray icon not visible

1. Check the overflow area (^ in taskbar)
2. Ensure background apps are allowed:
   - Settings > Privacy > Background apps
   - Allow Gity to run in background

### Slow performance

1. Check antivirus exclusions
2. Disable Windows Search indexing on repos
3. Ensure repos are on SSD
4. Close unnecessary applications

### "The system cannot find the path specified"

Check that:

- The path exists
- You have read/write access
- The path isn't too long (enable long paths if needed)

### Git not found

Add Git to PATH:

```powershell
# Add to current session
$env:PATH += ";C:\Program Files\Git\cmd"

# Add permanently (run as Admin)
[Environment]::SetEnvironmentVariable("PATH", $env:PATH + ";C:\Program Files\Git\cmd", "Machine")
```

## Visual Studio / Visual Studio Code

Both work seamlessly with Gity. The accelerated `git status` benefits:

- VS Code's Git integration
- Visual Studio's Git Changes window
- Any extension that calls Git

## Building from Source

Requirements:

- Visual Studio Build Tools or Visual Studio (with C++ workload)
- Rust (via rustup)

```powershell
# Install Rust
winget install Rustlang.Rustup

# Build
cargo build --release
```

The binary is at `target\release\gity.exe`.

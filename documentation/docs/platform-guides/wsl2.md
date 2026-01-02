# WSL2 (Windows Subsystem for Linux)

Special considerations for running Gity in WSL2.

## The Challenge

WSL2 has significant file system notification limitations that affect Gity's file watching.

## Filesystem Types in WSL2

### Linux Filesystem (`~/`)

```
WSL2 (ext4)             Windows
┌──────────────┐       ┌──────────────┐
│ ~/code/repo  │ ←───→ │ \\wsl$\...   │
│              │       │              │
└──────────────┘       └──────────────┘
```

- Files stored natively in WSL2's ext4 filesystem
- **inotify works correctly**
- Gity functions properly
- Best performance

### Windows Filesystem (`/mnt/c/`)

```
Windows (NTFS)          WSL2 (Linux)
┌──────────────┐       ┌──────────────┐
│ C:\code\repo │ ←───→ │ /mnt/c/code/ │
│              │  9P   │    repo      │
└──────────────┘       └──────────────┘
```

- Files accessed via 9P protocol
- **inotify does NOT work** across this boundary
- File changes often missed
- Gity will not function correctly

## The Rule

!!! warning "Critical"
    Keep your repositories on the **Linux filesystem** (`~/`), not on the Windows filesystem (`/mnt/c/`).

## Recommendations

| Workflow | Recommendation |
|----------|----------------|
| Repo on `/mnt/c/`, edit in Windows | Run Gity natively on Windows |
| Repo on `/mnt/c/`, edit in WSL2 | Move repo to Linux filesystem |
| Repo on `~/`, edit in WSL2 | Works correctly |
| Repo on `~/`, edit in Windows via `\\wsl$\` | Works correctly |

## Setting Up Correctly

### Clone to Linux Filesystem

```bash
# Good - on Linux filesystem
cd ~
git clone https://github.com/org/repo.git
gity register ~/repo  # Works correctly

# Bad - on Windows filesystem
gity register /mnt/c/Users/me/repo  # Will miss events!
```

### Move Existing Repo

```bash
# Move from Windows to Linux filesystem
cp -r /mnt/c/Users/me/code/repo ~/code/repo
gity register ~/code/repo
```

### Check Filesystem Type

```bash
# Check if path is on Windows filesystem
df -T /path/to/repo

# 9p or drvfs = Windows filesystem (bad)
# ext4 = Linux filesystem (good)
```

## Detecting WSL2

```bash
# Check if running in WSL
if grep -qi microsoft /proc/version; then
    echo "Running in WSL"
fi

# Check WSL version
wsl.exe -l -v
```

## Editor Workflows

### VS Code Remote - WSL

Works well:

1. Open VS Code on Windows
2. Connect to WSL using Remote - WSL extension
3. Open folder from Linux filesystem (`~/...`)
4. Gity running in WSL monitors correctly

### JetBrains IDEs

With Gateway or Remote Development:

1. Connect to WSL2
2. Open project from Linux filesystem
3. Gity in WSL monitors correctly

### Windows Editor, Linux Repo

If editing `\\wsl$\Ubuntu\home\user\repo` from Windows:

- Changes made from Windows are detected by WSL2's inotify
- Gity in WSL2 will see these changes
- This works correctly

## Performance Comparison

| Location | Read Speed | Write Speed | Gity Works? |
|----------|------------|-------------|-------------|
| Linux FS (`~/`) | Fast | Fast | Yes |
| Windows FS (`/mnt/c/`) | Slow | Slow | No |
| Windows FS with WSL1 | Medium | Medium | Partially |

## Docker in WSL2

When using Docker Desktop with WSL2 backend:

### Repo on Windows

```bash
docker run -v C:\code:/app ...
```

- Same 9P limitations apply
- Consider mounting from Linux filesystem instead

### Repo on Linux

```bash
docker run -v ~/code:/app ...
```

- Better performance
- File events work inside container (for Linux containers)

## Troubleshooting

### Events Not Working

1. Verify filesystem type:
   ```bash
   df -T /path/to/repo
   ```

2. If it shows `9p` or `drvfs`, the repo is on Windows filesystem

3. Move to Linux filesystem:
   ```bash
   mv /mnt/c/Users/me/repo ~/repo
   gity unregister /mnt/c/Users/me/repo
   gity register ~/repo
   ```

### Slow Performance

Windows filesystem in WSL2 is slow due to 9P overhead:

```bash
# Check if on Windows filesystem
df -T .

# If 9p/drvfs, consider moving to ~/
```

### "No space left on device"

inotify watch limit reached. Increase it:

```bash
# Check current limit
cat /proc/sys/fs/inotify/max_user_watches

# Increase temporarily
sudo sysctl fs.inotify.max_user_watches=524288

# Increase permanently
echo "fs.inotify.max_user_watches=524288" | sudo tee -a /etc/sysctl.conf
```

### Git Version

WSL2 distributions may have older Git. Update:

```bash
# Ubuntu/Debian
sudo add-apt-repository ppa:git-core/ppa
sudo apt update
sudo apt install git

# Verify version (needs 2.37+)
git --version
```

## Best Practices

1. **Always use Linux filesystem** — Keep repos in `~/` not `/mnt/c/`

2. **Use VS Code Remote - WSL** — Best editor integration for WSL development

3. **Install Gity in WSL** — Not on Windows, when developing in WSL

4. **Increase inotify limits** — WSL2 defaults may be too low

5. **Use native Linux tools** — Git, Gity, and other tools should run in WSL, not Windows

## Hybrid Workflows

If you must access repos from both Windows and WSL2:

### Option 1: Run Gity on Windows

- Keep repo on `C:\`
- Install and run Gity on Windows
- Access from WSL2 via `/mnt/c/`
- File watching works (via Windows)

### Option 2: Dual Registration

Not recommended, but possible:

```bash
# On Windows
gity register C:\code\repo

# In WSL2 (won't work well for /mnt/c/)
# Only register repos on Linux filesystem
gity register ~/other-repo
```

## WSL1 vs WSL2

| Feature | WSL1 | WSL2 |
|---------|------|------|
| Filesystem | Translates to NTFS | Native ext4 |
| Windows FS access | Direct | Via 9P |
| File notifications | Works on `/mnt/c/` | Broken on `/mnt/c/` |
| Recommendation | Consider for Windows FS repos | Use Linux FS only |

For Gity, WSL2 with repos on Linux filesystem is the best choice.

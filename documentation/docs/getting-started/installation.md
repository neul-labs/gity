# Installation

Gity is distributed as a single binary that works on Linux, macOS, and Windows.

## Requirements

- **Git 2.37+** — Required for fsmonitor protocol v2 support
- **Rust 1.75+** — Only if building from source

Check your Git version:

```bash
git --version
# Should be 2.37 or higher
```

## Installation Methods

=== "From Source (Recommended)"

    If you have Rust installed:

    ```bash
    cargo install --path crates/gity
    ```

    Or clone and build:

    ```bash
    git clone https://github.com/yourusername/gity
    cd gity
    cargo build --release
    # Binary is at ./target/release/gity
    ```

=== "Linux"

    **Debian/Ubuntu (.deb)**

    ```bash
    # Download from releases
    wget https://github.com/yourusername/gity/releases/latest/download/gity_amd64.deb
    sudo dpkg -i gity_amd64.deb
    ```

    **AppImage**

    ```bash
    wget https://github.com/yourusername/gity/releases/latest/download/gity-x86_64.AppImage
    chmod +x gity-x86_64.AppImage
    ./gity-x86_64.AppImage
    ```

=== "macOS"

    **Homebrew** (coming soon)

    ```bash
    brew install gity
    ```

    **Package Installer**

    Download the `.pkg` installer from [releases](https://github.com/yourusername/gity/releases).

=== "Windows"

    **MSI Installer**

    Download the MSI installer from [releases](https://github.com/yourusername/gity/releases).

    **Scoop** (coming soon)

    ```powershell
    scoop install gity
    ```

## Verify Installation

After installation, verify Gity is working:

```bash
gity --version
```

## Next Steps

Once installed, head to the [Quick Start](quick-start.md) guide to accelerate your first repository.

## Uninstallation

=== "From Source"

    ```bash
    cargo uninstall gity
    ```

=== "Linux (.deb)"

    ```bash
    sudo dpkg -r gity
    ```

=== "macOS"

    Run the uninstaller or remove manually:

    ```bash
    sudo rm /usr/local/bin/gity
    ```

=== "Windows"

    Use "Add or Remove Programs" in Windows Settings, or run:

    ```powershell
    msiexec /x gity.msi
    ```

Before uninstalling, make sure to unregister all repositories:

```bash
gity list
gity unregister /path/to/repo1
gity unregister /path/to/repo2
gity daemon stop
```

This ensures Git configuration is cleaned up properly.

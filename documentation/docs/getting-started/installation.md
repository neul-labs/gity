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

=== "cargo (Recommended)"

    If you have Rust installed:

    ```bash
    cargo install gity
    ```

    With system tray support:

    ```bash
    cargo install gity --features tray
    ```

    Or clone and build from source:

    ```bash
    git clone https://github.com/neul-labs/gity.git
    cd gity
    cargo build --release
    # Binary is at ./target/release/gity
    ```

=== "Homebrew (macOS & Linux)"

    ```bash
    brew tap neul-labs/tap
    brew install gity
    ```

=== "npm"

    ```bash
    npm install -g gity-cli
    ```

    Or use npx without installing:

    ```bash
    npx gity-cli register /path/to/repo
    ```

=== "pip"

    ```bash
    pip install gity-cli
    ```

    Or with uv:

    ```bash
    uv tool install gity-cli
    ```

=== "Linux"

    **Debian/Ubuntu (.deb)**

    Download from [GitHub Releases](https://github.com/neul-labs/gity/releases):

    ```bash
    wget https://github.com/neul-labs/gity/releases/latest/download/gity-<version>-x86_64-unknown-linux-gnu.tar.gz
    tar -xzf gity-<version>-x86_64-unknown-linux-gnu.tar.gz
    sudo cp gity /usr/local/bin/
    ```

    **Snap**

    ```bash
    snap install gity
    ```

=== "macOS"

    **Homebrew** (recommended)

    ```bash
    brew tap neul-labs/tap
    brew install gity
    ```

    **Package Installer**

    Download the `.pkg` installer from [GitHub Releases](https://github.com/neul-labs/gity/releases).

=== "Windows"

    **MSI Installer**

    Download the MSI installer from [GitHub Releases](https://github.com/neul-labs/gity/releases).

    **Chocolatey**

    ```powershell
    choco install gity
    ```

    **npm**

    ```powershell
    npm install -g gity-cli
    ```

## Verify Installation

After installation, verify Gity is working:

```bash
gity --version
```

## Supply Chain Security

All release artifacts are built and published via GitHub Actions with **OIDC-based trusted publishing** and **signed attestations**:

- **crates.io** — [Trusted Publishing](https://crates.io/docs/trusted-publishing) (OIDC)
- **PyPI** — [Trusted Publishing](https://docs.pypi.org/trusted-publishers/) (OIDC)
- **npm** — [Trusted Publishing](https://docs.npmjs.com/generating-provenance-statements) with automatic provenance
- **GitHub Releases** — Attested with `actions/attest-build-provenance`

Verify a release binary:

```bash
gh attestation verify gity-0.1.2-x86_64-unknown-linux-gnu.tar.gz --owner neul-labs
```

## Next Steps

Once installed, head to the [Quick Start](quick-start.md) guide to accelerate your first repository.

## Uninstallation

=== "cargo"

    ```bash
    cargo uninstall gity
    ```

=== "Homebrew"

    ```bash
    brew uninstall gity
    brew untap neul-labs/tap
    ```

=== "npm"

    ```bash
    npm uninstall -g gity-cli
    ```

=== "pip"

    ```bash
    pip uninstall gity
    ```

=== "Linux (.deb)"

    ```bash
    sudo dpkg -r gity
    ```

=== "Snap"

    ```bash
    snap remove gity
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

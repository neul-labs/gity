"""Gity - Make large Git repositories feel instant."""

__version__ = "0.1.2"

import sys
import platform
import os
import shutil
import hashlib
import urllib.request

BINARY_URLS = {
    ("x86_64", "Linux"): "https://github.com/neul-labs/gity/releases/download/v{version}/gity-{version}-x86_64-unknown-linux-gnu.tar.gz",
    ("aarch64", "Linux"): "https://github.com/neul-labs/gity/releases/download/v{version}/gity-{version}-aarch64-unknown-linux-gnu.tar.gz",
    ("x86_64", "Darwin"): "https://github.com/neul-labs/gity/releases/download/v{version}/gity-{version}-x86_64-apple-darwin.tar.gz",
    ("aarch64", "Darwin"): "https://github.com/neul-labs/gity/releases/download/v{version}/gity-{version}-aarch64-apple-darwin.tar.gz",
    ("x86_64", "Windows"): "https://github.com/neul-labs/gity/releases/download/v{version}/gity-{version}-x86_64-pc-windows-msvc.zip",
}


def get_platform_key():
    """Get platform key for binary selection."""
    arch = platform.machine()
    if arch == "x86_64":
        arch = "x86_64"
    elif arch == "aarch64" or arch == "arm64":
        arch = "aarch64"
    else:
        raise RuntimeError(f"Unsupported architecture: {arch}")
    return (arch, platform.system())


def get_install_dir():
    """Get the installation directory for the binary."""
    home = os.path.expanduser("~/.local")
    if platform.system() == "Windows":
        home = os.environ.get("LOCALAPPDATA", os.path.expanduser("~\\AppData\\Local"))
    return os.path.join(home, "gity")


def get_binary_path():
    """Get the path to the installed binary."""
    install_dir = get_install_dir()
    if platform.system() == "Windows":
        return os.path.join(install_dir, "gity.exe")
    return os.path.join(install_dir, "gity")


def ensure_binary_installed():
    """Download and install the binary if not present."""
    binary_path = get_binary_path()
    if os.path.exists(binary_path):
        return

    install_dir = get_install_dir()
    os.makedirs(install_dir, exist_ok=True)

    arch, system = get_platform_key()
    url_template = BINARY_URLS.get((arch, system))
    if not url_template:
        raise RuntimeError(f"No binary available for {arch} {system}")

    url = url_template.format(version=__version__)
    print(f"Downloading Gity {__version__} for {arch} {system}...")

    # Download
    tmp_path = binary_path + ".tmp"
    urllib.request.urlretrieve(url, tmp_path)

    # Extract
    if platform.system() == "Windows":
        import zipfile
        with zipfile.ZipFile(tmp_path, "r") as z:
            z.extractall(install_dir)
    else:
        import tarfile
        with tarfile.open(tmp_path, "r:gz") as t:
            t.extractall(install_dir)

    os.remove(tmp_path)
    os.chmod(binary_path, 0o755)
    print(f"Installed Gity to {binary_path}")


def main():
    """Entry point for the gity command."""
    ensure_binary_installed()
    binary_path = get_binary_path()

    # Execute the binary with any arguments
    os.execv(binary_path, [binary_path] + sys.argv[1:])


if __name__ == "__main__":
    main()

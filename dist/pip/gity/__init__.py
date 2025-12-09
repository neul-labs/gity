"""
Gity - Make large Git repositories feel instant.

This package provides a Python wrapper for the gity binary.
"""

import os
import platform
import subprocess
import sys
import urllib.request
import tarfile
import zipfile
import tempfile
from pathlib import Path

__version__ = "0.1.0"

REPO = "neul-labs/gity"


def get_platform_target():
    """Get the platform-specific target triple."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    platform_map = {
        ("darwin", "x86_64"): "x86_64-apple-darwin",
        ("darwin", "arm64"): "aarch64-apple-darwin",
        ("linux", "x86_64"): "x86_64-unknown-linux-gnu",
        ("linux", "aarch64"): "aarch64-unknown-linux-gnu",
        ("windows", "amd64"): "x86_64-pc-windows-msvc",
        ("windows", "x86_64"): "x86_64-pc-windows-msvc",
    }

    key = (system, machine)
    target = platform_map.get(key)

    if not target:
        raise RuntimeError(f"Unsupported platform: {system}-{machine}")

    return system, target


def get_binary_path():
    """Get the path to the gity binary."""
    bin_dir = Path(__file__).parent / "bin"
    system = platform.system().lower()
    binary_name = "gity.exe" if system == "windows" else "gity"
    return bin_dir / binary_name


def ensure_binary():
    """Ensure the gity binary is installed."""
    binary_path = get_binary_path()

    if binary_path.exists():
        return binary_path

    system, target = get_platform_target()
    ext = "zip" if system == "windows" else "tar.gz"

    url = f"https://github.com/{REPO}/releases/download/v{__version__}/gity-{__version__}-{target}.{ext}"

    print(f"Downloading gity {__version__} for {target}...")

    bin_dir = binary_path.parent
    bin_dir.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory() as tmpdir:
        archive_path = Path(tmpdir) / f"gity.{ext}"

        urllib.request.urlretrieve(url, archive_path)

        if ext == "tar.gz":
            with tarfile.open(archive_path, "r:gz") as tar:
                tar.extractall(bin_dir)
        else:
            with zipfile.ZipFile(archive_path, "r") as zip_ref:
                zip_ref.extractall(bin_dir)

    if system != "windows":
        binary_path.chmod(0o755)

    print("gity installed successfully!")
    return binary_path


def main():
    """Main entry point - runs the gity binary with passed arguments."""
    try:
        binary_path = ensure_binary()
        result = subprocess.run([str(binary_path)] + sys.argv[1:])
        sys.exit(result.returncode)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        print("You can install manually from: https://github.com/neul-labs/gity/releases")
        sys.exit(1)


if __name__ == "__main__":
    main()

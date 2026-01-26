#!/bin/bash
# Build script for platform packages
set -e

VERSION="0.1.0"
PROJECT_ROOT="$(dirname "$0")/.."

build_deb() {
    echo "Building Debian package..."
    cd "$PROJECT_ROOT"

    # Build release binary
    cargo build --release

    # Create package structure
    PKG_DIR="packaging/debian/gity_${VERSION}_amd64"
    mkdir -p "$PKG_DIR/DEBIAN"
    mkdir -p "$PKG_DIR/usr/bin"
    mkdir -p "$PKG_DIR/usr/share/doc/gity"

    # Copy files
    cp target/release/gity "$PKG_DIR/usr/bin/"
    cp packaging/debian/control "$PKG_DIR/DEBIAN/"
    cp README.md "$PKG_DIR/usr/share/doc/gity/"

    # Set permissions
    chmod 755 "$PKG_DIR/usr/bin/gity"

    # Build package
    dpkg-deb --build "$PKG_DIR"

    echo "Built: packaging/debian/gity_${VERSION}_amd64.deb"
}

build_macos() {
    echo "Building macOS app bundle..."
    cd "$PROJECT_ROOT"

    # Build release binary
    cargo build --release

    # Create app bundle structure
    APP_DIR="packaging/macos/Gity.app"
    mkdir -p "$APP_DIR/Contents/MacOS"
    mkdir -p "$APP_DIR/Contents/Resources"

    # Copy files
    cp target/release/gity "$APP_DIR/Contents/MacOS/"
    cp packaging/macos/Info.plist "$APP_DIR/Contents/"

    # Create DMG (requires create-dmg or hdiutil)
    if command -v create-dmg &> /dev/null; then
        create-dmg \
            --volname "Gity Installer" \
            --window-pos 200 120 \
            --window-size 600 400 \
            --icon-size 100 \
            --app-drop-link 450 185 \
            "packaging/macos/Gity-${VERSION}.dmg" \
            "$APP_DIR"
        echo "Built: packaging/macos/Gity-${VERSION}.dmg"
    else
        echo "App bundle created at: $APP_DIR"
        echo "Install create-dmg to generate DMG file"
    fi
}

build_windows() {
    echo "Building Windows MSI..."
    echo "Note: Run this on Windows with WiX Toolset installed"

    cd "$PROJECT_ROOT"

    # Build release binary
    cargo build --release --target x86_64-pc-windows-msvc

    # WiX build commands (run on Windows)
    # candle.exe -dSourceDir="target/x86_64-pc-windows-msvc/release" packaging/windows/gity.wxs
    # light.exe -ext WixUIExtension gity.wixobj -o packaging/windows/gity-${VERSION}.msi

    echo "See packaging/windows/gity.wxs for WiX configuration"
}

case "$1" in
    deb)
        build_deb
        ;;
    macos)
        build_macos
        ;;
    windows)
        build_windows
        ;;
    all)
        build_deb
        build_macos
        build_windows
        ;;
    *)
        echo "Usage: $0 {deb|macos|windows|all}"
        exit 1
        ;;
esac

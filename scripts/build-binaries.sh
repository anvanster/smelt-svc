#!/bin/bash
# Build multi-platform binaries for Smelt
# On macOS ARM: builds both ARM and Intel binaries
# On other platforms: builds native only

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Create bin directory for packaged binaries
mkdir -p bin
mkdir -p prebuilt

echo "Building Smelt binaries..."

# Detect current platform
ARCH=$(uname -m)
OS=$(uname -s)

echo "Detected platform: $OS $ARCH"

# Build for current platform
echo ""
echo "Building for current platform..."
cargo build --release

# Copy binaries to prebuilt directory
if [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "arm64" ]; then
        PLATFORM_SUFFIX="darwin-arm64"
    else
        PLATFORM_SUFFIX="darwin-x64"
    fi
elif [ "$OS" = "Linux" ]; then
    PLATFORM_SUFFIX="linux-x64"
else
    PLATFORM_SUFFIX="win32-x64"
fi

echo "Copying binaries to prebuilt/ with suffix: $PLATFORM_SUFFIX"
cp target/release/smelt "prebuilt/smelt-$PLATFORM_SUFFIX" 2>/dev/null || true
cp target/release/smelt-mcp "prebuilt/smelt-mcp-$PLATFORM_SUFFIX" 2>/dev/null || true

# Cross-compile for x86_64 on macOS ARM
if [ "$OS" = "Darwin" ] && [ "$ARCH" = "arm64" ]; then
    echo ""
    echo "Cross-compiling for macOS Intel (x86_64)..."

    # Check if x86_64 target is installed
    if ! rustup target list --installed | grep -q "x86_64-apple-darwin"; then
        echo "Installing x86_64-apple-darwin target..."
        rustup target add x86_64-apple-darwin
    fi

    cargo build --release --target x86_64-apple-darwin

    echo "Copying x86_64 binaries..."
    cp target/x86_64-apple-darwin/release/smelt "prebuilt/smelt-darwin-x64" 2>/dev/null || true
    cp target/x86_64-apple-darwin/release/smelt-mcp "prebuilt/smelt-mcp-darwin-x64" 2>/dev/null || true
fi

# Copy all prebuilt binaries to bin/ for packaging
echo ""
echo "Copying prebuilt binaries to bin/..."
cp prebuilt/* bin/ 2>/dev/null || true

echo ""
echo "Build complete! Binaries in bin/:"
ls -lh bin/

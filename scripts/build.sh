#!/bin/bash
# Build script for Smelt
# Usage: ./scripts/build.sh [options]
#   --clean         Clean build artifacts
#   --release       Build release binaries
#   --all-platforms Build for all platforms (macOS ARM + Intel)
#   --test          Run tests after build
#   --check         Run CI checks (fmt, clippy, test)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Parse arguments
CLEAN=false
RELEASE=false
ALL_PLATFORMS=false
RUN_TESTS=false
CHECK=false

for arg in "$@"; do
    case $arg in
        --clean)
            CLEAN=true
            ;;
        --release)
            RELEASE=true
            ;;
        --all-platforms)
            ALL_PLATFORMS=true
            ;;
        --test)
            RUN_TESTS=true
            ;;
        --check)
            CHECK=true
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Usage: $0 [--clean] [--release] [--all-platforms] [--test] [--check]"
            exit 1
            ;;
    esac
done

# Clean if requested
if [ "$CLEAN" = true ]; then
    echo "Cleaning build artifacts..."
    cargo clean
    rm -rf bin/
    echo "Clean complete."
fi

# Run CI checks if requested
if [ "$CHECK" = true ]; then
    echo "Running CI checks..."

    echo "Checking formatting..."
    cargo fmt --check

    echo "Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings

    echo "Running tests..."
    cargo test --workspace

    echo "CI checks passed!"
    exit 0
fi

# Build
if [ "$RELEASE" = true ]; then
    echo "Building release binaries..."
    cargo build --release

    if [ "$ALL_PLATFORMS" = true ]; then
        echo "Building for all platforms..."
        "$SCRIPT_DIR/build-binaries.sh"
    fi
else
    echo "Building debug binaries..."
    cargo build
fi

# Run tests if requested
if [ "$RUN_TESTS" = true ]; then
    echo "Running tests..."
    cargo test --workspace
fi

echo ""
echo "Build complete!"

if [ "$RELEASE" = true ]; then
    echo "Binaries:"
    ls -lh target/release/smelt target/release/smelt-mcp 2>/dev/null || true
fi

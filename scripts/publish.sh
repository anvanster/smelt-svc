#!/bin/bash
# Publish Smelt crates to crates.io
# Usage: ./scripts/publish.sh [crate-name]
#   No args: publish all crates in dependency order
#   crate-name: publish specific crate (e.g., smelt-core, smelt-mcp)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Crates in dependency order
CRATES=(
    "smelt-core"
    "smelt-memory"
    "smelt-validator"
    "smelt-cli"
    "smelt-api"
    "smelt-mcp"
)

publish_crate() {
    local crate=$1
    echo ""
    echo "=========================================="
    echo "Publishing $crate..."
    echo "=========================================="

    # Check if crate exists
    if [ ! -d "crates/$crate" ]; then
        echo "Error: crate $crate not found"
        return 1
    fi

    # Publish with --allow-dirty in case of uncommitted version bumps
    cargo publish -p "$crate" --allow-dirty

    echo "Published $crate successfully!"

    # Wait for crates.io to index (required for dependent crates)
    echo "Waiting for crates.io to index..."
    sleep 30
}

# Run CI checks first
echo "Running CI checks before publish..."
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace

echo ""
echo "CI checks passed!"

if [ -n "$1" ]; then
    # Publish specific crate
    publish_crate "$1"
else
    # Publish all crates
    echo ""
    echo "Publishing all crates in dependency order..."
    echo "Crates: ${CRATES[*]}"
    echo ""
    read -p "Continue? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 1
    fi

    for crate in "${CRATES[@]}"; do
        publish_crate "$crate"
    done
fi

echo ""
echo "=========================================="
echo "All crates published successfully!"
echo "=========================================="

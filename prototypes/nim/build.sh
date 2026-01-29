#!/usr/bin/env bash
# Build script for yx Nim prototype

set -e

echo "Building yx (Nim prototype)..."

# Build release binary
nim c -d:release --opt:size src/yx.nim

echo "Build complete!"
echo "Binary: src/yx ($(ls -lh src/yx | awk '{print $5}'))"
echo ""
echo "To install:"
echo "  cp src/yx /usr/local/bin/yx"
echo ""
echo "To run tests:"
echo "  shellspec spec/features/"

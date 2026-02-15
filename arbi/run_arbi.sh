#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"

echo "╔═══════════════════════════════════════════════════════╗"
echo "║              ARBI — Polymarket Arbitrage Bot          ║"
echo "╚═══════════════════════════════════════════════════════╝"

# Load env from integration/.env if available
ENV_FILE="$SCRIPT_DIR/../integration/.env"
if [ -f "$ENV_FILE" ]; then
    echo "📁 Loading env from integration/.env"
    set -a
    source "$ENV_FILE"
    set +a
fi

# Build if needed
if [ ! -f "$BUILD_DIR/arbi" ]; then
    echo "🔨 Building arbi..."
    mkdir -p "$BUILD_DIR"
    cd "$BUILD_DIR"
    cmake .. -DCMAKE_BUILD_TYPE=Release
    make -j$(sysctl -n hw.ncpu)
    cd "$SCRIPT_DIR"
    echo "✅ Build complete"
fi

# Run with all arguments passed through
echo "🚀 Starting arbi..."
exec "$BUILD_DIR/arbi" "$@"

#!/bin/bash

# Polymaster Launcher
# Starts Whale Watcher (Rust) and Dashboard in parallel.

# Function to kill background processes on exit (Ctrl+C)
cleanup() {
    echo ""
    echo "🛑 Stopping all services..."
    kill $(jobs -p) 2>/dev/null
    echo "✅ System stopped."
}
trap cleanup EXIT INT

echo "🚀 Launching Polymaster System..."
echo "================================="

# 1. Start Rust Whale Watcher
echo "🐋 Starting Whale Watcher (Rust)..."
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found."
    exit 1
fi
# Load Environment Variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

# Check if release binary exists, otherwise build it
if [ ! -f "target/release/wwatcher" ]; then
    echo "🔨 Building Release Binary (First Run)..."
    cargo build --release --bin wwatcher
fi
# cargo build --bin wwatcher
./target/release/wwatcher watch > watcher_rust.log 2>&1 &
WATCHER_PID=$!
echo "   (Logs: watcher_rust.log)"

# 2. Start Dashboard
echo "📊 Starting Dashboard..."
if ! command -v node &> /dev/null; then
    echo "❌ Node.js not found."
    exit 1
fi
node dashboard/server.js > dashboard.log 2>&1 &
DASH_PID=$!
echo "   (Logs: dashboard.log)"

echo "   👉 Dashboard available at http://localhost:3000"

# Wait for all processes
wait

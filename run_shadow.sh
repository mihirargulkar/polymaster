#!/bin/bash

# Polymaster Full Pipeline Launcher (Shadow Mode + Discord Bridge)
# This script starts the Rust watcher, the Discord bridge, and the Node autopilot.

echo "🚀 Launching Polymaster Trading Pipeline (Shadow Mode)..."

# Ensure we are in the root directory
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

# 1. Kill any existing instances to ensure clean start
echo "🧹 Cleaning up existing processes..."
pkill -f "node dist/index.js --mode=shadow" 2>/dev/null
pkill -f "target/release/wwatcher watch" 2>/dev/null
pkill -f "node discord-bridge.js" 2>/dev/null
sleep 1

# 2. Start the Discord Bridge (Converts raw JSON to rich Discord embeds)
echo "🌉 Starting Discord Bridge (Node)..."
nohup caffeinate -i node discord-bridge.js > discord_bridge.log 2>&1 &
BRIDGE_PID=$!

# 3. Start the Rust Watcher (Sources signals -> Discord Bridge & JSONL)
echo "📡 Starting Whale Watcher (Rust)..."
# Uses ~/.config/wwatcher/config.json which points to localhost:3000
nohup caffeinate -i ./target/release/wwatcher watch --threshold 10000 --interval 10 > watcher_rust.log 2>&1 &
WATCHER_PID=$!

# 4. Start the Shadow Autopilot (Consumes signals -> AI Research -> Shadow Trades)
echo "🤖 Starting Shadow Autopilot (Node)..."
cd integration
if [ ! -d "dist" ]; then
    echo "📦 Building integration layer..."
    npm run build
fi
nohup caffeinate -i node dist/index.js --mode=shadow > autopilot.log 2>&1 &
AUTOPILOT_PID=$!

echo "✅ Pipeline started successfully!"
echo "   - Bridge PID:    $BRIDGE_PID"
echo "   - Watcher PID:   $WATCHER_PID"
echo "   - Autopilot PID: $AUTOPILOT_PID"
echo "📝 Tailing autopilot.log (Ctrl+C to stop trailing, bot will keep running)..."
echo "---------------------------------------------------------------------------"

# Tail the log
tail -f autopilot.log

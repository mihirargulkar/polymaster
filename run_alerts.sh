#!/bin/bash
# run_alerts.sh - Start Whale Watcher with Discord Alerts

# Define colors
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${CYAN}🐳 Whale Watcher Discord Alert System${NC}"
echo "======================================="

# Check for node
if ! command -v node &> /dev/null; then
    echo -e "${RED}Error: node is not installed.${NC}"
    exit 1
fi

# Check for cargo
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo is not installed.${NC}"
    exit 1
fi

# 1. Install/Update dependencies for discord-bridge
echo -e "${YELLOW}📦 Checking Discord Bridge dependencies...${NC}"
if [ ! -d "node_modules" ]; then
    npm install express axios sqlite3
fi

# 2. Start Discord Bridge (Server)
echo -e "${YELLOW}🚀 Starting Discord Bridge...${NC}"
# Kill any existing bridge on port 3000
lsof -ti:3000 | xargs kill -9 2>/dev/null

node discord-bridge.js > discord_bridge.log 2>&1 &
BRIDGE_PID=$!
echo -e "${GREEN}   Bridge started (PID: $BRIDGE_PID)${NC}"
echo "   Logs: discord_bridge.log"

# Trap cleanup to kill bridge when script exits
cleanup() {
    echo -e "\n${YELLOW}🛑 Stopping Discord Bridge...${NC}"
    kill $BRIDGE_PID 2>/dev/null
    exit
}
trap cleanup SIGINT SIGTERM EXIT

# 3. Build & Run Whale Watcher (Client)
echo -e "${YELLOW}🐋 Starting Whale Watcher...${NC}"

CONFIG_FILE="$HOME/.config/wwatcher/config.json"

# Check if configured
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${YELLOW}⚠️  Configuration not found. Running setup first...${NC}"
    echo "   IMPORTANT: Set Webhook URL to: http://localhost:3000/webhook/whale-alerts"
    cargo run --bin wwatcher -- setup
else
    # Check if webhook is configured correctly
    if ! grep -q "webhook" "$CONFIG_FILE"; then
         echo -e "${YELLOW}⚠️  Webhook not configured in $CONFIG_FILE${NC}"
         echo "   Please run 'cargo run --bin wwatcher -- setup' and set Webhook URL to:"
         echo -e "   ${CYAN}http://localhost:3000/webhook/whale-alerts${NC}"
         echo "   Waiting 5 seconds before starting anyway..."
         sleep 5
    fi
fi

# Run watch command
cargo run --bin wwatcher -- watch

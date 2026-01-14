# Quick Start Guide

## DISCLAIMER

This tool is for informational and research purposes only. Use this data solely for informed decision-making and market analysis.

---

## Installation

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Clone and build
git clone https://github.com/neur0map/polymaster.git
cd polymaster
cargo build --release
cargo install --path .
```

## Basic Usage

```bash
# Start monitoring with default settings ($25k threshold, 5 second polling)
wwatcher watch

# Customize threshold and polling interval
wwatcher watch --threshold 50000 --interval 30
```

## Running as a System Service (Linux)

To run the watcher continuously as a background service:

### Step 1: Configure webhook (optional)

```bash
mkdir -p ~/.config/wwatcher
cat > ~/.config/wwatcher/config.json << 'EOF'
{
  "webhook_url": "https://your-webhook-url.com/webhook/polymaster"
}
EOF
```

### Step 2: Create systemd service file

```bash
sudo tee /etc/systemd/system/wwatcher.service > /dev/null << EOF
[Unit]
Description=Polymaster Whale Watcher
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME
ExecStart=$HOME/.cargo/bin/wwatcher watch --threshold 28000 --interval 5
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
```

### Step 3: Start and enable service

```bash
sudo systemctl daemon-reload
sudo systemctl enable wwatcher.service
sudo systemctl start wwatcher.service
```

### Service Management Commands

```bash
# Check service status
sudo systemctl status wwatcher.service

# View live logs
sudo journalctl -u wwatcher.service -f

# Restart service
sudo systemctl restart wwatcher.service

# Stop service
sudo systemctl stop wwatcher.service
```

### Quick Update and Restart

```bash
cd ~/polymaster && git pull && cargo build --release && cargo install --path . && sudo systemctl restart wwatcher.service
```

## What It Monitors

- Polymarket and Kalshi transactions over your threshold (default $25k)
- Wallet activity and repeat actors
- Unusual trading patterns and anomalies
- Entry and exit positions

## Optional: Configure Kalshi API

For higher rate limits, add Kalshi credentials:

```bash
wwatcher setup
```

## Webhook Integration

Send alerts to automation platforms:

- n8n (self-hosted)
- Zapier
- Make (Integromat)
- Any webhook endpoint

Payload includes: platform, alert_type (WHALE_ENTRY/WHALE_EXIT), action (BUY/SELL), value, price, market details, and wallet activity.

## Example Alert Output

```
[ALERT] LARGE TRANSACTION DETECTED - Polymarket
======================================================================
Market:   Will Trump win the 2024 Presidential Election?
Outcome:  Yes
Value:    $45,250.00
Price:    $0.7500 (75.0%)
Size:     60333.33 contracts
Side:     BUY
Time:     2026-01-08T21:30:00Z

[ANOMALY INDICATORS]
  - High conviction in likely outcome

Asset ID: 65396714035221124737...
======================================================================
```

## Troubleshooting

### Rate limit errors
Increase polling interval:
```bash
wwatcher watch --interval 60
```

### No transactions detected
Lower the threshold:
```bash
wwatcher watch --threshold 10000
```

### Service not starting
Check logs:
```bash
sudo journalctl -u wwatcher.service -n 50
```

### Update service configuration
Edit threshold or interval in service file, then reload:
```bash
sudo systemctl daemon-reload
sudo systemctl restart wwatcher.service
```

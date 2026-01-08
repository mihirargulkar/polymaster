# Whale Watcher

A Rust CLI tool that monitors large transactions on Polymarket and Kalshi prediction markets. Real-time alerts for significant market activity with built-in anomaly detection.

## Features

- **Real-time monitoring** of Polymarket and Kalshi transactions
- **Customizable alerts** for transactions above a threshold (default: $25,000)
- **Anomaly detection** - identifies unusual trading patterns including:
  - Extreme confidence bets (>95% or <5% probability)
  - Contrarian positions on unlikely outcomes
  - Exceptionally large position sizes (>100k contracts)
  - Major capital deployment (>$100k)
  - Possible information asymmetry indicators
- **Persistent configuration** - set up once, no need for exports
- **Professional CLI output** with clear formatting
- **No API keys required** for basic functionality (public data access)
- **Fast and efficient** - built with Rust

## Installation

### From Source

```bash
# Clone or navigate to the project
cd polymaster

# Build the project
cargo build --release

# Install to system (optional)
cargo install --path .
```

The binary will be available at `target/release/whale-watcher` or in your cargo bin directory.

## Quick Start

### 1. Setup (Optional)

Configure API credentials if you want authenticated access to Kalshi (optional):

```bash
whale-watcher setup
```

This will guide you through:
- **Kalshi API credentials** (optional) - Generate at https://kalshi.com/profile/api-keys
- **Polymarket** - No API key needed! Public access only

Your configuration is saved to `~/.config/whale-watcher/config.json` (macOS/Linux) or equivalent on Windows.

### 2. Watch for Whales

Start monitoring with default settings ($25,000 threshold, 5 second polling):

```bash
whale-watcher watch
```

Or customize:

```bash
# Watch for $50k+ trades, check every 30 seconds
whale-watcher watch --threshold 50000 --interval 30

# Watch for $10k+ trades, check every 10 seconds  
whale-watcher watch -t 10000 -i 10
```

### 3. Check Status

View your current configuration:

```bash
whale-watcher status
```

## API Information

### Polymarket

- **Public API**: `https://data-api.polymarket.com`
- **No authentication required** for public trade data
- **Documentation**: https://docs.polymarket.com

The tool uses the Polymarket Data API to fetch recent trades. This is a public endpoint that provides:
- Recent trade activity
- Market data
- Price information

### Kalshi

- **Public API**: `https://api.elections.kalshi.com/trade-api/v2`
- **Authentication**: Optional (public endpoints available)
- **Documentation**: https://docs.kalshi.com

For public trade data, no API key is needed. If you want access to your personal orders and fills, you can:
1. Create an account at https://kalshi.com
2. Generate API credentials at https://kalshi.com/profile/api-keys
3. Run `whale-watcher setup` and enter your credentials

## Alert Example

When a whale transaction is detected, you'll see:

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

## Command Reference

### `whale-watcher watch`

Start monitoring for large transactions.

**Options:**
- `-t, --threshold <AMOUNT>` - Minimum transaction size in USD (default: 25000)
- `-i, --interval <SECONDS>` - Polling interval in seconds (default: 5)

**Examples:**
```bash
whale-watcher watch                        # Default: $25k threshold, 5s interval
whale-watcher watch -t 50000               # $50k threshold
whale-watcher watch -i 30                  # Check every 30 seconds
whale-watcher watch -t 100000 -i 60        # $100k threshold, check every minute
```

### `whale-watcher setup`

Interactive setup wizard to configure API credentials.

```bash
whale-watcher setup
```

### `whale-watcher status`

Show current configuration status.

```bash
whale-watcher status
```

## Configuration File

Configuration is stored at:
- **macOS/Linux**: `~/.config/whale-watcher/config.json`
- **Windows**: `%APPDATA%\whale-watcher\config.json`

Example `config.json`:
```json
{
  "kalshi_api_key_id": "your-key-id",
  "kalshi_private_key": "your-private-key"
}
```

## Development

Built with:
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Tokio](https://tokio.rs/) - Async runtime
- [Reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [Clap](https://github.com/clap-rs/clap) - CLI argument parsing
- [Serde](https://serde.rs/) - JSON serialization

## Troubleshooting

### "No configuration found" warning

This is normal! The tool works without configuration using public APIs. Run `whale-watcher setup` only if you want to add Kalshi authentication.

### API errors

- **Polymarket**: Public endpoint, should work without issues
- **Kalshi**: Public endpoint works without auth, but rate limits may apply

### Rate Limiting

If you're getting rate limited:
- Increase the `--interval` to poll less frequently
- For Kalshi: Add API credentials via `whale-watcher setup`

## Documentation

- [QUICKSTART.md](QUICKSTART.md) - Quick start guide
- [ANOMALY_DETECTION.md](ANOMALY_DETECTION.md) - Detailed anomaly detection patterns and use cases
- [API_REFERENCE.md](API_REFERENCE.md) - API documentation and technical details

## License

This tool is for educational and monitoring purposes. Please review the terms of service for Polymarket and Kalshi APIs.

## Disclaimer

This is a monitoring tool, not a trading bot. It does not execute any trades. Always verify transaction data and do your own research before making investment decisions.

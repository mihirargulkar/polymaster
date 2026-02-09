# Whale Watcher

A Rust CLI tool that monitors large transactions on Polymarket and Kalshi prediction markets. Real-time alerts for significant market activity with built-in anomaly detection.

Repository: https://github.com/neur0map/polymaster

## DISCLAIMER
This tool is for informational and research purposes only. Use this data solely for informed decision-making and market analysis.

## Features

- Real-time monitoring of Polymarket and Kalshi transactions
- Audio alerts with triple beep for repeat actors
- Wallet tracking detects repeated large transactions from same wallet
  - Elevated alerts for repeat actors (2+ txns in 1 hour)
  - High priority alerts for heavy actors (5+ txns in 24 hours)
  - Tracks volume and transaction frequency per wallet
- Customizable alerts for transactions above a threshold (default $25,000)
- Anomaly detection identifies unusual trading patterns:
  - Extreme confidence bets (over 95% or under 5% probability)
  - Contrarian positions on unlikely outcomes
  - Exceptionally large position sizes (over 100k contracts)
  - Major capital deployment (over $100k)
  - Possible information asymmetry indicators
- Webhook notifications send alerts to n8n, Zapier, Make, or any webhook endpoint
- Exit detection with special alerts when whales are selling or exiting positions
- Persistent configuration saves settings between runs
- Clean CLI output with clear formatting
- No API keys required for basic functionality (public data access)
- Fast and efficient, built with Rust

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

The binary will be available at `target/release/wwatcher` or in your cargo bin directory.

## Quick Start

```bash
cargo install --path .
wwatcher watch
```

See [QUICKSTART.md](QUICKSTART.md) for detailed setup instructions and webhook integration.

## API Information

### Polymarket

Public API: https://data-api.polymarket.com

No authentication required for public trade data. The tool uses the Polymarket Data API to fetch:

- Recent trade activity
- Market data
- Price information

Documentation: https://docs.polymarket.com

### Kalshi

Public API: https://api.elections.kalshi.com/trade-api/v2

Authentication is optional. Public endpoints are available without an API key. For access to personal orders and fills:

1. Create an account at https://kalshi.com
2. Generate API credentials at https://kalshi.com/profile/api-keys
3. Run `wwatcher setup` and enter your credentials

Documentation: https://docs.kalshi.com

## DISCLAIMER
 Currently, there is no code in place to view your order or place orders.
 current setup only allows for tracking transactions but I do plan to improve the application down the road with n8n.

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

## Commands

```bash
wwatcher watch              # Start monitoring
wwatcher setup              # Configure API and webhook
wwatcher status             # View configuration
wwatcher history            # View alert history
```

See [QUICKSTART.md](QUICKSTART.md) for detailed command options and examples.

## Configuration

Configuration is stored at `~/.config/wwatcher/config.json` (macOS/Linux) or `%APPDATA%\wwatcher\config.json` (Windows).

Use `wwatcher setup` to configure API credentials and webhook URL. See [QUICKSTART.md](QUICKSTART.md) for webhook integration details.

## Development

Built with:

- Rust - Systems programming language
- Tokio - Async runtime
- Reqwest - HTTP client
- Clap - CLI argument parsing
- Serde - JSON serialization

## Troubleshooting

### No configuration found warning

This is normal. The tool works without configuration using public APIs. Run `wwatcher setup` only if you want to add Kalshi authentication.

### API errors

- Polymarket: Public endpoint should work without issues
- Kalshi: Public endpoint works without auth, but rate limits may apply

### Rate Limiting

If you're getting rate limited:

- Increase the `--interval` to poll less frequently
- For Kalshi: Add API credentials via `wwatcher setup`

## Contributors

Thanks to these contributors for their ideas and improvements:

- [@fuzmik](https://github.com/fuzmik) - Suggested alert history logging feature

## AI Agent Integration

wwatcher includes an AI integration layer that turns whale alerts into actionable research. When you receive a whale alert, the agent investigates using RapidAPI data sources, analyzes the position, and delivers research-backed insights.

### Quick Start

```bash
# Build the CLI
cd integration
npm install
npm run build

# Configure your API key
echo "RAPIDAPI_KEY=your-key-here" > .env

# Test it works
node dist/cli.js status
node dist/cli.js fetch "Bitcoin price above 100k"
```

### CLI Commands

```bash
node dist/cli.js status                           # Health check
node dist/cli.js alerts --limit=10 --min=50000    # Query alerts
node dist/cli.js summary                          # Aggregate stats
node dist/cli.js search "bitcoin"                 # Search alerts
node dist/cli.js fetch "BTC price above 100k"    # Fetch market data
node dist/cli.js fetch "Lakers game" --category=sports
```

### Modular Provider System

Providers are organized by category in `integration/providers/`:

```
providers/
├── crypto.json     # Cryptocurrency APIs (Coinranking)
├── sports.json     # Sports data (NBA API)
├── weather.json    # Weather forecasts (Meteostat)
├── news.json       # News aggregation (Crypto News)
└── README.md       # How to add more providers
```

**Adding new providers**: Create a new JSON file or add to existing category files. See [`integration/providers/README.md`](./integration/providers/README.md) for the full schema and examples.

### OpenClaw Skill Installation

```bash
mkdir -p ~/.openclaw/skills/wwatcher-ai
cp integration/skill/SKILL.md ~/.openclaw/skills/wwatcher-ai/SKILL.md
```

### MCP Server (Claude Code)

```bash
npm run start:mcp
```

Add to your MCP config:
```json
{
  "mcpServers": {
    "wwatcher": {
      "command": "node",
      "args": ["/path/to/integration/dist/index.js"]
    }
  }
}
```

### RapidAPI Setup

Your single API key works for all subscribed services. Subscribe to these (free tiers available):

| Category | API | Link |
|----------|-----|------|
| Crypto | Coinranking | [rapidapi.com/Coinranking/api/coinranking1](https://rapidapi.com/Coinranking/api/coinranking1) |
| Sports | NBA API | [rapidapi.com/api-sports/api/nba-api-free-data](https://rapidapi.com/api-sports/api/nba-api-free-data) |
| Weather | Meteostat | [rapidapi.com/meteostat/api/meteostat](https://rapidapi.com/meteostat/api/meteostat) |
| News | Crypto News | [rapidapi.com/Starter-api/api/cryptocurrency-news2](https://rapidapi.com/Starter-api/api/cryptocurrency-news2) |

### Documentation

- [`integration/providers/README.md`](./integration/providers/README.md) — Adding custom providers
- [`integration/skill/SKILL.md`](./integration/skill/SKILL.md) — OpenClaw skill reference
- [`instructions_for_ai_agent.md`](./instructions_for_ai_agent.md) — Complete agent instructions
- [`integration/README.md`](./integration/README.md) — CLI and MCP server details

## License

This tool is for educational and monitoring purposes. Review the terms of service for Polymarket and Kalshi APIs.


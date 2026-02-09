# wwatcher MCP Integration

MCP (Model Context Protocol) server that exposes wwatcher whale alert data to AI agents. Includes RapidAPI integration for pulling contextual market data (crypto prices, sports odds, weather forecasts, news).

## Quick Start

```bash
cd integration
npm install
npm run build
```

## Configuration

1. Copy `.env.example` to `.env`
2. Add your RapidAPI key: `RAPIDAPI_KEY=your-key`
3. Get your key at https://rapidapi.com

## MCP Server Setup

Add to your MCP client config (e.g., `~/.openclaw/openclaw.json`):

```json
{
  "mcpServers": {
    "wwatcher": {
      "command": "node",
      "args": ["/absolute/path/to/integration/dist/index.js"],
      "env": {
        "RAPIDAPI_KEY": "your-key"
      }
    }
  }
}
```

## Tools

| Tool | Description |
|------|-------------|
| `get_recent_alerts` | Query alert history with filters (limit, platform, alert_type, min_value, since) |
| `get_alert_summary` | Aggregate stats: total volume, breakdown by platform/market/action, top markets |
| `search_alerts` | Text search in market titles and outcomes |
| `fetch_market_data` | Pull contextual data from RapidAPI providers based on market title keywords |
| `get_wwatcher_status` | Health check: history file, alert count, provider status |

## Modes

- `--mode=realtime` (default) — watches alert_history.jsonl for new alerts in real-time
- `--mode=snapshot` — loads existing history only, no live watching

## Providers

Data providers are configured in `providers.json`. Add new providers by editing this file — no code changes needed.

| Provider | Category | Data |
|----------|----------|------|
| Open Weather | weather | 5-day forecasts |
| CoinMarketCap | crypto | Price quotes |
| The Odds API | sports | Game odds |
| Newscatcher | news | Article search |

## For AI Agents

See [`instructions_for_ai_agent.md`](../instructions_for_ai_agent.md) in the repository root for complete agent instructions.

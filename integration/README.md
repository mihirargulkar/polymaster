# wwatcher-ai Integration

AI agent integration for wwatcher whale alert monitoring. Includes:
- **CLI tool** for OpenClaw and shell-based agents
- **MCP server** for Claude Code and MCP-compatible clients
- **RapidAPI integration** for contextual market data (crypto, sports, weather, news)

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

Subscribe to these APIs:
- [Open Weather](https://rapidapi.com/worldapi/api/open-weather13)
- [CoinMarketCap](https://rapidapi.com/coinmarketcap/api/coinmarketcap-api1)
- [The Odds API](https://rapidapi.com/therundown/api/therundown-therundown-v1)
- [Newscatcher](https://rapidapi.com/newscatcher-api-newscatcher-api-default/api/newscatcher)

---

## Option 1: CLI (OpenClaw / Shell)

For OpenClaw agents or any shell-based automation.

### Commands

```bash
# Health check
node dist/cli.js status

# Query alerts
node dist/cli.js alerts --limit=10 --min=50000
node dist/cli.js alerts --platform=polymarket --type=WHALE_ENTRY

# Aggregate stats
node dist/cli.js summary

# Search alerts
node dist/cli.js search "bitcoin"

# Fetch market data from RapidAPI
node dist/cli.js fetch "Bitcoin price above 100k"
node dist/cli.js fetch "Lakers vs Celtics" --category=sports
```

### CLI Options

**alerts:**
| Option | Description |
|--------|-------------|
| `--limit=N` | Max alerts to return (default: 20) |
| `--platform=X` | Filter: polymarket, kalshi |
| `--type=X` | Filter: WHALE_ENTRY, WHALE_EXIT |
| `--min=N` | Minimum USD value |
| `--since=ISO` | Alerts after timestamp |

**fetch:**
| Option | Description |
|--------|-------------|
| `--category=X` | Override: weather, crypto, sports, news |

### OpenClaw Skill Installation

```bash
mkdir -p ~/.openclaw/skills/wwatcher-ai
cp skill/SKILL.md ~/.openclaw/skills/wwatcher-ai/SKILL.md
```

---

## Option 2: MCP Server (Claude Code)

For MCP-compatible clients like Claude Code.

### Setup

Add to your MCP client config:

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

### Start MCP Server

```bash
npm run start:mcp
# or
node dist/index.js
```

### MCP Tools

| Tool | Description |
|------|-------------|
| `get_recent_alerts` | Query alert history with filters |
| `get_alert_summary` | Aggregate stats: volume, top markets, whale counts |
| `search_alerts` | Text search in market titles/outcomes |
| `fetch_market_data` | Pull RapidAPI data based on market keywords |
| `get_wwatcher_status` | Health check |

### Modes

- `--mode=realtime` (default) — watches for new alerts in real-time
- `--mode=snapshot` — loads existing history only

---

## Providers

Data providers are configured in `providers.json`. Add new providers by editing this file — no code changes needed.

| Provider | Category | Data |
|----------|----------|------|
| Open Weather | weather | 5-day forecasts |
| CoinMarketCap | crypto | Price quotes |
| The Odds API | sports | Game odds |
| Newscatcher | news | Article search |

### Adding a Provider

```json
{
  "your_provider": {
    "name": "Provider Name",
    "category": "category",
    "rapidapi_host": "api-host.p.rapidapi.com",
    "env_key": "RAPIDAPI_KEY",
    "keywords": ["keyword1", "keyword2"],
    "endpoints": {
      "main": {
        "method": "GET",
        "path": "/v1/endpoint",
        "description": "What it does",
        "params": {}
      }
    }
  }
}
```

---

## For AI Agents

See [`instructions_for_ai_agent.md`](../instructions_for_ai_agent.md) for complete agent instructions including:
- Research workflows
- Analysis output format
- Category-specific guidance
- Pattern detection

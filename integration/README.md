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

Subscribe to these APIs (free tiers available):

| Category | API | Link |
|----------|-----|------|
| Crypto | Coinranking | [rapidapi.com/Coinranking/api/coinranking1](https://rapidapi.com/Coinranking/api/coinranking1) |
| Sports | NBA API | [rapidapi.com/api-sports/api/nba-api-free-data](https://rapidapi.com/api-sports/api/nba-api-free-data) |
| Weather | Meteostat | [rapidapi.com/meteostat/api/meteostat](https://rapidapi.com/meteostat/api/meteostat) |
| News | Crypto News | [rapidapi.com/Starter-api/api/cryptocurrency-news2](https://rapidapi.com/Starter-api/api/cryptocurrency-news2) |

Your single API key works for all subscribed services.

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

Providers are organized by category in the `providers/` directory:

```
providers/
├── README.md       # Full documentation for adding providers
├── crypto.json     # Coinranking (BTC, ETH, SOL prices)
├── sports.json     # NBA API (games, scores)
├── weather.json    # Meteostat (forecasts)
└── news.json       # Cryptocurrency News
```

### Adding a Provider

**Option 1**: Add to an existing category file (e.g., `providers/crypto.json`)

**Option 2**: Create a new category file (e.g., `providers/politics.json`)

```json
{
  "provider_key": {
    "name": "Display Name",
    "category": "politics",
    "rapidapi_host": "api.p.rapidapi.com",
    "env_key": "RAPIDAPI_KEY",
    "keywords": ["election", "president", "vote"],
    "endpoints": {
      "markets": {
        "method": "GET",
        "path": "/v1/markets",
        "description": "What it returns",
        "params": {}
      }
    }
  }
}
```

The system automatically loads all `*.json` files from the `providers/` directory.

See [`providers/README.md`](./providers/README.md) for the complete schema and examples.

---

## For AI Agents

See [`instructions_for_ai_agent.md`](../instructions_for_ai_agent.md) for complete agent instructions including:
- Research workflows
- Analysis output format
- Category-specific guidance
- Pattern detection

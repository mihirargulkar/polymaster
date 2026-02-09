# Instructions for AI Agent (OpenClaw)

> This file is the master entry point for OpenClaw agents working with the wwatcher project.

## 1. What This Project Is

**wwatcher** (Whale Watcher) is a Rust CLI tool that monitors large transactions ("whale bets") on prediction markets:

- **Polymarket** — crypto-native prediction market (public API, no auth required)
- **Kalshi** — regulated US prediction market (optional API auth for personal data)

Alerts are logged to `~/.config/wwatcher/alert_history.jsonl` in JSONL format. Each line is a JSON object with this schema:

```json
{
  "platform": "Polymarket" | "Kalshi",
  "alert_type": "WHALE_ENTRY" | "WHALE_EXIT",
  "action": "BUY" | "SELL",
  "value": 45250.00,
  "price": 0.75,
  "price_percent": 75,
  "size": 60333.33,
  "timestamp": "2026-01-08T21:30:00Z",
  "market_title": "Will Trump win the 2024 Presidential Election?",
  "outcome": "Yes",
  "wallet_id": "0xabc...",
  "wallet_activity": {
    "transactions_last_hour": 3,
    "transactions_last_day": 7,
    "total_value_hour": 125000.0,
    "total_value_day": 450000.0,
    "is_repeat_actor": true,
    "is_heavy_actor": true
  }
}
```

## 2. Your Role as AI Agent

You are an autonomous research and analysis agent. When whale alerts fire, you:

1. **Monitor alerts** by periodically running the CLI to check for new entries
2. **Pull contextual data** from RapidAPI providers (crypto prices, sports odds, weather, news)
3. **Perform actual research and estimation** — don't just return raw data
4. **Produce analysis** comparing the whale's position with external data

### Example Workflow

A sports alert fires: "Whale bought $50K YES on NBA game Lakers vs Celtics"

1. Run `wwatcher-ai alerts --limit=5` to see the full alert details
2. Run `wwatcher-ai fetch "Lakers vs Celtics" --category=sports` → gets current odds
3. Research: odds show Lakers +3.5, whale bet YES on Lakers win at 42% implied probability
4. Pull injury reports, recent form, head-to-head record (via news or your knowledge)
5. Present analysis:
   - "Whale is betting against the spread consensus. Lakers are underdogs at +3.5"
   - "Key factor: LeBron returned from injury 2 games ago, averaging 28pts since return"
   - "Estimation: 45-50% win probability for Lakers, market at 42% — whale may see slight edge"
   - "Confidence: Medium — the edge is thin but whale is a repeat actor with 7 txns today"

## 3. Setup Instructions (OpenClaw)

### Step 1: Build the CLI

```bash
cd /home/neur0map/polymaster-test/integration
npm install
npm run build
```

### Step 2: Configure Environment

Edit `/home/neur0map/polymaster-test/integration/.env`:
```
RAPIDAPI_KEY=your-rapidapi-key-here
```

Get your key at https://rapidapi.com. Subscribe to:
- Open Weather: https://rapidapi.com/worldapi/api/open-weather13
- CoinMarketCap: https://rapidapi.com/coinmarketcap/api/coinmarketcap-api1
- The Odds API: https://rapidapi.com/therundown/api/therundown-therundown-v1
- Newscatcher: https://rapidapi.com/newscatcher-api-newscatcher-api-default/api/newscatcher

### Step 3: Install Skill

```bash
mkdir -p ~/.openclaw/skills/wwatcher-ai
cp /home/neur0map/polymaster-test/integration/skill/SKILL.md ~/.openclaw/skills/wwatcher-ai/SKILL.md
```

### Step 4: Verify

```bash
cd /home/neur0map/polymaster-test/integration && node dist/cli.js status
```

You should see:
- `status: "running"`
- `history_file.exists: true` (if wwatcher has been run before)
- `providers.count: 4` (weather, crypto, sports, news)
- `api_key_configured: true` (if RAPIDAPI_KEY is set)

## 4. CLI Commands

### `status` — Health Check
```bash
cd /home/neur0map/polymaster-test/integration && node dist/cli.js status
```

### `alerts` — Query Alerts
```bash
cd /home/neur0map/polymaster-test/integration && node dist/cli.js alerts --limit=10 --min=50000
cd /home/neur0map/polymaster-test/integration && node dist/cli.js alerts --platform=polymarket --type=WHALE_ENTRY
```

Options:
- `--limit=N` — Max alerts (default: 20)
- `--platform=X` — polymarket or kalshi
- `--type=X` — WHALE_ENTRY or WHALE_EXIT
- `--min=N` — Min USD value
- `--since=ISO` — Alerts after timestamp

### `summary` — Aggregate Stats
```bash
cd /home/neur0map/polymaster-test/integration && node dist/cli.js summary
```

### `search` — Text Search
```bash
cd /home/neur0map/polymaster-test/integration && node dist/cli.js search "bitcoin"
```

### `fetch` — Get Market Data
```bash
cd /home/neur0map/polymaster-test/integration && node dist/cli.js fetch "Bitcoin price above 100k"
cd /home/neur0map/polymaster-test/integration && node dist/cli.js fetch "Lakers vs Celtics" --category=sports
```

## 5. File Locations

| Item | Path |
|------|------|
| Alert history | `~/.config/wwatcher/alert_history.jsonl` |
| RapidAPI key | `/home/neur0map/polymaster-test/integration/.env` |
| Providers config | `/home/neur0map/polymaster-test/integration/providers.json` |
| CLI | `/home/neur0map/polymaster-test/integration/dist/cli.js` |
| Skill | `~/.openclaw/skills/wwatcher-ai/SKILL.md` |

## 6. Research & Estimation Workflow

### Crypto Alerts
1. `wwatcher-ai fetch "BTC..."` → CoinMarketCap price
2. Compare whale's entry price (from `price_percent`) to current market
3. Estimate if whale is ahead/behind, assess momentum vs contrarian

### Sports Alerts
1. `wwatcher-ai fetch "Team A vs Team B" --category=sports` → odds
2. Cross-reference whale's bet side with consensus
3. Research injuries, form, head-to-head

### Weather Alerts
1. `wwatcher-ai fetch "City high temp..." --category=weather` → forecast
2. Compare threshold in market to forecast range
3. Assess forecast confidence

### Political/News Alerts
1. `wwatcher-ai fetch "Market title" --category=news` → articles
2. Look for recent events affecting probability
3. Identify if whale sees something market hasn't priced

## 7. Adding New RapidAPI Providers

Edit `/home/neur0map/polymaster-test/integration/providers.json`:

```json
{
  "your_provider": {
    "name": "Provider Name",
    "category": "your_category",
    "rapidapi_host": "provider-host.p.rapidapi.com",
    "env_key": "RAPIDAPI_KEY",
    "keywords": ["keyword1", "keyword2"],
    "endpoints": {
      "endpoint_name": {
        "method": "GET",
        "path": "/v1/your/endpoint/{param}",
        "description": "What this endpoint does",
        "params": {
          "param": { "type": "string", "required": true, "description": "Parameter description" }
        }
      }
    }
  }
}
```

No code changes needed — the CLI reads providers.json dynamically.

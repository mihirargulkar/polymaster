# Instructions for AI Agent

> This file is the master entry point for any AI agent (OpenClaw, Claude, etc.) working with the wwatcher project.

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

Market categories detected by wwatcher (from Kalshi ticker parsing):
- **Crypto/Stocks**: BTC, ETH, SOL, SPX, TSLA price thresholds
- **Sports**: NFL, NBA, NHL, MLB, Soccer game outcomes, totals, spreads, player props
- **Weather**: High/low temperature thresholds for cities
- **Political**: Presidential, election markets
- **General**: Any other prediction market

## 2. Your Role as AI Agent

You are an autonomous research and analysis agent. When whale alerts fire, you:

1. **Monitor alerts** in real-time via the MCP server tools
2. **Pull contextual data** from RapidAPI providers (crypto prices, sports odds, weather, news)
3. **Perform actual research and estimation** — don't just return raw data
4. **Produce analysis** comparing the whale's position with external data

### Example Workflow

A sports alert fires: "Whale bought $50K YES on NBA game Lakers vs Celtics"

1. Call `get_recent_alerts` to see the full alert details
2. Call `fetch_market_data` with the market title → gets current odds from The Odds API
3. Research: odds show Lakers +3.5, whale bet YES on Lakers win at 42% implied probability
4. Pull injury reports, recent form, head-to-head record
5. Present analysis:
   - "Whale is betting against the spread consensus. Lakers are underdogs at +3.5"
   - "Key factor: LeBron returned from injury 2 games ago, averaging 28pts since return"
   - "Estimation: 45-50% win probability for Lakers, market at 42% — whale may see slight edge"
   - "Confidence: Medium — the edge is thin but whale is a repeat actor with 7 txns today"

## 3. Setup Instructions

Follow these steps to set up the MCP integration:

### Step 1: Build the MCP Server

```bash
cd integration
npm install
npm run build
```

### Step 2: Configure Environment

Copy the example env file and add your RapidAPI key:

```bash
cp integration/.env.example integration/.env
```

Edit `integration/.env`:
```
RAPIDAPI_KEY=your-rapidapi-key-here
```

Tell the user: "Go to https://rapidapi.com, create an account, subscribe to the APIs you want (Open Weather, CoinMarketCap, The Odds API, Newscatcher), then copy your API key and paste it into `integration/.env`"

### Step 3: Configure MCP Server

Add this to `~/.openclaw/openclaw.json` (or the relevant MCP client config):

```json
{
  "mcpServers": {
    "wwatcher": {
      "command": "node",
      "args": ["/path/to/polymaster/integration/dist/index.js"],
      "env": {
        "RAPIDAPI_KEY": "your-rapidapi-key"
      }
    }
  }
}
```

Replace `/path/to/polymaster` with the actual path to the cloned repository.

### Step 4: Install Skill (OpenClaw)

```bash
mkdir -p ~/.openclaw/skills/wwatcher-ai
cp integration/skill/SKILL.md ~/.openclaw/skills/wwatcher-ai/SKILL.md
```

### Step 5: Verify

Call the `get_wwatcher_status` tool. You should see:
- `status: "running"`
- `history_file.exists: true` (if wwatcher has been run before)
- `providers.count: 4` (weather, crypto, sports, news)
- `api_key_configured: true` (if RAPIDAPI_KEY is set)

## 4. File Locations for Sensitive Keys

| Key | File | Format |
|-----|------|--------|
| RapidAPI key | `integration/.env` | `RAPIDAPI_KEY=abc123` |
| Kalshi API | `~/.config/wwatcher/config.json` | JSON: `kalshi_api_key_id`, `kalshi_private_key` |
| OpenClaw env | `~/.openclaw/.env` | `RAPIDAPI_KEY=abc123` |
| MCP server env | `~/.openclaw/openclaw.json` | JSON: `mcpServers.wwatcher.env` |

**Never commit API keys.** The `integration/.env` file is gitignored.

## 5. How to Use RapidAPI Providers

The file `integration/providers.json` defines all available data sources. Each provider has:

- **rapidapi_host** — the RapidAPI host header
- **endpoints** — API endpoints with path, method, and parameter definitions
- **keywords** — terms used to auto-match market titles to providers

### Available Providers

| Provider | Category | What It Provides |
|----------|----------|-----------------|
| Open Weather | weather | 5-day forecasts by lat/lon |
| CoinMarketCap | crypto | Latest cryptocurrency price quotes |
| The Odds API | sports | Current odds for upcoming games |
| Newscatcher | news | News article search (matches all alerts) |

### Using the `fetch_market_data` Tool

```
fetch_market_data({ market_title: "Bitcoin price above $100k" })
→ Auto-matches "crypto" provider → calls CoinMarketCap latest_quotes with symbol=BTC

fetch_market_data({ market_title: "Lakers vs Celtics", category: "sports" })
→ Matches "sports" provider → calls The Odds API with sport=basketball_nba

fetch_market_data({ market_title: "NYC high temp above 63°F" })
→ Auto-matches "weather" provider → calls Open Weather forecast
```

### Direct API Calls (curl examples)

**Crypto:**
```bash
curl -H "X-RapidAPI-Key: YOUR_KEY" -H "X-RapidAPI-Host: coinmarketcap-api1.p.rapidapi.com" \
  "https://coinmarketcap-api1.p.rapidapi.com/v1/cryptocurrency/quotes/latest?symbol=BTC"
```

**Sports:**
```bash
curl -H "X-RapidAPI-Key: YOUR_KEY" -H "X-RapidAPI-Host: therundown-therundown-v1.p.rapidapi.com" \
  "https://therundown-therundown-v1.p.rapidapi.com/sports/basketball_nba/odds"
```

**Weather:**
```bash
curl -H "X-RapidAPI-Key: YOUR_KEY" -H "X-RapidAPI-Host: open-weather13.p.rapidapi.com" \
  "https://open-weather13.p.rapidapi.com/city/fivedaysforcast/40.7128/-74.0060/EN"
```

**News:**
```bash
curl -H "X-RapidAPI-Key: YOUR_KEY" -H "X-RapidAPI-Host: newscatcher.p.rapidapi.com" \
  "https://newscatcher.p.rapidapi.com/v2/search?q=bitcoin&lang=en"
```

## 6. Research & Estimation Workflow

When an alert fires, follow the appropriate workflow based on market category:

### Crypto Alerts
1. Pull current price + 24h trend + volume via `fetch_market_data`
2. Compare whale's entry price (from `price_percent`) to current market price
3. Estimate if whale is ahead/behind on their position
4. Assess market sentiment: is this a contrarian bet or riding momentum?
5. Flag: large position size, repeat actor status, exit signals

### Sports Alerts
1. Pull odds from `fetch_market_data` → The Odds API
2. If outdoor game, also pull weather data
3. Cross-reference whale's bet side with consensus odds
4. Look for: injury news, recent form, head-to-head records
5. Estimate probability and compare with market implied probability
6. Flag: whale betting against consensus, multiple whales same game

### Weather Alerts
1. Pull multi-day forecast via `fetch_market_data` → Open Weather
2. Compare the threshold in the market (e.g., "high temp ≥ 63°F") with forecast
3. Estimate probability based on forecast confidence
4. Flag: forecast uncertainty, upcoming weather pattern changes

### Political Alerts
1. Pull recent news via `fetch_market_data` with category "news"
2. Assess current probability vs market price
3. Look for: polling data, recent events, policy announcements
4. Identify if whale sees something the market hasn't priced in
5. Flag: heavy actor exits, contrarian bets on unlikely outcomes

### General / Unknown Category
1. Pull news articles related to the market title
2. Perform sentiment analysis on recent coverage
3. Present context: what is this market about, what are the key factors
4. Flag: any unusual patterns (timing, size, repeat actors)

## 7. Adding New RapidAPI Providers

To add a new data source, edit `integration/providers.json`:

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

**No code changes needed** — the fetcher reads providers.json dynamically at startup.

Tell the user which RapidAPI subscription is needed for the new provider.

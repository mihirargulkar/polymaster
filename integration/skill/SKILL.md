# wwatcher-ai — Whale Alert Research Agent

## Trigger

**When you see a message containing any of these patterns:**
- "LARGE TRANSACTION DETECTED"
- "WHALE ALERT"
- "Research this whale alert"
- JSON with `"alert_type": "WHALE_ENTRY"` or `"alert_type": "WHALE_EXIT"`

**When triggered:**
1. Parse the alert details from the message
2. Check user preferences (skip silently if filtered out)
3. Score the alert
4. Run context-aware research
5. Reply with a structured signal

---

## Quick Reference

```bash
cd ~/polymaster/integration

# Context-aware research (use this for whale alerts)
node dist/cli.js research "Market title" --context='<full_alert_json>'

# Score an alert
node dist/cli.js score '<alert_json>'

# Legacy research (no context)
node dist/cli.js research "Market title" --category=crypto

# Other commands
node dist/cli.js status                    # Health check
node dist/cli.js alerts --limit=5          # Recent alerts
node dist/cli.js fetch "query"             # RapidAPI only
node dist/cli.js perplexity "query"        # Single search
node dist/cli.js preferences show          # Preferences schema
```

---

## User Preferences

The user can set alert filters at any time using natural language.
Store preferences in memory under key `wwatcher_preferences`.

**When a whale alert arrives:**
1. Load preferences from memory key `wwatcher_preferences`
2. Check each filter against the alert data
3. If any filter fails, silently skip (do not respond)
4. If all filters pass, proceed with research

**When the user updates preferences**, confirm the change and show current active filters.

**Preference schema** (all fields optional):
```json
{
  "min_value": 100000,
  "min_win_rate": 0.6,
  "max_leaderboard_rank": 100,
  "platforms": ["polymarket"],
  "categories": ["crypto", "politics"],
  "directions": ["buy"],
  "tier_filter": "high"
}
```

**Natural language examples:**
- "Only alert me on whales with 60%+ win rate" → `{ "min_win_rate": 0.6 }`
- "Skip anything under $100k" → `{ "min_value": 100000 }`
- "Only crypto and politics markets" → `{ "categories": ["crypto", "politics"] }`
- "Top 100 leaderboard traders only" → `{ "max_leaderboard_rank": 100 }`
- "Polymarket only" → `{ "platforms": ["polymarket"] }`
- "Only high tier alerts" → `{ "tier_filter": "high" }`
- "Show me my current filters" → read memory, list active preferences
- "Reset my filters" → clear `wwatcher_preferences` from memory

---

## Research Workflow

### Step 1: Parse the Alert

From the incoming message, extract the full alert JSON including:
- **Core**: platform, action, value, price, market_title, outcome, timestamp
- **Wallet**: wallet_id, wallet_activity (repeat/heavy actor, txn counts)
- **Whale Profile**: leaderboard_rank, win_rate, portfolio_value, positions_count
- **Market Context**: yes_price, no_price, spread, volume_24h, open_interest, tags
- **Order Book**: best_bid, best_ask, bid_depth_10pct, ask_depth_10pct
- **Top Holders**: top 5 holders with shares and percentages

### Step 2: Check Preferences

Load `wwatcher_preferences` from memory. If preferences exist, check:
- `min_value`: alert.value >= min_value
- `min_win_rate`: alert.whale_profile.win_rate >= min_win_rate
- `max_leaderboard_rank`: alert.whale_profile.leaderboard_rank <= max_leaderboard_rank
- `platforms`: alert.platform in platforms list
- `categories`: any alert tag matches categories list
- `directions`: alert.action matches directions list

If any check fails → **silently skip** (do not respond).

### Step 3: Score + Research

```bash
node dist/cli.js research "Market title" --context='<full_alert_json>'
```

This single command:
1. Scores the alert (tier: high/medium/low, factors list)
2. If `tier_filter` preference is set and tier is below threshold → skip
3. Generates 3 targeted Perplexity queries based on the score
4. Fetches prediction market data (related markets, cross-platform match)
5. Fetches RapidAPI data if relevant providers match
6. Returns a structured signal

### Step 4: Deliver Signal

Format the response as:

```
WHALE SIGNAL: [market_title]

Direction: [BULLISH/BEARISH] | Confidence: [HIGH/MEDIUM/LOW]

Key Factors:
- [factor 1]
- [factor 2]
- [factor 3]

Whale: [whale_quality summary]
Book: [market_pressure summary]

Research: [2-3 sentence research_summary]

Cross-Platform: [if cross_platform match found, show title + price]
Related Markets: [if related markets found, list top 2-3]
```

---

## Scoring Reference

| Factor | Signal | Score |
|--------|--------|-------|
| Leaderboard top 10 | Elite trader | +30 |
| Leaderboard top 50 | Strong trader | +25 |
| Leaderboard top 100 | Known trader | +20 |
| Leaderboard top 500 | Ranked trader | +10 |
| Win rate >= 80% | Elite accuracy | +20 |
| Win rate >= 70% | Strong accuracy | +15 |
| Win rate >= 60% | Above average | +10 |
| Heavy actor (5+ txns/24h) | High conviction | +15 |
| Repeat actor (2+ txns/1h) | Active trader | +10 |
| Trade >= $250k | Massive bet | +20 |
| Trade >= $100k | Large bet | +15 |
| Trade >= $50k | Significant bet | +10 |
| Bid imbalance >= 65% | Directional pressure | +10 |
| Contrarian position | Against consensus | +15 |
| Portfolio >= $1M | Whale portfolio | +10 |

**Tier thresholds:**
- **High**: score >= 60
- **Medium**: score >= 35
- **Low**: score < 35

---

## Category Guide

| Category | RapidAPI Data | Research Focus |
|----------|---------------|----------------|
| crypto | Coinranking prices | On-chain data, institutional flows, technicals |
| sports | Game data | Injuries, odds movement, matchups |
| weather | Meteostat forecast | Model confidence, patterns |
| politics | — | Polls, demographics, developments |
| prediction-markets | Polymarket/Kalshi | Related markets, cross-platform, price history |

---

## API Keys

Required in `integration/.env`:
```
PERPLEXITY_API_KEY=xxx
```

Optional (for RapidAPI data enrichment):
```
RAPIDAPI_KEY=xxx
```

Prediction market data (Polymarket/Kalshi) requires no API keys.

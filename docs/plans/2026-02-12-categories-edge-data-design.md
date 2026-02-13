# Categories + Edge Data + Storage + Wallet Memory Design

**Date**: 2026-02-12
**Status**: Implemented (Phases 1-6 complete, Phase 7 in progress)
**Scope**: Market categories, edge data enrichment, exit detection fixes, SQLite storage, 12h wallet memory, setup wizard overhaul

---

## 1. Market Category System

### Unified Category Model

A category/subcategory system that maps across both Kalshi and Polymarket. Users select what to watch; alerts filter accordingly.

Every category has an "All" option (selected by default). A global "All Categories" option watches everything (current behavior).

```
Categories:
  [x] All Categories          <- default, no filtering
  [ ] Sports
      [x] All
      [ ] NBA
      [ ] NFL
      [ ] MLB
      [ ] NHL
      [ ] Soccer
      [ ] Golf
      [ ] MMA/UFC
      [ ] Tennis
      [ ] College Football
      [ ] College Basketball
  [ ] Politics
      [x] All
      [ ] US Elections
      [ ] Congress/Legislation
      [ ] Policy/Regulation
      [ ] International Politics
  [ ] Economics
      [x] All
      [ ] Fed/Interest Rates
      [ ] Inflation/CPI
      [ ] Jobs/Unemployment
      [ ] GDP
      [ ] Recession
  [ ] Crypto
      [x] All
      [ ] Bitcoin
      [ ] Ethereum
      [ ] Altcoins
      [ ] Regulation
  [ ] Finance
      [x] All
      [ ] S&P 500
      [ ] NASDAQ
      [ ] Commodities
      [ ] Forex
      [ ] Individual Stocks
  [ ] Weather/Climate
      [x] All
      [ ] Temperature
      [ ] Storms/Hurricanes
      [ ] Natural Disasters
  [ ] Tech
      [x] All
      [ ] AI/ML
      [ ] Product Launches
      [ ] Company Events
  [ ] Culture
      [x] All
      [ ] Entertainment/Awards
      [ ] Social Media
      [ ] Celebrity
  [ ] World Events
      [x] All
      [ ] Geopolitics
      [ ] Conflicts
      [ ] Treaties/Agreements
  [ ] Health
      [x] All
      [ ] Pandemics
      [ ] FDA/Drug Approvals
      [ ] Public Health
```

### Config Format

```json
{
  "categories": ["all"]
}
```

Examples:
- `["all"]` — watch everything (default)
- `["sports:all", "crypto:all"]` — all sports + all crypto
- `["sports:nba", "sports:nfl", "politics:us_elections"]` — specific subcategories

### Platform Mapping

**Kalshi**: Uses `GET /search/tags_by_categories` to get category-tag mappings. Filter via `GET /series?category={category}` then match market tickers against active series.

**Polymarket**: Uses `GET /tags` for tag IDs. Filter via `GET /events?tag_id={id}` or client-side keyword matching on trade data.

### Hybrid Filtering Strategy

- **Kalshi (server-side)**: Use series tickers grouped by category. When polling `/markets/trades`, filter by series tickers matching user's selected categories.
- **Polymarket (client-side)**: The `/trades` endpoint doesn't support tag filtering. Match market titles against a keyword map.

### Keyword Map (Polymarket client-side matching)

Stored as a configurable data structure. Ships with sensible defaults, users can extend.

```rust
"sports:nba" -> ["NBA", "basketball", "Lakers", "Celtics", "Warriors", "Bucks", ...]
"sports:nfl" -> ["NFL", "football", "Super Bowl", "Chiefs", "Eagles", "Patriots", ...]
"crypto:bitcoin" -> ["Bitcoin", "BTC", "bitcoin price", "satoshi", ...]
"politics:us_elections" -> ["President", "election", "electoral", "White House", "nominee", ...]
"economics:fed" -> ["Fed", "interest rate", "FOMC", "Federal Reserve", "rate cut", "rate hike", ...]
"weather:temperature" -> ["temperature", "high temp", "low temp", "degrees", "heat", "cold", ...]
```

Case-insensitive substring matching on market title.

---

## 2. Edge Data Enrichment

### What We Pull

When a whale trade crosses the threshold, make 1 additional API call to get market context:

| Data Point | Polymarket Source | Kalshi Source | Why It Matters |
|-----------|------------------|---------------|----------------|
| Yes/No price | `outcomePrices` via Gamma API | `yes_bid`, `no_bid` via `/markets/{ticker}` | Current market odds |
| Bid-ask spread | `spread`, `bestBid/Ask` | `yes_bid` vs `yes_ask` | Liquidity quality |
| 24h volume | `volume24hr` | `volume_24h` | Activity level, spike detection |
| Open interest | `openInterest` | `open_interest` | Total money at stake |
| 24h price change | `oneDayPriceChange` | Compute from `last_price` vs `previous_price` | Direction of market movement |
| Liquidity | `liquidityClob` | `liquidity_dollars` | How much can move without slippage |

### API Calls

- **Polymarket**: `GET https://gamma-api.polymarket.com/markets/{slug_or_condition_id}` — returns all fields in one call
- **Kalshi**: `GET https://api.elections.kalshi.com/trade-api/v2/markets/{ticker}` — returns all fields in one call

One extra request per whale alert, not per poll cycle. Negligible overhead.

### New Rust Struct

```rust
pub struct MarketContext {
    pub yes_price: f64,
    pub no_price: f64,
    pub spread: f64,
    pub volume_24h: f64,
    pub open_interest: f64,
    pub price_change_24h: f64,
    pub liquidity: f64,
}
```

### Display Output (Terminal)

After the existing alert, append:

```
[MARKET CONTEXT]
Odds:        YES 65.0% | NO 35.0%
Spread:      $0.01 (tight)
24h Volume:  $450,000 (5.2x weekly avg)
Open Interest: $2,100,000
24h Move:    +3.2%
Liquidity:   $180,000
```

---

## 3. Exit Detection Fixes

### Current Bugs

**Bug 1 — Kalshi (critical)**: `taker_side` returns `"yes"` or `"no"`, never `"sell"`. The check `side.to_uppercase() == "SELL"` always returns false. Exit detection is completely dead on Kalshi.

**Bug 2 — Polymarket (partial)**: Direct SELL trades are detectable, but whales can exit a YES position by buying NO tokens (shows as BUY), making ~50% of exits invisible.

### Fix: Best-Effort Detection

#### Polymarket

1. **Direct sells**: Continue detecting `side == "SELL"` (works today, just rare).
2. **Inferred exits**: Use `wallet_memory` SQLite table (see Section 6) to check if the same wallet previously bought the **opposite outcome** in the same market. Flag as "PROBABLE EXIT".

Alert header for inferred exit:
```
[PROBABLE EXIT] Whale previously bought YES ($75k) — now buying NO ($50k) — Polymarket
```

#### Kalshi

Accept the limitation. Kalshi's public trade API doesn't expose buy/sell direction, only which side (yes/no) the taker is on. Document this clearly.

Remove the dead `is_sell` check for Kalshi. Instead, just report the taker side accurately:
- `taker_side: "yes"` -> "Taking YES position"
- `taker_side: "no"` -> "Taking NO position"

No fake exit detection.

---

## 4. Updated Webhook Payload

Full payload with new `category`, `subcategory`, and `market_context` fields:

```json
{
  "platform": "Polymarket",
  "alert_type": "WHALE_ENTRY",
  "action": "BUY",
  "category": "crypto",
  "subcategory": "bitcoin",
  "value": 50000.0,
  "price": 0.65,
  "price_percent": 65,
  "size": 76923.08,
  "timestamp": "2026-02-12T18:00:00Z",
  "market_title": "Bitcoin above 100k by March?",
  "outcome": "Yes",
  "wallet_id": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD",
  "wallet_activity": {
    "transactions_last_hour": 3,
    "transactions_last_day": 5,
    "total_value_hour": 150000.0,
    "total_value_day": 380000.0,
    "is_repeat_actor": true,
    "is_heavy_actor": true
  },
  "market_context": {
    "yes_price": 0.65,
    "no_price": 0.35,
    "spread": 0.01,
    "volume_24h": 450000.0,
    "open_interest": 2100000.0,
    "price_change_24h": 3.2,
    "liquidity": 180000.0
  }
}
```

For returning whale alerts:
```json
{
  "alert_type": "RETURNING_WHALE",
  "whale_memory": {
    "scenario": "doubling_down",
    "previous_positions": [
      {
        "market_title": "Bitcoin above 100k by March?",
        "outcome": "Yes",
        "value": 50000.0,
        "hours_ago": 3.2
      }
    ],
    "total_12h_volume": 180000.0,
    "total_12h_transactions": 4
  }
}
```

For inferred exits:
```json
{
  "alert_type": "PROBABLE_EXIT",
  "action": "BUY",
  "inferred_exit": {
    "previous_outcome": "Yes",
    "previous_value": 75000.0,
    "current_outcome": "No",
    "current_value": 50000.0
  }
}
```

Backward-compatible — all new fields are additive.

---

## 5. Local SQLite Database

### Why SQLite (replaces JSONL)

Current problems with `alert_history.jsonl`:
- Append-only, grows forever, no size limit
- `show_alert_history()` reads entire file into memory via `read_to_string()`
- No indexing — wallet lookups require scanning entire file
- No persistence for wallet tracker (lost on restart)
- No structured queries

### Database Location

`~/.config/wwatcher/wwatcher.db` (single file, same directory as current config)

### Schema

```sql
-- Alert history (replaces alert_history.jsonl)
CREATE TABLE alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    platform TEXT NOT NULL,
    alert_type TEXT NOT NULL,
    action TEXT NOT NULL,
    category TEXT,
    subcategory TEXT,
    value REAL NOT NULL,
    price REAL NOT NULL,
    size REAL NOT NULL,
    market_title TEXT,
    market_id TEXT,
    outcome TEXT,
    wallet_hash TEXT,
    wallet_id TEXT,
    timestamp TEXT NOT NULL,
    market_context TEXT,       -- JSON blob
    wallet_activity TEXT,      -- JSON blob
    created_at INTEGER DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX idx_alerts_wallet_hash ON alerts(wallet_hash);
CREATE INDEX idx_alerts_timestamp ON alerts(created_at);
CREATE INDEX idx_alerts_category ON alerts(category);
CREATE INDEX idx_alerts_platform ON alerts(platform);

-- Wallet short-term memory (12h rolling window)
CREATE TABLE wallet_memory (
    wallet_hash TEXT NOT NULL,
    wallet_id TEXT NOT NULL,
    market_title TEXT,
    market_id TEXT,
    outcome TEXT,
    action TEXT,
    value REAL NOT NULL,
    price REAL NOT NULL,
    platform TEXT NOT NULL,
    category TEXT,
    seen_at INTEGER NOT NULL,
    PRIMARY KEY (wallet_hash, market_id, seen_at)
);

CREATE INDEX idx_wallet_memory_hash ON wallet_memory(wallet_hash);
CREATE INDEX idx_wallet_memory_seen ON wallet_memory(seen_at);

-- App metadata (schema version, last cleanup, etc.)
CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT
);

-- Initial metadata
INSERT INTO metadata (key, value) VALUES ('schema_version', '1');
INSERT INTO metadata (key, value) VALUES ('created_at', strftime('%s', 'now'));
```

### Auto-Pruning

Runs on every watch loop cycle (every 5 seconds):
- `DELETE FROM wallet_memory WHERE seen_at < strftime('%s','now') - 43200` (12h wallet memory)
- `DELETE FROM alerts WHERE created_at < strftime('%s','now') - (86400 * ?)` (configurable retention, default 30 days)

### Migration from JSONL

On first run with new version:
1. Check if `alert_history.jsonl` exists
2. Parse each line, insert into `alerts` table
3. Rename JSONL to `alert_history.jsonl.bak`
4. Log: "Migrated X alerts to SQLite database"

### Rust Dependency

Add `rusqlite` crate with `bundled` feature (bundles SQLite, no system dependency):

```toml
rusqlite = { version = "0.31", features = ["bundled"] }
```

### Wallet Hash

`SHA256(wallet_id)` — consistent length, fast index lookups, collision-resistant.

```rust
use sha2::{Sha256, Digest};

fn wallet_hash(wallet_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(wallet_id.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

Additional dependency:
```toml
sha2 = "0.10"
```

---

## 6. 12-Hour Wallet Short-Term Memory

### Overview

Track every whale wallet for 12 hours. When a known wallet reappears, generate special alerts based on their recent behavior pattern.

### How It Works

1. **On every whale alert**, before displaying, query `wallet_memory` by `wallet_hash`:
   ```sql
   SELECT * FROM wallet_memory
   WHERE wallet_hash = ? AND seen_at > strftime('%s','now') - 43200
   ORDER BY seen_at DESC
   ```

2. **If the wallet has history**, classify the scenario and generate a special alert:

   | Scenario | Condition | Alert |
   |----------|-----------|-------|
   | **Doubling down** | Same market, same outcome | `[RETURNING WHALE] Doubling down on YES` |
   | **Whale flip** | Same market, opposite outcome | `[WHALE FLIP] Was YES, now buying NO` |
   | **Category conviction** | Different market, same category | `[WHALE PATTERN] Active in Crypto markets` |
   | **Known whale** | Any previous activity | `[KNOWN WHALE] 4 txns in 12h totaling $180k` |

3. **After the alert**, insert the new transaction into `wallet_memory`.

4. **Cleanup**: Every poll cycle, `DELETE FROM wallet_memory WHERE seen_at < strftime('%s','now') - 43200`.

### In-Memory Hot Cache

To avoid hitting SQLite on every single trade (most won't be whales, and most whales won't be known):

```rust
pub struct WalletTracker {
    db: rusqlite::Connection,
    known_hashes: HashSet<String>,     // hot cache of active wallet hashes
    last_cache_refresh: Instant,
}
```

- `known_hashes` refreshes from DB every 5 minutes
- `is_known(hash)` checks the HashSet (O(1), no DB call)
- `get_history(hash)` queries DB only when hash is in the set
- `record(...)` inserts into DB + adds to HashSet

### Terminal Display for Returning Whales

```
[RETURNING WHALE] Doubling down — Polymarket
══════════════════════════════════════════════════════════
Question:   Bitcoin above 100k by March?
Position:   BUYING 'Yes' shares

PREVIOUS ACTIVITY (last 12h):
  3h ago:  BUY YES $50,000 @ 62% — Bitcoin above 100k by March?
  7h ago:  BUY YES $30,000 @ 58% — Bitcoin above 100k by March?
  Total:   $80,000 across 2 transactions

NOW ADDING: $45,000 YES @ 65%
CUMULATIVE: $125,000 in this market (all YES)
══════════════════════════════════════════════════════════
```

For whale flips:
```
[WHALE FLIP] Changed position — Polymarket
══════════════════════════════════════════════════════════
Question:   Bitcoin above 100k by March?
Position:   NOW BUYING 'No' shares (was YES)

PREVIOUS POSITION:
  6h ago:  BUY YES $50,000 @ 62%

NOW BUYING NO: $25,000 @ 35%
SIGNAL: Whale reversed conviction
══════════════════════════════════════════════════════════
```

---

## 7. Improved Setup Wizard

### Full Flow

```
═══════════════════════════════════════════════════════════
                  WHALE WATCHER SETUP
═══════════════════════════════════════════════════════════

Step 1 of 6: WHAT TO WATCH
──────────────────────────
What markets do you want to monitor?

  [1] All Categories (everything)     <- default
  [2] Pick specific categories

> 2

Select categories (comma-separated, e.g. 1,3,5):

  [1] Sports       (NBA, NFL, MLB, NHL, Soccer, Golf...)
  [2] Politics     (Elections, Congress, Policy...)
  [3] Crypto       (Bitcoin, Ethereum, Altcoins...)
  [4] Economics    (Fed rates, Inflation, Jobs, GDP...)
  [5] Finance      (S&P 500, NASDAQ, Stocks...)
  [6] Weather      (Temperature, Storms...)
  [7] Tech         (AI, Product launches...)
  [8] Culture      (Entertainment, Awards...)
  [9] World Events (Geopolitics, Conflicts...)
  [10] Health      (FDA, Pandemics...)

> 1,3

You selected: Sports, Crypto

Narrow down? (or Enter for all subcategories)

Sports subcategories:
  [1] All Sports     <- default
  [2] NBA
  [3] NFL
  ...
> 1

Crypto subcategories:
  [1] All Crypto     <- default
  [2] Bitcoin
  [3] Ethereum
  ...
> 2,3

Watching: Sports (all), Crypto (Bitcoin, Ethereum)


Step 2 of 6: ALERT THRESHOLD
─────────────────────────────
Minimum trade size to alert on (in USD).

  [1] $10,000   (high volume)
  [2] $25,000   (recommended)
  [3] $50,000   (mega-whales only)
  [4] Custom amount

> 2

Threshold: $25,000


Step 3 of 6: PLATFORMS
──────────────────────
Which platforms to monitor?

  [1] Both Polymarket + Kalshi (recommended)
  [2] Polymarket only
  [3] Kalshi only

> 1


Step 4 of 6: NOTIFICATIONS
───────────────────────────
Webhook URL for external alerts (n8n, Discord, Zapier)
(Enter to skip)

>


Step 5 of 6: AI AGENT MODE
───────────────────────────
Use with AI agent (OpenClaw, Claude Code)?
Adds deep research capabilities for market analysis.

Enable AI Agent mode? (y/N): y

RapidAPI Key (optional, enhances market data):
>

Perplexity API Key (optional, enables web research):
> sk-xxx

AI Mode: Enabled (Perplexity only)


Step 6 of 6: DATA RETENTION
────────────────────────────
How long to keep alert history?

  [1] 7 days
  [2] 30 days (recommended)
  [3] 90 days
  [4] Keep forever

> 2

History retention: 30 days


═══════════════════════════════════════════════════════════
                 CONFIGURATION SAVED
═══════════════════════════════════════════════════════════

  Categories:    Sports (all), Crypto (BTC, ETH)
  Threshold:     $25,000
  Platforms:     Polymarket + Kalshi
  Webhook:       Not configured
  AI Mode:       Enabled (Perplexity only)
  Retention:     30 days
  Database:      ~/.config/wwatcher/wwatcher.db

Run 'wwatcher watch' to start monitoring.
Run 'wwatcher setup' anytime to reconfigure.
```

### Updated Config Schema

```json
{
  "categories": ["sports:all", "crypto:bitcoin", "crypto:ethereum"],
  "threshold": 25000,
  "platforms": ["polymarket", "kalshi"],
  "history_retention_days": 30,
  "kalshi_api_key_id": null,
  "kalshi_private_key": null,
  "webhook_url": null,
  "rapidapi_key": null,
  "perplexity_api_key": null,
  "ai_agent_mode": false
}
```

### API Key Rules

- **RapidAPI**: Always optional. Enhances AI research with live market data (crypto prices, sports odds, weather). AI agent works without it but with reduced data.
- **Perplexity**: Always optional. Enables deep web research for market analysis. AI agent works without it but skips research queries.
- **AI Agent Mode**: Can be enabled with neither, either, or both keys. Each key independently unlocks its feature.
- **Kalshi API**: Always optional. Public data works without auth. Auth enables orderbook access.
```

---

## 8. API Reference File

Create `docs/API_REFERENCE.md` — single file documenting:

1. All webhook payload formats (entry, exit, probable exit, returning whale)
2. All alert types and their fields
3. Polymarket API endpoints used (with example responses)
4. Kalshi API endpoints used (with example responses)
5. RapidAPI provider formats
6. Config file schema
7. SQLite database schema
8. Category/subcategory definitions

---

## 9. Implementation Order

### Phase 1: Storage Foundation
1. Add `rusqlite` and `sha2` dependencies to `Cargo.toml`
2. Create `src/db.rs` — SQLite connection, schema creation, migration from JSONL
3. Replace `history.rs` to write/read from SQLite instead of JSONL
4. Wire up DB initialization in `main.rs`

### Phase 2: Wallet Memory
5. Implement `WalletTracker` backed by SQLite `wallet_memory` table + in-memory hash cache
6. Add returning whale detection logic (doubling down, flip, category conviction, known whale)
7. Add returning whale terminal display and webhook payload fields
8. Add 12h auto-cleanup in watch loop

### Phase 3: Exit Detection Fixes
9. Fix Kalshi `taker_side` handling — remove dead sell check, report yes/no accurately
10. Add inferred exit detection for Polymarket using `wallet_memory` data

### Phase 4: Edge Data
11. Add `MarketContext` struct
12. Add `fetch_market_context()` for Polymarket (Gamma API)
13. Add `fetch_market_context()` for Kalshi (`/markets/{ticker}`)
14. Wire edge data into alert pipeline — fetch on whale detect, display, webhook, persist

### Phase 5: Categories
15. Create `src/categories.rs` — category model, keyword map, platform mappings
16. Add `categories` field to config
17. Implement hybrid filtering in watch loop (server-side Kalshi, client-side Polymarket)

### Phase 6: Setup Wizard
18. Overhaul `setup.rs` with 6-step guided flow
19. Add category selection (step 1), threshold presets (step 2), platform selection (step 3), retention (step 6)

### Phase 7: Documentation
20. Create `docs/API_REFERENCE.md`
21. Update README with new features

---

## 10. Files Modified

### Rust Core (Modified)
- `Cargo.toml` — Add `rusqlite`, `sha2` dependencies
- `src/main.rs` — Initialize DB, pass to watch command
- `src/types.rs` — Rewrite `WalletTracker` to use SQLite + hash cache
- `src/config.rs` — Add `categories`, `threshold`, `platforms`, `history_retention_days` fields
- `src/alerts/mod.rs` — Add `MarketContext` to `AlertData`, add returning whale fields, update `build_alert_payload`
- `src/alerts/display.rs` — Add market context display, returning whale display, fix exit headers
- `src/alerts/history.rs` — Rewrite to use SQLite instead of JSONL
- `src/alerts/webhook.rs` — Updated payload format with new fields
- `src/platforms/polymarket.rs` — Add `fetch_market_context()` function
- `src/platforms/kalshi.rs` — Add `fetch_market_context()`, fix `taker_side` handling
- `src/commands/watch.rs` — Category filtering, edge data fetching, wallet memory checks, inferred exit detection
- `src/commands/setup.rs` — Complete overhaul with 6-step guided flow

### New Files
- `src/db.rs` — SQLite connection management, schema, migrations, queries
- `src/categories.rs` — Category model, keyword map, platform mappings
- `docs/API_REFERENCE.md` — Unified API/webhook reference
- `docs/plans/2026-02-12-categories-edge-data-design.md` — This document

### Deleted/Archived
- `~/.config/wwatcher/alert_history.jsonl` — Migrated to SQLite, renamed to `.bak`

---

## 11. New Dependencies

```toml
# Cargo.toml additions
rusqlite = { version = "0.31", features = ["bundled"] }  # SQLite with bundled lib
sha2 = "0.10"                                            # SHA256 for wallet hashing
```

Both are well-maintained, widely-used Rust crates. `bundled` feature means no system SQLite dependency.

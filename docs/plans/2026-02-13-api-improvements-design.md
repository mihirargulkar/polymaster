# API Improvements Plan

**Date**: 2026-02-13
**Status**: Complete (all 4 phases implemented)
**Scope**: Maximize API utilization across Polymarket and Kalshi — no ML, pure API intelligence

---

## Current State: 5 endpoints

| Platform | Endpoint | Purpose |
|---|---|---|
| Polymarket | `GET /trades` (Data API) | Fetch recent trades |
| Polymarket | `GET /markets` (Gamma API) | Market metadata + edge data |
| Kalshi | `GET /markets/trades` | Fetch recent trades |
| Kalshi | `GET /markets/{ticker}` | Market details + edge data |
| Kalshi | `GET /markets` | Market title lookup |

---

## Phase 1: Quick Wins

### 1a. Polymarket server-side whale filtering
- Add `filterType=CASH&filterAmount={threshold}` to `/trades` query
- Eliminates client-side filtering of small trades
- One query param change in `polymarket.rs`

### 1b. Polymarket trades endpoint improvements
- Add `takerOnly=true` (already default, make explicit)
- Use `limit=10000` with server-side filter instead of `limit=100` without

---

## Phase 2: Whale Intelligence (Polymarket only — Kalshi trades are anonymous)

### 2a. WhaleProfile struct
```rust
pub struct WhaleProfile {
    pub wallet_id: String,
    pub portfolio_value: Option<f64>,
    pub positions_count: Option<u32>,
    pub leaderboard_rank: Option<u32>,
    pub leaderboard_profit: Option<f64>,
    pub win_rate: Option<f64>,
    pub markets_traded: Option<u32>,
    pub fetched_at: std::time::Instant,
}
```

### 2b. New API calls per whale alert
- `GET data-api.polymarket.com/value?user={wallet}` — portfolio value
- `GET data-api.polymarket.com/positions?user={wallet}&limit=5` — top current positions
- `GET data-api.polymarket.com/closed-positions?user={wallet}&limit=20` — recent closed for win rate
- `GET data-api.polymarket.com/leaderboard?limit=500` — cached, check if wallet is ranked

### 2c. Caching strategy
- Cache leaderboard for 1 hour (it's the same for all alerts)
- Cache individual whale profiles for 30 minutes
- Store in a HashMap<String, WhaleProfile> with TTL check

### 2d. Display output
```
[WHALE PROFILE] — Polymarket
Portfolio:    $2,340,000
Rank:         #45 on leaderboard
Win Rate:     73% (142/195 markets)
Top Holdings: 5 active positions
```

### 2e. Webhook payload addition
```json
{
  "whale_profile": {
    "portfolio_value": 2340000,
    "leaderboard_rank": 45,
    "win_rate": 0.73,
    "markets_traded": 195,
    "positions_count": 5
  }
}
```

---

## Phase 3: Market Intelligence

### 3a. Order book depth
- Kalshi: `GET /markets/{ticker}/orderbook` — public, no auth
- Polymarket: `GET clob.polymarket.com/book?token_id={asset_id}` — public
- Add to MarketContext struct:
```rust
pub struct OrderBookSummary {
    pub best_bid: f64,
    pub best_ask: f64,
    pub bid_depth_10pct: f64,  // total $ within 10% of best bid
    pub ask_depth_10pct: f64,  // total $ within 10% of best ask
    pub bid_levels: u32,
    pub ask_levels: u32,
}
```
- Display: "Book: $45k bid / $38k ask (12 levels deep)"

### 3b. Top holders per market (Polymarket)
- `GET data-api.polymarket.com/top-holders?market={condition_id}`
- Show "Top 5 holders control X% of open interest"
- Gives context: is the whale joining or fighting the crowd?

### 3c. Native category APIs (replace keyword matching for Kalshi)
- Kalshi: `GET /events?status=open` returns `category` field natively
- Kalshi: `GET /search/tags_by_categories` for official taxonomy
- Use Kalshi's native categories instead of keyword matching when available
- Keep keyword matching as fallback for Polymarket (no native category on trade data)

### 3d. Polymarket tags/events
- `GET gamma-api.polymarket.com/tags` — full tag list
- `GET gamma-api.polymarket.com/events?tag={slug}` — events by tag
- Cache tag mappings, use for category enrichment on whale alerts

---

## Phase 4: Real-Time WebSockets

### 4a. Architecture change
- Replace polling loop with WebSocket connections
- Keep polling as fallback if WebSocket disconnects
- Dual connection: one per platform

### 4b. Kalshi WebSocket
- Connect: `wss://api.elections.kalshi.com/trade-api/ws/v2`
- Subscribe to `trade` channel (all trades, real-time)
- Subscribe to `ticker` channel for price updates on active markets
- Auth headers optional for public channels
- Ping every 10 seconds to keep alive

### 4c. Polymarket WebSocket
- Connect to CLOB WebSocket (market channel)
- Subscribe to trade events per active market
- Alternative: continue polling Data API but with server-side filter (simpler)

### 4d. Hybrid approach (recommended)
- Kalshi: WebSocket `trade` channel (well-documented, straightforward)
- Polymarket: Keep HTTP polling with server-side CASH filter (their WS requires per-market subscription which doesn't fit "watch everything" mode)

---

## Implementation Order

### Phase 1 (Quick Wins)
1. Update `polymarket::fetch_recent_trades()` — add filterType/filterAmount params
2. Pass threshold from watch loop to fetch function
3. Build + test

### Phase 2 (Whale Intelligence)
4. Create `src/whale_profile.rs` with WhaleProfile struct and fetch functions
5. Add `fetch_whale_profile()` — calls value, positions, closed-positions endpoints
6. Add leaderboard cache with 1h TTL
7. Compute win rate from closed positions
8. Add `print_whale_profile()` to display.rs
9. Add whale_profile to webhook payload
10. Wire into watch loop (fetch after whale detected, before display)
11. Build + test

### Phase 3 (Market Intelligence)
12. Add `fetch_order_book()` for both platforms
13. Add OrderBookSummary to MarketContext or as separate display
14. Add `fetch_top_holders()` for Polymarket
15. Add Kalshi native category lookup via `/events` endpoint
16. Cache Kalshi event→category mappings
17. Add Polymarket tag caching via Gamma API
18. Build + test

### Phase 4 (Real-Time)
19. Add `tokio-tungstenite` dependency for WebSocket
20. Create `src/ws/kalshi.rs` — WebSocket client for trade channel
21. Add reconnection logic with exponential backoff
22. Modify watch loop: try WebSocket first, fall back to polling
23. Build + test

---

## Files Modified

### New Files
- `src/whale_profile.rs` — WhaleProfile struct, fetch, cache, display
- `src/ws/mod.rs` — WebSocket module (Phase 4)
- `src/ws/kalshi.rs` — Kalshi WebSocket client (Phase 4)

### Modified Files
- `src/platforms/polymarket.rs` — server-side filter, order book, top holders
- `src/platforms/kalshi.rs` — order book fetch, native categories
- `src/alerts/mod.rs` — WhaleProfile in AlertData, OrderBookSummary
- `src/alerts/display.rs` — whale profile display, order book display
- `src/alerts/webhook.rs` — whale profile in payload (via build_alert_payload)
- `src/commands/watch.rs` — wire whale profile, order book, WebSocket
- `src/main.rs` — add mod whale_profile, mod ws
- `src/categories.rs` — integrate native Kalshi categories
- `Cargo.toml` — add tokio-tungstenite (Phase 4)

---

## New Dependencies
- Phase 4 only: `tokio-tungstenite = "0.21"` for WebSocket support

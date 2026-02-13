# Webhook Reference

Complete guide to the webhook JSON payload sent by wwatcher on every whale alert. Use this to build integrations with n8n, Zapier, Make, Discord, Telegram, or any webhook endpoint.

---

## Full Payload Schema

Every webhook POST sends a JSON body. Fields marked **(optional)** may be absent depending on platform and data availability.

```json
{
  "platform": "Polymarket",
  "alert_type": "WHALE_ENTRY",
  "action": "BUY",
  "value": 50000.0,
  "price": 0.65,
  "price_percent": 65,
  "size": 76923.08,
  "timestamp": "2026-02-13T18:00:00Z",
  "market_title": "Will Bitcoin reach 100k by end of 2026?",
  "outcome": "Yes",
  "wallet_id": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",

  "wallet_activity": {
    "transactions_last_hour": 2,
    "transactions_last_day": 5,
    "total_value_hour": 125000.0,
    "total_value_day": 380000.0,
    "is_repeat_actor": true,
    "is_heavy_actor": true
  },

  "market_context": {
    "yes_price": 0.65,
    "no_price": 0.35,
    "spread": 0.02,
    "volume_24h": 450000.0,
    "open_interest": 2100000.0,
    "price_change_24h": 3.2,
    "liquidity": 180000.0,
    "tags": ["crypto", "bitcoin"]
  },

  "whale_profile": {
    "portfolio_value": 2340000.0,
    "leaderboard_rank": 45,
    "leaderboard_profit": 890000.0,
    "win_rate": 0.73,
    "markets_traded": 195,
    "positions_count": 12
  },

  "order_book": {
    "best_bid": 0.64,
    "best_ask": 0.66,
    "bid_depth_10pct": 45000.0,
    "ask_depth_10pct": 38000.0,
    "bid_levels": 12,
    "ask_levels": 9
  },

  "top_holders": {
    "holders": [
      { "wallet": "0x742d...f0bEb", "shares": 150000.0, "value": 97500.0 },
      { "wallet": "0x8a3f...2c1D", "shares": 120000.0, "value": 78000.0 },
      { "wallet": "0x5b9c...e4aF", "shares": 95000.0, "value": 61750.0 },
      { "wallet": "0x1d6e...b3c2", "shares": 80000.0, "value": 52000.0 },
      { "wallet": "0x9f2a...7d8E", "shares": 65000.0, "value": 42250.0 }
    ],
    "total_shares": 1250000.0
  }
}
```

---

## Field Reference

### Core Fields (always present)

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `platform` | string | `"Polymarket"` or `"Kalshi"` | `"Polymarket"` |
| `alert_type` | string | `"WHALE_ENTRY"` or `"WHALE_EXIT"` | `"WHALE_ENTRY"` |
| `action` | string | `"BUY"` or `"SELL"` (Kalshi uses `"YES"`/`"NO"`) | `"BUY"` |
| `value` | number | Trade value in USD | `50000.0` |
| `price` | number | Price per contract (0.0 to 1.0) | `0.65` |
| `price_percent` | integer | Price as percentage (0 to 100) | `65` |
| `size` | number | Number of contracts | `76923.08` |
| `timestamp` | string | ISO 8601 timestamp | `"2026-02-13T18:00:00Z"` |
| `market_title` | string or null | Market question text | `"Will Bitcoin reach 100k?"` |
| `outcome` | string or null | Outcome being traded | `"Yes"` |

### Wallet ID (Polymarket only)

| Field | Type | Description |
|-------|------|-------------|
| `wallet_id` | string | On-chain wallet address. Only present for Polymarket trades. Kalshi trades are anonymous. |

### Wallet Activity (optional)

Present when the wallet has been seen before in the current session.

| Field | Type | Description |
|-------|------|-------------|
| `wallet_activity.transactions_last_hour` | integer | Number of whale trades by this wallet in last hour |
| `wallet_activity.transactions_last_day` | integer | Number of whale trades by this wallet in last 24 hours |
| `wallet_activity.total_value_hour` | number | Total USD value of trades in last hour |
| `wallet_activity.total_value_day` | number | Total USD value of trades in last 24 hours |
| `wallet_activity.is_repeat_actor` | boolean | True if 2+ transactions in 1 hour |
| `wallet_activity.is_heavy_actor` | boolean | True if 5+ transactions in 24 hours |

### Market Context (optional)

Fetched per alert from the platform's market API.

| Field | Type | Description |
|-------|------|-------------|
| `market_context.yes_price` | number | Current YES price (0.0-1.0) |
| `market_context.no_price` | number | Current NO price (0.0-1.0) |
| `market_context.spread` | number | Bid-ask spread |
| `market_context.volume_24h` | number | 24-hour trading volume in USD |
| `market_context.open_interest` | number | Total open interest in USD |
| `market_context.price_change_24h` | number | 24-hour price change as percentage |
| `market_context.liquidity` | number | Available liquidity in USD |
| `market_context.tags` | array of strings | Market tags/categories from the platform |

### Whale Profile (Polymarket only, optional)

Fetched from Polymarket Data API. Cached for 30 minutes per wallet. Only present for Polymarket trades where the wallet is identifiable.

| Field | Type | Description |
|-------|------|-------------|
| `whale_profile.portfolio_value` | number | Total portfolio value in USD |
| `whale_profile.leaderboard_rank` | integer | Rank on Polymarket leaderboard (top 500) |
| `whale_profile.leaderboard_profit` | number | Total profit from leaderboard |
| `whale_profile.win_rate` | number | Win rate from closed positions (0.0-1.0) |
| `whale_profile.markets_traded` | integer | Number of markets traded |
| `whale_profile.positions_count` | integer | Current number of open positions |

Note: Each field within `whale_profile` may be absent if the API call failed or returned no data. The object itself is only present when at least one field has data.

### Order Book (optional)

Fetched per alert. Polymarket uses CLOB API, Kalshi uses their orderbook endpoint.

| Field | Type | Description |
|-------|------|-------------|
| `order_book.best_bid` | number | Highest bid price (0.0-1.0) |
| `order_book.best_ask` | number | Lowest ask price (0.0-1.0) |
| `order_book.bid_depth_10pct` | number | Total USD value of bids within 10% of best bid |
| `order_book.ask_depth_10pct` | number | Total USD value of asks within 10% of best ask |
| `order_book.bid_levels` | integer | Number of distinct bid price levels |
| `order_book.ask_levels` | integer | Number of distinct ask price levels |

### Top Holders (Polymarket only, optional)

Top 5 holders of the market's shares. Only available for Polymarket.

| Field | Type | Description |
|-------|------|-------------|
| `top_holders.holders` | array | Array of top 5 holder objects |
| `top_holders.holders[].wallet` | string | Wallet address |
| `top_holders.holders[].shares` | number | Number of shares held |
| `top_holders.holders[].value` | number | Value of shares in USD |
| `top_holders.total_shares` | number | Total shares across all holders in the market |

---

## Platform Differences

| Feature | Polymarket | Kalshi |
|---------|-----------|--------|
| `wallet_id` | Yes (on-chain address) | Never (anonymous) |
| `wallet_activity` | Yes | No |
| `whale_profile` | Yes (portfolio, rank, win rate) | Never |
| `market_context` | Yes | Yes |
| `order_book` | Yes (CLOB API) | Yes (orderbook API) |
| `top_holders` | Yes | Never |
| `market_context.tags` | Yes (from Gamma API) | Yes (native category) |
| `action` values | `BUY` / `SELL` | `YES` / `NO` |
| Real-time delivery | HTTP polling (5s) | WebSocket (instant) + HTTP fallback |

---

## n8n Integration

### Webhook Node Setup

1. Add a **Webhook** node as the trigger
2. Set **HTTP Method** to `POST`
3. Copy the webhook URL
4. Run `wwatcher setup` and paste the URL when prompted

### n8n Telegram Message Template

Use this in a **Telegram** node's message field. Each `{{ }}` expression is kept simple for n8n compatibility.

```
{{ $json.alert_type === 'WHALE_EXIT' ? '🔴' : '🟢' }} *{{ $json.alert_type }}* — {{ $json.platform }}

*{{ $json.market_title }}*
{{ $json.action }} {{ $json.outcome }} @ {{ $json.price_percent }}%

💰 *${{ $json.value.toLocaleString('en-US', {maximumFractionDigits: 0}) }}*
📊 {{ $json.size.toLocaleString('en-US', {maximumFractionDigits: 0}) }} contracts

{{ $json.market_context ? '📈 Vol 24h: $' + $json.market_context.volume_24h.toLocaleString('en-US', {maximumFractionDigits: 0}) : '' }}
{{ $json.market_context ? '🏦 Open Interest: $' + $json.market_context.open_interest.toLocaleString('en-US', {maximumFractionDigits: 0}) : '' }}
{{ $json.market_context ? '💧 Liquidity: $' + $json.market_context.liquidity.toLocaleString('en-US', {maximumFractionDigits: 0}) : '' }}
{{ $json.market_context && $json.market_context.price_change_24h ? '📉 24h Move: ' + $json.market_context.price_change_24h.toFixed(1) + '%' : '' }}

{{ $json.whale_profile ? '🐋 WHALE PROFILE' : '' }}
{{ $json.whale_profile && $json.whale_profile.leaderboard_rank ? '🏆 Rank: #' + $json.whale_profile.leaderboard_rank : '' }}
{{ $json.whale_profile && $json.whale_profile.portfolio_value ? '💼 Portfolio: $' + $json.whale_profile.portfolio_value.toLocaleString('en-US', {maximumFractionDigits: 0}) : '' }}
{{ $json.whale_profile && $json.whale_profile.leaderboard_profit ? '📈 Profit: $' + $json.whale_profile.leaderboard_profit.toLocaleString('en-US', {maximumFractionDigits: 0}) : '' }}
{{ $json.whale_profile && $json.whale_profile.win_rate ? '🎯 Win Rate: ' + ($json.whale_profile.win_rate * 100).toFixed(1) + '%' : '' }}

{{ $json.order_book ? '📖 ORDER BOOK' : '' }}
{{ $json.order_book ? 'Bid: $' + $json.order_book.bid_depth_10pct.toLocaleString('en-US', {maximumFractionDigits: 0}) + ' (' + $json.order_book.bid_levels + ' lvls) | Ask: $' + $json.order_book.ask_depth_10pct.toLocaleString('en-US', {maximumFractionDigits: 0}) + ' (' + $json.order_book.ask_levels + ' lvls)' : '' }}

{{ $json.wallet_activity && $json.wallet_activity.is_heavy_actor ? '🚨 HEAVY ACTOR: ' + $json.wallet_activity.transactions_last_day + ' txns, $' + $json.wallet_activity.total_value_day.toLocaleString('en-US', {maximumFractionDigits: 0}) + ' in 24h' : '' }}
{{ $json.wallet_activity && $json.wallet_activity.is_repeat_actor && !$json.wallet_activity.is_heavy_actor ? '⚠️ REPEAT ACTOR: ' + $json.wallet_activity.transactions_last_hour + ' txns in 1h' : '' }}

{{ $json.wallet_id ? '👛 ' + $json.wallet_id.substring(0, 8) + '...' + $json.wallet_id.slice(-6) : '' }}
🕐 {{ $json.timestamp }}
```

### n8n Discord Message Template

For a **Discord** node, use this in the message content:

```
{{ $json.alert_type === 'WHALE_EXIT' ? '🔴' : '🟢' }} **{{ $json.alert_type }}** — {{ $json.platform }}

**{{ $json.market_title }}**
{{ $json.action }} {{ $json.outcome }} @ {{ $json.price_percent }}%
💰 **${{ $json.value.toLocaleString('en-US', {maximumFractionDigits: 0}) }}** ({{ $json.size.toLocaleString('en-US', {maximumFractionDigits: 0}) }} contracts)

{{ $json.whale_profile && $json.whale_profile.leaderboard_rank ? '🏆 Leaderboard #' + $json.whale_profile.leaderboard_rank + ' | Win Rate: ' + ($json.whale_profile.win_rate * 100).toFixed(1) + '%' : '' }}
{{ $json.order_book ? '📖 Book: $' + $json.order_book.bid_depth_10pct.toLocaleString('en-US', {maximumFractionDigits: 0}) + ' bid / $' + $json.order_book.ask_depth_10pct.toLocaleString('en-US', {maximumFractionDigits: 0}) + ' ask' : '' }}
{{ $json.market_context ? '📊 Vol: $' + $json.market_context.volume_24h.toLocaleString('en-US', {maximumFractionDigits: 0}) + ' | OI: $' + $json.market_context.open_interest.toLocaleString('en-US', {maximumFractionDigits: 0}) : '' }}
```

### n8n IF Node — Filter by Criteria

Use an **IF** node to route alerts based on conditions:

**Heavy actors only:**
```
{{ $json.wallet_activity && $json.wallet_activity.is_heavy_actor }}
```

**Polymarket only:**
```
{{ $json.platform === 'Polymarket' }}
```

**Value above $100k:**
```
{{ $json.value >= 100000 }}
```

**Top 100 leaderboard whales:**
```
{{ $json.whale_profile && $json.whale_profile.leaderboard_rank && $json.whale_profile.leaderboard_rank <= 100 }}
```

**Exits only:**
```
{{ $json.alert_type === 'WHALE_EXIT' }}
```

### n8n Code Node — Computed Fields

Use a **Code** node to add computed fields for downstream nodes:

```javascript
const item = $input.first().json;

// Bid/ask imbalance
let imbalance = 'balanced';
if (item.order_book) {
  const total = item.order_book.bid_depth_10pct + item.order_book.ask_depth_10pct;
  if (total > 0) {
    const bidPct = item.order_book.bid_depth_10pct / total;
    if (bidPct > 0.65) imbalance = 'strong_bid';
    else if (bidPct > 0.55) imbalance = 'moderate_bid';
    else if (bidPct < 0.35) imbalance = 'strong_ask';
    else if (bidPct < 0.45) imbalance = 'moderate_ask';
  }
}

// Top holder concentration
let top5Pct = 0;
if (item.top_holders && item.top_holders.total_shares > 0) {
  const top5Shares = item.top_holders.holders.reduce((sum, h) => sum + h.shares, 0);
  top5Pct = (top5Shares / item.top_holders.total_shares) * 100;
}

// Whale quality score (higher = more credible whale)
let whaleScore = 0;
if (item.whale_profile) {
  if (item.whale_profile.leaderboard_rank && item.whale_profile.leaderboard_rank <= 50) whaleScore += 3;
  else if (item.whale_profile.leaderboard_rank && item.whale_profile.leaderboard_rank <= 200) whaleScore += 1;
  if (item.whale_profile.win_rate && item.whale_profile.win_rate > 0.6) whaleScore += 2;
  if (item.whale_profile.portfolio_value && item.whale_profile.portfolio_value > 500000) whaleScore += 2;
}
if (item.wallet_activity && item.wallet_activity.is_heavy_actor) whaleScore += 1;

return [{
  json: {
    ...item,
    computed: {
      imbalance,
      top5_holder_pct: top5Pct.toFixed(1),
      whale_quality_score: whaleScore,
      is_high_quality_whale: whaleScore >= 4,
    }
  }
}];
```

---

## Webhook Payload Size

Typical payload sizes:
- **Minimal** (Kalshi, no market context): ~300 bytes
- **Standard** (Polymarket with market context): ~700 bytes
- **Full** (all optional fields present): ~1,500 bytes

---

## Testing Webhooks

```bash
# Send test webhook alerts
wwatcher test-webhook

# This sends two test payloads:
# 1. Polymarket BUY alert ($50,000)
# 2. Kalshi SELL alert ($35,000)
```

You can also use curl to simulate a webhook:

```bash
curl -X POST http://your-n8n-url/webhook/xxx \
  -H "Content-Type: application/json" \
  -d '{
    "platform": "Polymarket",
    "alert_type": "WHALE_ENTRY",
    "action": "BUY",
    "value": 75000,
    "price": 0.72,
    "price_percent": 72,
    "size": 104166.67,
    "timestamp": "2026-02-13T20:00:00Z",
    "market_title": "Will BTC reach 100k?",
    "outcome": "Yes",
    "wallet_id": "0xabc123def456",
    "market_context": {
      "yes_price": 0.72,
      "no_price": 0.28,
      "spread": 0.01,
      "volume_24h": 500000,
      "open_interest": 3000000,
      "price_change_24h": 2.5,
      "liquidity": 200000,
      "tags": ["crypto"]
    },
    "whale_profile": {
      "portfolio_value": 1500000,
      "leaderboard_rank": 23,
      "leaderboard_profit": 450000,
      "win_rate": 0.68,
      "markets_traded": 142,
      "positions_count": 8
    },
    "order_book": {
      "best_bid": 0.71,
      "best_ask": 0.73,
      "bid_depth_10pct": 55000,
      "ask_depth_10pct": 42000,
      "bid_levels": 15,
      "ask_levels": 11
    }
  }'
```

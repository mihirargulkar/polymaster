# API Reference

This document provides details about the Polymarket and Kalshi APIs used by Whale Watcher.

## Polymarket API

### Base URL
```
https://data-api.polymarket.com
```

### Endpoints Used

#### Get Trades
- **Endpoint**: `/trades`
- **Method**: GET
- **Authentication**: None (public)
- **Parameters**:
  - `limit` (optional): Number of trades to fetch (default: 100)

**Example Response**:
```json
[
  {
    "id": "123456789",
    "market": "0xbd31dc8a...",
    "asset": "65396714035221124737...",
    "side": "BUY",
    "size": 1000.50,
    "price": 0.75,
    "timestamp": 1704753600,
    "type": "TRADE"
  }
]
```

### Documentation
- Official Docs: https://docs.polymarket.com
- Data API: https://polymarket.notion.site/Polymarket-Data-API-Docs-15fd316c50d58062bf8ee1b4bcf3d461

### Rate Limits
- Public endpoints: ~100 requests/minute (not officially documented)
- No authentication required for public data

---

## Kalshi API

### Base URL
```
https://api.elections.kalshi.com/trade-api/v2
```

### Endpoints Used

#### Get Trades
- **Endpoint**: `/markets/trades`
- **Method**: GET
- **Authentication**: Optional (public endpoint available)
- **Parameters**:
  - `limit` (optional): Max number of trades (1-1000, default: 100)
  - `cursor` (optional): Pagination cursor
  - `ticker` (optional): Filter by market ticker
  - `min_ts` (optional): Minimum timestamp
  - `max_ts` (optional): Maximum timestamp

**Example Response**:
```json
{
  "trades": [
    {
      "trade_id": "abc123",
      "ticker": "KXHIGHNY-24DEC-T63",
      "price": 75,
      "count": 100,
      "yes_price": 75,
      "no_price": 25,
      "taker_side": "yes",
      "created_time": "2026-01-08T21:00:00Z"
    }
  ],
  "cursor": "next_page_cursor"
}
```

### Authentication (Optional)

For authenticated requests, Kalshi uses HMAC-based authentication:

**Headers Required**:
- `KALSHI-ACCESS-KEY`: Your API key ID
- `KALSHI-ACCESS-SIGNATURE`: HMAC signature
- `KALSHI-ACCESS-TIMESTAMP`: Request timestamp

**Note**: The current implementation uses basic header auth for simplicity. For production use, implement proper HMAC signing.

### Documentation
- Official Docs: https://docs.kalshi.com
- API Reference: https://docs.kalshi.com/api-reference

### Rate Limits
- Public endpoints: Available without authentication
- Authenticated: Higher limits based on tier
- Default: 100 requests/minute for public endpoints

---

## Data Models

### Trade Value Calculation

**Polymarket**:
```
trade_value = size × price
```
Where:
- `size`: Number of contracts
- `price`: Price per contract (0.00 to 1.00, representing probability)

**Kalshi**:
```
trade_value = (yes_price / 100) × count
```
Where:
- `yes_price`: Price in cents (0-100)
- `count`: Number of contracts
- Divide by 100 to convert cents to dollars

### Example Calculations

**Polymarket Example**:
- Size: 50,000 contracts
- Price: $0.60
- Value: 50,000 × 0.60 = **$30,000**

**Kalshi Example**:
- Count: 1,000 contracts
- Yes Price: 75 cents
- Value: (75 / 100) × 1,000 = **$750**

---

## Public vs Authenticated Access

### Polymarket
| Feature | Public | Authenticated |
|---------|--------|---------------|
| Market data | ✅ | ✅ |
| Trade history | ✅ | ✅ |
| Personal orders | ❌ | ✅ |
| Place trades | ❌ | ✅ |

**Current Implementation**: Uses public endpoints only

### Kalshi
| Feature | Public | Authenticated |
|---------|--------|---------------|
| Market data | ✅ | ✅ |
| Trade history | ✅ | ✅ |
| Personal orders | ❌ | ✅ |
| Place trades | ❌ | ✅ |
| Higher rate limits | ❌ | ✅ |

**Current Implementation**: Public by default, basic auth headers if credentials provided

---

## Error Handling

Both APIs may return these common errors:

### HTTP Status Codes
- `200 OK`: Success
- `400 Bad Request`: Invalid parameters
- `401 Unauthorized`: Authentication failed (authenticated endpoints)
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: API error

### Whale Watcher Handling
- Logs errors to stderr
- Continues monitoring other platform if one fails
- Returns empty trade list on parse errors (graceful degradation)

---

## Getting API Credentials

### Polymarket
Not required - public data access only!

### Kalshi
1. Sign up at https://kalshi.com
2. Navigate to https://kalshi.com/profile/api-keys
3. Click "Generate New API Key"
4. Save your API Key ID and Private Key
5. Run `whale-watcher setup` to configure

⚠️ **Security Note**: Never commit API keys to version control!

---

## Testing APIs Directly

### Polymarket
```bash
# Fetch recent trades
curl "https://data-api.polymarket.com/trades?limit=10"
```

### Kalshi
```bash
# Fetch recent trades (public)
curl "https://api.elections.kalshi.com/trade-api/v2/markets/trades?limit=10"
```

---

## Future Enhancements

Potential additions for the tool:
- WebSocket support for real-time data (lower latency)
- Full HMAC authentication for Kalshi
- Historical trade analysis
- Market filtering by category
- Alert notifications (email, Discord, Telegram)
- Database storage of whale transactions

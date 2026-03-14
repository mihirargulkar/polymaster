# API Reference Documentation

## Polymarket (Gamma API)
- **Base URL**: `https://gamma-api.polymarket.com`
- **Purpose**: Market discovery and querying active events.
- **Key Params**: `?active=true&closed=false&limit=100`
- **Output Notes**: A single Event can contain multiple mutually exclusive "Markets" inside its `markets` array. Always refer to the primary market for prices.
- **CLOB API**: `https://clob.polymarket.com`. Requires L1 signature authentication (EIP-712). Used for order placement and live depth queries.

## Kalshi (REST API Level 2)
- **Base URL**: `https://trading-api.kalshi.com/trade-api/v2` (Or `https://demo-api.kalshi.co` for testing).
- **Purpose**: Unified endpoint for discovery and trade execution.
- **Auth Scheme**: RSA Asymmetric Key signature.
    - Requires passing headers:
      - `KALSHI-ACCESS-KEY`
      - `KALSHI-ACCESS-SIGNATURE` (Base64 encoded SHA256 of `timestamp + method + path`)
      - `KALSHI-ACCESS-TIMESTAMP`
- **Prices**: Kalshi prices are strictly in cents (e.g. 49 = $0.49). Convert accordingly when standardizing across platforms.

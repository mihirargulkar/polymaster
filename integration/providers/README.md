# Providers Directory

This directory contains RapidAPI provider configurations organized by category. Each JSON file defines one or more providers for a specific market category.

## Structure

```
providers/
├── README.md       # This file
├── crypto.json     # Cryptocurrency price APIs
├── sports.json     # Sports odds and game data
├── weather.json    # Weather forecasts
└── news.json       # News aggregation
```

## Adding a New Provider

### Option 1: Add to Existing Category

Edit the relevant category file (e.g., `crypto.json`) and add a new provider entry:

```json
{
  "existing_provider": { ... },
  "new_provider": {
    "name": "Display Name",
    "category": "crypto",
    "rapidapi_host": "api-name.p.rapidapi.com",
    "env_key": "RAPIDAPI_KEY",
    "keywords": ["bitcoin", "btc", "ethereum"],
    "endpoints": {
      "endpoint_name": {
        "method": "GET",
        "path": "/api/endpoint",
        "description": "What this endpoint returns",
        "params": {
          "param_name": {
            "type": "string",
            "required": true,
            "default": "default_value"
          }
        }
      }
    }
  }
}
```

### Option 2: Create a New Category

Create a new JSON file (e.g., `politics.json`):

```json
{
  "polymarket_data": {
    "name": "Polymarket API",
    "category": "politics",
    "rapidapi_host": "polymarket.p.rapidapi.com",
    "env_key": "RAPIDAPI_KEY",
    "keywords": ["election", "president", "senate", "congress", "vote"],
    "endpoints": {
      "markets": {
        "method": "GET",
        "path": "/markets",
        "description": "List political prediction markets",
        "params": {}
      }
    }
  }
}
```

The system automatically loads all `*.json` files from this directory.

## Provider Schema

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | ✓ | Display name for the provider |
| `category` | string | ✓ | Category for matching (crypto, sports, weather, news, etc.) |
| `rapidapi_host` | string | ✓ | RapidAPI host header value |
| `env_key` | string | ✓ | Environment variable for API key (usually `RAPIDAPI_KEY`) |
| `keywords` | string[] | ✓ | Keywords that trigger this provider when found in market titles |
| `match_all` | boolean | | If true, provider matches all queries (useful for news) |
| `endpoints` | object | ✓ | Map of endpoint definitions |

### Endpoint Schema

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `method` | string | ✓ | HTTP method (GET, POST) |
| `path` | string | ✓ | API path (can include `{param}` placeholders) |
| `description` | string | ✓ | What the endpoint returns |
| `params` | object | | Parameter definitions with type, required, and default |

## Keyword Matching

When analyzing a market title like "Will Bitcoin reach $100k by March?":

1. The system checks each provider's `keywords` array
2. Matches are case-insensitive
3. Providers with more keyword matches rank higher
4. Use `--category=X` flag to force a specific category

**Tips for keywords:**
- Include common variations (BTC, Bitcoin, btc)
- Add team names for sports (Lakers, Celtics, Warriors)
- Include event names (Super Bowl, World Series)
- Be specific enough to avoid false matches

## Testing Your Provider

After adding a provider, rebuild and test:

```bash
cd /path/to/integration
npm run build
node dist/cli.js status  # Verify provider loaded
node dist/cli.js fetch "your test query" --category=your_category
```

## Finding RapidAPI Endpoints

1. Browse [RapidAPI Hub](https://rapidapi.com/hub)
2. Subscribe to an API (many have free tiers)
3. Copy the host from the code snippets
4. Document the endpoints you need

Your single `RAPIDAPI_KEY` in `.env` works for all subscribed APIs.
